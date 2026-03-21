use serde::{Deserialize, Serialize};

/// Identifies an open document by absolute path.
pub type DocId = String;

/// Output format for export commands.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RenderFormat {
    HtmlRdfa,
    JsonLd,
    Turtle,
}

/// Entity data transfer object — pre-resolved for frontend consumption.
/// Spans are character offsets (UTF-16), ready for CodeMirror.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityDto {
    pub id: String,
    pub label: String,
    pub type_iris: Vec<String>,
    pub type_prefix: String,
    pub span_start: usize,
    pub span_end: usize,
    pub status: EntityStatus,
    pub top_relations: Vec<Relation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityStatus {
    Synced,
    Stale,
    Detached,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub predicate_label: String,
    pub target_label: String,
    pub target_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidecarStatus {
    pub synced: usize,
    pub stale: usize,
    pub detached: usize,
    pub total_triples: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaleAnchor {
    pub entity_id: String,
    pub old_snippet: String,
    pub new_text: String,
    pub span_start: usize,
    pub span_end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub has_sidecar: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub path: String,
    pub files: Vec<FileEntry>,
}

/// Maps byte offset to UTF-16 character offset for CodeMirror compatibility.
pub fn byte_to_char_offset(source: &str, byte_offset: usize) -> usize {
    source[..byte_offset].encode_utf16().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_dto_serializes_to_json() {
        let entity = EntityDto {
            id: "_:e1".into(),
            label: "Niko Matsakis".into(),
            type_iris: vec!["http://schema.org/Person".into()],
            type_prefix: "schema:Person".into(),
            span_start: 0,
            span_end: 13,
            status: EntityStatus::Synced,
            top_relations: vec![Relation {
                predicate_label: "performerIn".into(),
                target_label: "RustConf".into(),
                target_id: "_:e2".into(),
            }],
        };
        let json = serde_json::to_string(&entity).unwrap();
        assert!(json.contains("Niko Matsakis"));
        assert!(json.contains("synced"));
        assert!(json.contains("performerIn"));
    }

    #[test]
    fn byte_to_char_ascii() {
        assert_eq!(byte_to_char_offset("hello world", 5), 5);
    }

    #[test]
    fn byte_to_char_multibyte() {
        // "cafe\u{0301}" = "café" — the é is 2 bytes (U+0301 combining accent)
        let s = "caf\u{00e9}!"; // é is 2 bytes in UTF-8, 1 UTF-16 code unit
        assert_eq!(byte_to_char_offset(s, 3), 3); // "caf" = 3 bytes, 3 chars
        assert_eq!(byte_to_char_offset(s, 5), 4); // "café" = 5 bytes, 4 chars
    }

    #[test]
    fn sidecar_status_serializes() {
        let status = SidecarStatus {
            synced: 3,
            stale: 1,
            detached: 0,
            total_triples: 12,
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"synced\":3"));
    }
}
