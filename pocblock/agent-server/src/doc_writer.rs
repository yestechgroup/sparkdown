use anyhow::Result;
use yrs::{Array, Doc, Map, Out, Transact, WriteTxn};

/// Insert an agent note block into the Yrs document.
pub fn insert_agent_note(
    doc: &Doc,
    after_block_id: Option<&str>,
    agent_id: &str,
    agent_name: &str,
    note_type: &str,
    content: &str,
    confidence: f64,
) -> Result<String> {
    let block_id = generate_block_id();

    let mut txn = doc.transact_mut();
    let blocks = txn.get_or_insert_map("blocks");

    // Create a nested Y.Map for this block
    let block_map = blocks.insert(&mut txn, block_id.as_str(), yrs::MapPrelim::default());

    // Set system properties
    block_map.insert(&mut txn, "sys:flavour", "sparkdown:agent-note");
    block_map.insert(&mut txn, "sys:id", block_id.as_str());
    block_map.insert(&mut txn, "sys:children", yrs::ArrayPrelim::default());

    // Set block properties
    block_map.insert(&mut txn, "prop:agentId", agent_id);
    block_map.insert(&mut txn, "prop:agentName", agent_name);
    block_map.insert(&mut txn, "prop:noteType", note_type);
    block_map.insert(&mut txn, "prop:content", content);
    block_map.insert(&mut txn, "prop:confidence", confidence);
    block_map.insert(&mut txn, "prop:accepted", false);

    // Try to add to parent's children array
    if let Some(after_id) = after_block_id {
        insert_after_block(&mut txn, &blocks, &block_id, after_id);
    } else {
        insert_into_first_note(&mut txn, &blocks, &block_id);
    }

    Ok(block_id)
}

fn get_flavour(txn: &yrs::TransactionMut, block_map: &yrs::MapRef) -> String {
    block_map
        .get(txn, "sys:flavour")
        .and_then(|v| match v {
            Out::Any(yrs::Any::String(s)) => Some(s.to_string()),
            _ => None,
        })
        .unwrap_or_default()
}

/// Find the parent of after_block_id and insert block_id right after it
fn insert_after_block(
    txn: &mut yrs::TransactionMut,
    blocks: &yrs::MapRef,
    block_id: &str,
    after_id: &str,
) {
    let keys: Vec<String> = blocks.keys(txn).map(|k| k.to_string()).collect();
    for key in &keys {
        if let Some(Out::YMap(block_map)) = blocks.get(txn, key) {
            if let Some(Out::YArray(children)) = block_map.get(txn, "sys:children") {
                let child_ids: Vec<String> = children
                    .iter(txn)
                    .filter_map(|v| match v {
                        Out::Any(yrs::Any::String(s)) => Some(s.to_string()),
                        _ => None,
                    })
                    .collect();

                if let Some(pos) = child_ids.iter().position(|id| id == after_id) {
                    children.insert(txn, (pos + 1) as u32, block_id);
                    return;
                }
            }
        }
    }

    // Fallback
    insert_into_first_note(txn, blocks, block_id);
}

/// Insert block_id into the first affine:note block's children
fn insert_into_first_note(
    txn: &mut yrs::TransactionMut,
    blocks: &yrs::MapRef,
    block_id: &str,
) {
    let keys: Vec<String> = blocks.keys(txn).map(|k| k.to_string()).collect();
    for key in &keys {
        if let Some(Out::YMap(block_map)) = blocks.get(txn, key) {
            if get_flavour(txn, &block_map) == "affine:note" {
                if let Some(Out::YArray(children)) = block_map.get(txn, "sys:children") {
                    let len = children.len(txn);
                    children.insert(txn, len, block_id);
                    return;
                }
            }
        }
    }
}

/// Remove all agent note blocks from the document.
pub fn clear_agent_notes(doc: &Doc) -> Result<usize> {
    let mut txn = doc.transact_mut();
    let blocks = txn.get_or_insert_map("blocks");
    let mut removed = 0;

    // Collect IDs of agent notes to remove
    let agent_note_ids: Vec<String> = {
        let keys: Vec<String> = blocks.keys(&txn).map(|k| k.to_string()).collect();
        keys.into_iter()
            .filter(|key| {
                blocks
                    .get(&txn, key)
                    .and_then(|v| match v {
                        Out::YMap(m) => Some(get_flavour(&txn, &m)),
                        _ => None,
                    })
                    .map(|f| f == "sparkdown:agent-note")
                    .unwrap_or(false)
            })
            .collect()
    };

    // Remove agent note IDs from parent children arrays
    let all_keys: Vec<String> = blocks.keys(&txn).map(|k| k.to_string()).collect();
    for key in &all_keys {
        if let Some(Out::YMap(block_map)) = blocks.get(&txn, key) {
            if let Some(Out::YArray(children)) = block_map.get(&txn, "sys:children") {
                let mut i = 0u32;
                while i < children.len(&txn) {
                    if let Out::Any(yrs::Any::String(s)) = children.get(&txn, i).unwrap_or_default()
                    {
                        if agent_note_ids.contains(&s.to_string()) {
                            children.remove(&mut txn, i);
                            continue;
                        }
                    }
                    i += 1;
                }
            }
        }
    }

    // Remove the blocks themselves
    for id in &agent_note_ids {
        blocks.remove(&mut txn, id);
        removed += 1;
    }

    Ok(removed)
}

fn generate_block_id() -> String {
    uuid::Uuid::new_v4().to_string().replace('-', "")[..10].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use yrs::{Doc, ReadTxn, Transact};

    #[test]
    fn insert_and_read_agent_note() {
        let doc = Doc::new();

        // Set up a minimal BlockSuite-like structure
        {
            let mut txn = doc.transact_mut();
            let blocks = txn.get_or_insert_map("blocks");

            let root = blocks.insert(&mut txn, "root", yrs::MapPrelim::default());
            root.insert(&mut txn, "sys:flavour", "affine:page");
            root.insert(
                &mut txn,
                "sys:children",
                yrs::ArrayPrelim::from(vec![yrs::In::Any(yrs::Any::String("note1".into()))]),
            );

            let note = blocks.insert(&mut txn, "note1", yrs::MapPrelim::default());
            note.insert(&mut txn, "sys:flavour", "affine:note");
            note.insert(
                &mut txn,
                "sys:children",
                yrs::ArrayPrelim::from(vec![yrs::In::Any(yrs::Any::String("para1".into()))]),
            );

            let para = blocks.insert(&mut txn, "para1", yrs::MapPrelim::default());
            para.insert(&mut txn, "sys:flavour", "affine:paragraph");
            para.insert(&mut txn, "sys:children", yrs::ArrayPrelim::default());
        }

        let id = insert_agent_note(
            &doc,
            Some("para1"),
            "test-agent",
            "Test Agent",
            "entity",
            "schema:Person \u{2014} Test",
            0.9,
        )
        .unwrap();

        let txn = doc.transact();
        let blocks = txn.get_map("blocks").unwrap();
        assert!(blocks.get(&txn, &id).is_some());
    }

    #[test]
    fn clear_removes_only_agent_notes() {
        let doc = Doc::new();

        {
            let mut txn = doc.transact_mut();
            let blocks = txn.get_or_insert_map("blocks");

            let note = blocks.insert(&mut txn, "note1", yrs::MapPrelim::default());
            note.insert(&mut txn, "sys:flavour", "affine:note");
            note.insert(
                &mut txn,
                "sys:children",
                yrs::ArrayPrelim::from(vec![yrs::In::Any(yrs::Any::String("para1".into()))]),
            );

            let para = blocks.insert(&mut txn, "para1", yrs::MapPrelim::default());
            para.insert(&mut txn, "sys:flavour", "affine:paragraph");
            para.insert(&mut txn, "sys:children", yrs::ArrayPrelim::default());
        }

        let _id = insert_agent_note(
            &doc,
            Some("para1"),
            "test-agent",
            "Test",
            "entity",
            "test",
            0.9,
        )
        .unwrap();

        let removed = clear_agent_notes(&doc).unwrap();
        assert_eq!(removed, 1);

        let txn = doc.transact();
        let blocks = txn.get_map("blocks").unwrap();
        assert!(blocks.get(&txn, "para1").is_some());
    }
}
