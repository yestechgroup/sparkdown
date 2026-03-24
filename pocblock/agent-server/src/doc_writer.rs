use anyhow::Result;
use yrs::{
    Doc, Transact, WriteTxn, XmlElementPrelim, XmlFragment as XmlFragmentTrait, XmlOut,
    XmlTextPrelim, Xml,
};

/// Insert an agent note node into the Yrs document (ProseMirror/Tiptap schema).
///
/// The note is inserted as an XmlElement("agentNote") into the Y.XmlFragment("default").
/// If `after_node_index` is provided, the note is placed after that node index.
/// Otherwise it's appended at the end.
pub fn insert_agent_note(
    doc: &Doc,
    after_node_index: Option<usize>,
    agent_id: &str,
    agent_name: &str,
    note_type: &str,
    content: &str,
    confidence: f64,
) -> Result<()> {
    let mut txn = doc.transact_mut();
    let frag = txn.get_or_insert_xml_fragment("default");

    // Determine insertion position
    let child_count = frag.len(&txn);
    let insert_pos = match after_node_index {
        Some(idx) => {
            let pos = (idx as u32 + 1).min(child_count);
            pos
        }
        None => child_count,
    };

    // Create the agentNote element
    let el = frag.insert(&mut txn, insert_pos, XmlElementPrelim::empty("agentNote"));

    // Set attributes
    el.insert_attribute(&mut txn, "agentId", agent_id);
    el.insert_attribute(&mut txn, "agentName", agent_name);
    el.insert_attribute(&mut txn, "noteType", note_type);
    el.insert_attribute(&mut txn, "confidence", &confidence.to_string());
    el.insert_attribute(&mut txn, "accepted", "false");

    // Set text content
    el.insert(&mut txn, 0, XmlTextPrelim::new(content));

    Ok(())
}

/// Remove all agentNote nodes from the document.
pub fn clear_agent_notes(doc: &Doc) -> Result<usize> {
    let mut txn = doc.transact_mut();
    let frag = txn.get_or_insert_xml_fragment("default");

    // Collect indices of agentNote elements (in reverse order for safe removal)
    let mut to_remove = Vec::new();
    let len = frag.len(&txn);
    for i in 0..len {
        if let Some(XmlOut::Element(el)) = frag.get(&txn, i) {
            if el.tag().as_ref() == "agentNote" {
                to_remove.push(i);
            }
        }
    }

    let count = to_remove.len();

    // Remove in reverse order so indices remain valid
    for idx in to_remove.into_iter().rev() {
        frag.remove_range(&mut txn, idx, 1);
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use yrs::{Doc, ReadTxn, Transact};

    #[test]
    fn insert_and_read_agent_note() {
        let doc = Doc::new();

        // Set up a minimal ProseMirror-like structure
        {
            let mut txn = doc.transact_mut();
            let frag = txn.get_or_insert_xml_fragment("default");
            let para = frag.insert(&mut txn, 0, XmlElementPrelim::empty("paragraph"));
            para.insert(&mut txn, 0, XmlTextPrelim::new("Hello world"));
        }

        insert_agent_note(
            &doc,
            Some(0), // after the paragraph
            "entity-detector",
            "Entity Detector",
            "entity",
            "schema:Person — Test",
            0.9,
        )
        .unwrap();

        let txn = doc.transact();
        let frag = txn.get_xml_fragment("default").unwrap();
        assert_eq!(frag.len(&txn), 2);

        // Second child should be the agent note
        if let Some(XmlOut::Element(el)) = frag.get(&txn, 1) {
            assert_eq!(el.tag().as_ref(), "agentNote");
            assert_eq!(
                el.get_attribute(&txn, "noteType"),
                Some("entity".to_string())
            );
        } else {
            panic!("Expected XmlElement at index 1");
        }
    }

    #[test]
    fn clear_removes_only_agent_notes() {
        let doc = Doc::new();

        {
            let mut txn = doc.transact_mut();
            let frag = txn.get_or_insert_xml_fragment("default");
            let para = frag.insert(&mut txn, 0, XmlElementPrelim::empty("paragraph"));
            para.insert(&mut txn, 0, XmlTextPrelim::new("Keep this"));
        }

        insert_agent_note(&doc, Some(0), "test", "Test", "entity", "Remove this", 0.9).unwrap();
        insert_agent_note(&doc, Some(1), "test", "Test", "summary", "Also remove", 0.8).unwrap();

        let removed = clear_agent_notes(&doc).unwrap();
        assert_eq!(removed, 2);

        let txn = doc.transact();
        let frag = txn.get_xml_fragment("default").unwrap();
        assert_eq!(frag.len(&txn), 1); // Only the paragraph remains
    }
}
