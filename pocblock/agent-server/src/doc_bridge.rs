use serde::{Deserialize, Serialize};
use yrs::{Array, Doc, GetString, Map, Out, ReadTxn, Transact};

/// A simplified, agent-friendly view of the BlockSuite document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentView {
    pub blocks: Vec<BlockView>,
}

/// One block in the document tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockView {
    pub id: String,
    pub flavour: String,
    pub text: Option<String>,
    pub block_type: Option<String>,
    pub props: serde_json::Value,
    pub children: Vec<String>,
    pub parent: Option<String>,
}

impl DocumentView {
    /// Extract only human-authored text blocks for agent analysis.
    /// Filters out agent notes to prevent feedback loops.
    pub fn text_for_analysis(&self) -> String {
        self.blocks
            .iter()
            .filter(|b| b.flavour == "affine:paragraph" || b.flavour == "affine:list")
            .filter(|b| b.text.is_some())
            .map(|b| format!("[{}] {}", b.id, b.text.as_deref().unwrap_or("")))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Find the last content block ID (for appending summaries)
    pub fn last_content_block_id(&self) -> Option<&str> {
        self.blocks
            .iter()
            .filter(|b| b.flavour.starts_with("affine:"))
            .last()
            .map(|b| b.id.as_str())
    }
}

fn out_to_string(out: &Out) -> Option<String> {
    match out {
        Out::Any(yrs::Any::String(s)) => Some(s.to_string()),
        _ => None,
    }
}

/// Read the current state of a Yrs Doc into a DocumentView.
pub fn read_document(doc: &Doc) -> DocumentView {
    let txn = doc.transact();
    let mut blocks = Vec::new();

    if let Some(blocks_map) = txn.get_map("blocks") {
        for (key, value) in blocks_map.iter(&txn) {
            if let Out::YMap(block_map) = value {
                let flavour = block_map
                    .get(&txn, "sys:flavour")
                    .and_then(|v| out_to_string(&v))
                    .unwrap_or_default();

                let text = block_map.get(&txn, "prop:text").and_then(|v| match v {
                    Out::YText(t) => Some(t.get_string(&txn)),
                    Out::Any(yrs::Any::String(s)) => Some(s.to_string()),
                    _ => None,
                });

                let block_type = block_map
                    .get(&txn, "prop:type")
                    .and_then(|v| out_to_string(&v));

                let children = block_map
                    .get(&txn, "sys:children")
                    .map(|v| match v {
                        Out::YArray(arr) => arr
                            .iter(&txn)
                            .filter_map(|item| out_to_string(&item))
                            .collect(),
                        _ => vec![],
                    })
                    .unwrap_or_default();

                blocks.push(BlockView {
                    id: key.to_string(),
                    flavour,
                    text,
                    block_type,
                    props: serde_json::Value::Null,
                    children,
                    parent: None,
                });
            }
        }
    }

    DocumentView { blocks }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_doc_returns_empty_view() {
        let doc = Doc::new();
        let view = read_document(&doc);
        assert!(view.blocks.is_empty());
    }

    #[test]
    fn text_for_analysis_excludes_agent_notes() {
        let view = DocumentView {
            blocks: vec![
                BlockView {
                    id: "b1".into(),
                    flavour: "affine:paragraph".into(),
                    text: Some("Human text".into()),
                    block_type: Some("text".into()),
                    props: serde_json::Value::Null,
                    children: vec![],
                    parent: None,
                },
                BlockView {
                    id: "b2".into(),
                    flavour: "sparkdown:agent-note".into(),
                    text: Some("Agent text".into()),
                    block_type: None,
                    props: serde_json::Value::Null,
                    children: vec![],
                    parent: None,
                },
            ],
        };

        let analysis_text = view.text_for_analysis();
        assert!(analysis_text.contains("Human text"));
        assert!(!analysis_text.contains("Agent text"));
    }
}
