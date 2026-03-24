use serde::{Deserialize, Serialize};
use yrs::{
    Doc, GetString, ReadTxn, Transact, Xml, XmlElementRef, XmlFragment as XmlFragmentTrait,
    XmlFragmentRef, XmlOut,
};

/// A simplified, agent-friendly view of the ProseMirror/Tiptap document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentView {
    pub nodes: Vec<NodeView>,
}

/// One node in the ProseMirror document tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeView {
    /// Sequential index in the flat node list (used as a stable reference for agents)
    pub index: usize,
    /// ProseMirror node type: "paragraph", "heading", "codeBlock", "agentNote", etc.
    pub node_type: String,
    /// Plain text content of the node
    pub text: Option<String>,
    /// Node attributes (e.g., level for headings, agentId for agent notes)
    pub attrs: serde_json::Value,
}

impl DocumentView {
    /// Extract only human-authored text nodes for agent analysis.
    /// Filters out agentNote nodes to prevent feedback loops.
    pub fn text_for_analysis(&self) -> String {
        self.nodes
            .iter()
            .filter(|n| n.node_type != "agentNote")
            .filter(|n| n.text.is_some())
            .map(|n| format!("[{}] {}", n.index, n.text.as_deref().unwrap_or("")))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Find the last content node index (for appending summaries)
    pub fn last_content_node_index(&self) -> Option<usize> {
        self.nodes
            .iter()
            .filter(|n| n.node_type != "agentNote")
            .last()
            .map(|n| n.index)
    }
}

/// Read the current state of a Yrs Doc into a DocumentView.
///
/// Tiptap with y-prosemirror stores the document as:
///   Y.XmlFragment("default") containing Y.XmlElement children
///   Each XmlElement has a tag name matching the ProseMirror node type
///   Text content lives in Y.XmlText children of the elements
pub fn read_document(doc: &Doc) -> DocumentView {
    let txn = doc.transact();
    let mut nodes = Vec::new();

    if let Some(fragment) = txn.get_xml_fragment("default") {
        read_fragment(&txn, &fragment, &mut nodes);
    }

    DocumentView { nodes }
}

fn read_fragment(txn: &yrs::Transaction, fragment: &XmlFragmentRef, nodes: &mut Vec<NodeView>) {
    let len = fragment.len(txn);
    for i in 0..len {
        if let Some(child) = fragment.get(txn, i) {
            match child {
                XmlOut::Element(el) => read_element(txn, &el, nodes),
                XmlOut::Text(t) => {
                    let text = t.get_string(txn);
                    if !text.is_empty() {
                        nodes.push(NodeView {
                            index: nodes.len(),
                            node_type: "text".to_string(),
                            text: Some(text),
                            attrs: serde_json::Value::Null,
                        });
                    }
                }
                _ => {}
            }
        }
    }
}

fn read_element(txn: &yrs::Transaction, el: &XmlElementRef, nodes: &mut Vec<NodeView>) {
    let node_type = el.tag().to_string();

    // Collect attributes
    let mut attrs = serde_json::Map::new();
    for (key, value) in el.attributes(txn) {
        let k: String = key.to_string();
        let v: String = value.to_string();
        attrs.insert(k, serde_json::Value::String(v));
    }

    // Collect text from XmlText children
    let mut text_parts = Vec::new();
    let child_count = el.len(txn);
    for i in 0..child_count {
        if let Some(child) = el.get(txn, i) {
            match child {
                XmlOut::Text(t) => {
                    text_parts.push(t.get_string(txn));
                }
                XmlOut::Element(child_el) => {
                    // Recursively handle nested elements (e.g., list items)
                    read_element(txn, &child_el, nodes);
                }
                _ => {}
            }
        }
    }

    let text = if text_parts.is_empty() {
        None
    } else {
        let joined = text_parts.join("");
        if joined.is_empty() {
            None
        } else {
            Some(joined)
        }
    };

    let attrs_value = if attrs.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::Value::Object(attrs)
    };

    nodes.push(NodeView {
        index: nodes.len(),
        node_type,
        text,
        attrs: attrs_value,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use yrs::WriteTxn;

    #[test]
    fn empty_doc_returns_empty_view() {
        let doc = Doc::new();
        let view = read_document(&doc);
        assert!(view.nodes.is_empty());
    }

    #[test]
    fn reads_xml_fragment_content() {
        let doc = Doc::new();
        {
            let mut txn = doc.transact_mut();
            let frag = txn.get_or_insert_xml_fragment("default");
            let para = frag.insert(&mut txn, 0, yrs::XmlElementPrelim::empty("paragraph"));
            para.insert(&mut txn, 0, yrs::XmlTextPrelim::new("Hello world"));
        }

        let view = read_document(&doc);
        assert_eq!(view.nodes.len(), 1);
        assert_eq!(view.nodes[0].node_type, "paragraph");
        assert_eq!(view.nodes[0].text.as_deref(), Some("Hello world"));
    }

    #[test]
    fn text_for_analysis_excludes_agent_notes() {
        let view = DocumentView {
            nodes: vec![
                NodeView {
                    index: 0,
                    node_type: "paragraph".into(),
                    text: Some("Human text".into()),
                    attrs: serde_json::Value::Null,
                },
                NodeView {
                    index: 1,
                    node_type: "agentNote".into(),
                    text: Some("Agent text".into()),
                    attrs: serde_json::Value::Null,
                },
            ],
        };

        let analysis_text = view.text_for_analysis();
        assert!(analysis_text.contains("Human text"));
        assert!(!analysis_text.contains("Agent text"));
    }
}
