//! Parser and serializer for `.sparkdown-sem` sidecar files.
//!
//! The format is Turtle-inspired with an extension for byte-span anchors.

use oxrdf::{BlankNode, NamedNode};
use sparkdown_core::prefix::PrefixMap;

use crate::anchor::{Anchor, AnchorStatus};
use crate::graph::{OverlayError, SemanticEntity, SemanticGraph, Triple, TripleObject};
use crate::vocab;

/// Parse a `.sparkdown-sem` sidecar file into a `SemanticGraph`.
pub fn parse(input: &str) -> Result<SemanticGraph, OverlayError> {
    let mut parser = Parser::new(input);
    parser.parse()
}

/// Serialize a `SemanticGraph` to `.sparkdown-sem` format.
pub fn serialize(graph: &SemanticGraph) -> String {
    let mut out = String::new();

    // Source hash
    out.push_str(&format!(
        "@source-hash \"sha256:{}\" .\n",
        graph.source_hash_hex()
    ));

    // Prefixes
    for (prefix, iri) in graph.prefixes.iter() {
        // Skip standard RDF prefixes that aren't used in sidecar
        if ["rdf", "rdfs", "owl", "xsd", "wikidata", "skos", "foaf"].contains(&prefix) {
            continue;
        }
        out.push_str(&format!("@prefix {prefix}: <{iri}> .\n"));
    }
    out.push('\n');

    // Entity blocks (with anchors)
    for entity in &graph.entities {
        let id = format!("_:{}", entity.id.as_str());
        let anchor = format_anchor(&entity.anchor);
        out.push_str(&format!("{id} {anchor}"));

        let mut first = true;

        // Types
        for ty in &entity.types {
            let curie = iri_to_curie(ty.as_str(), &graph.prefixes);
            if first {
                out.push_str(&format!(" a {curie}"));
                first = false;
            } else {
                out.push_str(&format!(" ;\n    a {curie}"));
            }
        }

        // Snippet
        if !entity.anchor.snippet.is_empty() {
            let escaped = entity.anchor.snippet.replace('"', "\\\"");
            if first {
                out.push_str(&format!(" sd:snippet \"{escaped}\""));
                first = false;
            } else {
                out.push_str(&format!(" ;\n    sd:snippet \"{escaped}\""));
            }
        }

        // Property triples for this entity
        for triple in graph.triples_for_subject(&entity.id) {
            let pred_curie = iri_to_curie(triple.predicate.as_str(), &graph.prefixes);
            // Skip snippet (already handled above) and type triples
            if triple.predicate.as_str() == vocab::snippet().as_str() {
                continue;
            }
            let obj_str = match &triple.object {
                TripleObject::Entity(bn) => format!("_:{}", bn.as_str()),
                TripleObject::Literal { value, .. } => format!("\"{}\"", value.replace('"', "\\\"")),
            };
            if first {
                out.push_str(&format!(" {pred_curie} {obj_str}"));
                first = false;
            } else {
                out.push_str(&format!(" ;\n    {pred_curie} {obj_str}"));
            }
        }

        out.push_str(" .\n\n");
    }

    // Relationship triples (not already serialized with entities)
    let entity_ids: std::collections::HashSet<_> =
        graph.entities.iter().map(|e| &e.id).collect();
    let mut relationship_triples: Vec<&Triple> = Vec::new();
    for triple in &graph.triples {
        // Include triples whose subject has no entity (relationship-only blocks)
        if !entity_ids.contains(&triple.subject) {
            relationship_triples.push(triple);
        }
    }
    // Also include triples where the subject IS an entity but the triple was
    // not serialized in the entity block (object references to other entities
    // that are standalone relationship triples per the spec)
    // Actually, all triples for an entity subject are serialized in the entity block.
    // Standalone relationship triples only exist for entities referencing other entities.
    // The spec shows these as separate blocks. Let's group by subject.
    let mut rel_subjects_seen = std::collections::HashSet::new();
    for triple in &graph.triples {
        if !entity_ids.contains(&triple.subject) && rel_subjects_seen.insert(&triple.subject) {
            let id = format!("_:{}", triple.subject.as_str());
            let mut first = true;
            for t in graph.triples_for_subject(&triple.subject) {
                let pred_curie = iri_to_curie(t.predicate.as_str(), &graph.prefixes);
                let obj_str = match &t.object {
                    TripleObject::Entity(bn) => format!("_:{}", bn.as_str()),
                    TripleObject::Literal { value, .. } => {
                        format!("\"{}\"", value.replace('"', "\\\""))
                    }
                };
                if first {
                    out.push_str(&format!("{id} {pred_curie} {obj_str}"));
                    first = false;
                } else {
                    out.push_str(&format!(" ;\n    {pred_curie} {obj_str}"));
                }
            }
            out.push_str(" .\n");
        }
    }

    out
}

fn format_anchor(anchor: &Anchor) -> String {
    if anchor.is_open_ended() {
        format!("[{}..]", anchor.span.start)
    } else {
        format!("[{}..{}]", anchor.span.start, anchor.span.end)
    }
}

fn iri_to_curie(iri: &str, prefixes: &PrefixMap) -> String {
    for (prefix, base) in prefixes.iter() {
        if let Some(local) = iri.strip_prefix(base) {
            return format!("{prefix}:{local}");
        }
    }
    // Fallback: return as full IRI in angle brackets
    format!("<{iri}>")
}

// --- Parser ---

struct Parser<'a> {
    input: &'a str,
    pos: usize,
    line: usize,
    col: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    fn parse(&mut self) -> Result<SemanticGraph, OverlayError> {
        let source_hash = self.parse_source_hash()?;
        let prefixes = self.parse_prefixes()?;

        let mut entities: Vec<SemanticEntity> = Vec::new();
        let mut triples: Vec<Triple> = Vec::new();

        self.skip_ws_and_comments();

        while self.pos < self.input.len() {
            self.skip_ws_and_comments();
            if self.pos >= self.input.len() {
                break;
            }

            // Expect a blank node ID
            let id = self.parse_blank_node()?;
            self.skip_ws();

            // Check if next is an anchor `[...]` or a predicate
            if self.peek() == Some('[') {
                // Entity block with anchor
                let anchor = self.parse_anchor()?;
                self.skip_ws();

                let (types, mut entity_triples, snippet) =
                    self.parse_predicate_list(&id, &prefixes)?;

                let anchor = if let Some(snip) = snippet {
                    Anchor::new(anchor.span, snip)
                } else {
                    anchor
                };

                entities.push(SemanticEntity {
                    id: id.clone(),
                    anchor,
                    types,
                    status: AnchorStatus::Synced,
                });
                triples.append(&mut entity_triples);
            } else {
                // Relationship block (no anchor)
                let (types, mut rel_triples, _snippet) =
                    self.parse_predicate_list(&id, &prefixes)?;

                // If there are type assignments, they become triples too
                for ty in types {
                    triples.push(Triple {
                        subject: id.clone(),
                        predicate: NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")
                            .unwrap(),
                        object: TripleObject::Entity(BlankNode::new(ty.as_str()).unwrap_or_else(
                            |_| {
                                // Type IRI as literal fallback — shouldn't happen normally
                                BlankNode::new("error").unwrap()
                            },
                        )),
                    });
                }
                triples.append(&mut rel_triples);
            }
        }

        Ok(SemanticGraph {
            source_hash,
            prefixes,
            entities,
            triples,
        })
    }

    fn parse_source_hash(&mut self) -> Result<[u8; 32], OverlayError> {
        self.skip_ws_and_comments();
        self.expect_str("@source-hash")?;
        self.skip_ws();
        let hash_str = self.parse_quoted_string()?;
        self.skip_ws();
        self.expect_char('.')?;
        self.skip_ws_and_newlines();

        // Parse "sha256:hexstring"
        let hex = hash_str
            .strip_prefix("sha256:")
            .ok_or_else(|| self.error("expected sha256: prefix in source hash"))?;

        let mut hash = [0u8; 32];
        let hex_bytes: Vec<u8> = (0..hex.len())
            .step_by(2)
            .map(|i| {
                u8::from_str_radix(&hex[i..i.min(hex.len()) + 2.min(hex.len() - i)], 16)
                    .unwrap_or(0)
            })
            .collect();
        let copy_len = hash.len().min(hex_bytes.len());
        hash[..copy_len].copy_from_slice(&hex_bytes[..copy_len]);

        Ok(hash)
    }

    fn parse_prefixes(&mut self) -> Result<PrefixMap, OverlayError> {
        let mut prefixes = PrefixMap::new();
        prefixes.seed_builtins();

        loop {
            self.skip_ws_and_comments();
            if !self.looking_at("@prefix") {
                break;
            }
            self.expect_str("@prefix")?;
            self.skip_ws();
            let name = self.parse_prefix_name()?;
            self.expect_char(':')?;
            self.skip_ws();
            self.expect_char('<')?;
            let iri = self.parse_until('>')?;
            self.expect_char('>')?;
            self.skip_ws();
            self.expect_char('.')?;
            self.skip_ws_and_newlines();

            prefixes.insert(name, iri);
        }

        Ok(prefixes)
    }

    fn parse_blank_node(&mut self) -> Result<BlankNode, OverlayError> {
        self.expect_str("_:")?;
        let start = self.pos;
        while self.pos < self.input.len()
            && (self.input.as_bytes()[self.pos].is_ascii_alphanumeric()
                || self.input.as_bytes()[self.pos] == b'_')
        {
            self.advance();
        }
        if self.pos == start {
            return Err(self.error("expected blank node identifier after _:"));
        }
        let name = &self.input[start..self.pos];
        BlankNode::new(name).map_err(|_| self.error(&format!("invalid blank node ID: _:{name}")))
    }

    fn parse_anchor(&mut self) -> Result<Anchor, OverlayError> {
        self.expect_char('[')?;
        let start = self.parse_usize()?;
        self.expect_str("..")?;

        let end = if self.peek() == Some(']') {
            usize::MAX // open-ended
        } else {
            self.parse_usize()?
        };

        self.expect_char(']')?;

        if end != usize::MAX && end < start {
            return Err(OverlayError::InvalidAnchor {
                entity: String::new(),
                reason: format!("end ({end}) < start ({start})"),
            });
        }

        Ok(Anchor::new(start..end, ""))
    }

    fn parse_predicate_list(
        &mut self,
        subject: &BlankNode,
        prefixes: &PrefixMap,
    ) -> Result<(Vec<NamedNode>, Vec<Triple>, Option<String>), OverlayError> {
        let mut types = Vec::new();
        let mut triples = Vec::new();
        let mut snippet = None;

        loop {
            self.skip_ws_and_comments();
            if self.peek() == Some('.') {
                self.advance();
                break;
            }

            let predicate = self.parse_predicate(prefixes)?;
            self.skip_ws();
            let object = self.parse_object(prefixes)?;

            // Handle special predicates
            if predicate.as_str() == "http://www.w3.org/1999/02/22-rdf-syntax-ns#type" {
                // rdf:type — add to types list
                match &object {
                    ParsedObject::Iri(iri) => types.push(iri.clone()),
                    ParsedObject::BlankNode(_) => {} // unusual but ok
                    ParsedObject::Literal { .. } => {} // invalid for rdf:type, ignore
                }
            } else if predicate.as_str() == vocab::snippet().as_str() {
                // sd:snippet — extract snippet text
                if let ParsedObject::Literal { value, .. } = &object {
                    snippet = Some(value.clone());
                }
            } else {
                // Regular triple
                let triple_object = match object {
                    ParsedObject::BlankNode(bn) => TripleObject::Entity(bn),
                    ParsedObject::Iri(iri) => TripleObject::Entity(
                        BlankNode::new(iri.as_str()).unwrap_or_else(|_| {
                            BlankNode::new("ref").unwrap()
                        }),
                    ),
                    ParsedObject::Literal { value, datatype } => {
                        TripleObject::Literal { value, datatype }
                    }
                };
                triples.push(Triple {
                    subject: subject.clone(),
                    predicate,
                    object: triple_object,
                });
            }

            self.skip_ws_and_comments();
            if self.peek() == Some(';') {
                self.advance();
                continue;
            }
            if self.peek() == Some('.') {
                self.advance();
                break;
            }

            return Err(self.error("expected ';' or '.' in predicate list"));
        }

        Ok((types, triples, snippet))
    }

    fn parse_predicate(&mut self, prefixes: &PrefixMap) -> Result<NamedNode, OverlayError> {
        if self.looking_at("a ") || self.looking_at("a\t") {
            self.advance(); // skip 'a'
            return Ok(
                NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type").unwrap(),
            );
        }
        self.parse_curie_as_iri(prefixes)
    }

    fn parse_object(&mut self, prefixes: &PrefixMap) -> Result<ParsedObject, OverlayError> {
        self.skip_ws();
        match self.peek() {
            Some('"') => {
                let value = self.parse_quoted_string()?;
                Ok(ParsedObject::Literal {
                    value,
                    datatype: None,
                })
            }
            Some('_') => {
                let bn = self.parse_blank_node()?;
                Ok(ParsedObject::BlankNode(bn))
            }
            _ => {
                let iri = self.parse_curie_as_iri(prefixes)?;
                Ok(ParsedObject::Iri(iri))
            }
        }
    }

    fn parse_curie_as_iri(&mut self, prefixes: &PrefixMap) -> Result<NamedNode, OverlayError> {
        let start = self.pos;
        // Read prefix:local
        while self.pos < self.input.len() {
            let ch = self.input.as_bytes()[self.pos];
            if ch.is_ascii_alphanumeric() || ch == b':' || ch == b'_' || ch == b'-' || ch == b'/' {
                self.advance();
            } else {
                break;
            }
        }
        let token = &self.input[start..self.pos];
        if token.is_empty() {
            return Err(self.error("expected CURIE or IRI"));
        }

        prefixes
            .resolve(token)
            .map_err(|_| OverlayError::UnresolvedPrefix(token.to_string()))
    }

    fn parse_quoted_string(&mut self) -> Result<String, OverlayError> {
        self.expect_char('"')?;
        let mut s = String::new();
        loop {
            match self.peek() {
                None => return Err(self.error("unterminated string")),
                Some('\\') => {
                    self.advance();
                    match self.peek() {
                        Some('"') => {
                            s.push('"');
                            self.advance();
                        }
                        Some('\\') => {
                            s.push('\\');
                            self.advance();
                        }
                        Some('n') => {
                            s.push('\n');
                            self.advance();
                        }
                        _ => s.push('\\'),
                    }
                }
                Some('"') => {
                    self.advance();
                    return Ok(s);
                }
                Some(ch) => {
                    s.push(ch);
                    self.advance();
                }
            }
        }
    }

    fn parse_prefix_name(&mut self) -> Result<String, OverlayError> {
        let start = self.pos;
        while self.pos < self.input.len() {
            let ch = self.input.as_bytes()[self.pos];
            if ch.is_ascii_alphanumeric() || ch == b'_' || ch == b'-' {
                self.advance();
            } else {
                break;
            }
        }
        if self.pos == start {
            return Err(self.error("expected prefix name"));
        }
        Ok(self.input[start..self.pos].to_string())
    }

    fn parse_usize(&mut self) -> Result<usize, OverlayError> {
        let start = self.pos;
        while self.pos < self.input.len() && self.input.as_bytes()[self.pos].is_ascii_digit() {
            self.advance();
        }
        if self.pos == start {
            return Err(self.error("expected integer"));
        }
        self.input[start..self.pos]
            .parse()
            .map_err(|_| self.error("integer out of range"))
    }

    fn parse_until(&mut self, ch: char) -> Result<String, OverlayError> {
        let start = self.pos;
        while self.pos < self.input.len() && self.input.as_bytes()[self.pos] != ch as u8 {
            self.advance();
        }
        Ok(self.input[start..self.pos].to_string())
    }

    // --- Helpers ---

    fn peek(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn advance(&mut self) {
        if self.pos < self.input.len() {
            if self.input.as_bytes()[self.pos] == b'\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
            self.pos += 1;
        }
    }

    fn skip_ws(&mut self) {
        while self.pos < self.input.len() {
            let ch = self.input.as_bytes()[self.pos];
            if ch == b' ' || ch == b'\t' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn skip_ws_and_newlines(&mut self) {
        while self.pos < self.input.len() {
            let ch = self.input.as_bytes()[self.pos];
            if ch == b' ' || ch == b'\t' || ch == b'\n' || ch == b'\r' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn skip_ws_and_comments(&mut self) {
        loop {
            self.skip_ws_and_newlines();
            if self.pos < self.input.len() && self.input.as_bytes()[self.pos] == b'#' {
                // Skip comment line
                while self.pos < self.input.len() && self.input.as_bytes()[self.pos] != b'\n' {
                    self.advance();
                }
            } else {
                break;
            }
        }
    }

    fn looking_at(&self, s: &str) -> bool {
        self.input[self.pos..].starts_with(s)
    }

    fn expect_str(&mut self, s: &str) -> Result<(), OverlayError> {
        if !self.looking_at(s) {
            return Err(self.error(&format!("expected '{s}'")));
        }
        for _ in s.chars() {
            self.advance();
        }
        Ok(())
    }

    fn expect_char(&mut self, ch: char) -> Result<(), OverlayError> {
        if self.peek() != Some(ch) {
            return Err(self.error(&format!("expected '{ch}'")));
        }
        self.advance();
        Ok(())
    }

    fn error(&self, message: &str) -> OverlayError {
        OverlayError::Parse {
            line: self.line,
            col: self.col,
            message: message.to_string(),
        }
    }
}

/// Intermediate object representation during parsing.
enum ParsedObject {
    BlankNode(BlankNode),
    Iri(NamedNode),
    Literal {
        value: String,
        datatype: Option<NamedNode>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE_SIDECAR: &str = r#"@source-hash "sha256:a1b2c3d4e5f60000000000000000000000000000000000000000000000000000" .
@prefix schema: <http://schema.org/> .
@prefix sd: <urn:sparkdown:vocab/> .
@prefix dc: <http://purl.org/dc/terms/> .

# Document-level
_:doc [0..] a schema:Event ;
    schema:name "RustConf 2026" ;
    schema:startDate "2026-09-10" ;
    schema:endDate "2026-09-12" .

# Entities
_:e1 [142..158] a schema:Person ;
    sd:snippet "Niko Matsakis" .

_:e2 [210..218] a schema:Place ;
    schema:name "Portland" ;
    sd:snippet "Portland" .

# Relationships
_:e1 schema:performerIn _:doc .
_:doc schema:location _:e2 .

# Rhetorical structure
_:s1 [1200..1450] a sd:Section ;
    sd:role sd:Review ;
    sd:snippet "An excellent conference..." .
"#;

    #[test]
    fn parse_example_sidecar() {
        let graph = parse(EXAMPLE_SIDECAR).unwrap();

        // Check source hash
        assert_eq!(graph.source_hash[0], 0xa1);
        assert_eq!(graph.source_hash[1], 0xb2);

        // Check entities
        assert_eq!(graph.entities.len(), 4); // doc, e1, e2, s1

        // Check doc entity
        let doc = graph.entity_by_id(&BlankNode::new("doc").unwrap()).unwrap();
        assert!(doc.anchor.is_open_ended());
        assert_eq!(doc.types.len(), 1);
        assert_eq!(doc.types[0].as_str(), "http://schema.org/Event");

        // Check e1 entity
        let e1 = graph.entity_by_id(&BlankNode::new("e1").unwrap()).unwrap();
        assert_eq!(e1.anchor.span, 142..158);
        assert_eq!(e1.anchor.snippet, "Niko Matsakis");

        // Check e2 entity
        let e2 = graph.entity_by_id(&BlankNode::new("e2").unwrap()).unwrap();
        assert_eq!(e2.anchor.span, 210..218);

        // Check s1 entity
        let s1 = graph.entity_by_id(&BlankNode::new("s1").unwrap()).unwrap();
        assert_eq!(s1.anchor.span, 1200..1450);

        // Check triples
        assert!(graph.triples.len() >= 5); // name, dates, location, performerIn, role + others
    }

    #[test]
    fn round_trip() {
        let graph = parse(EXAMPLE_SIDECAR).unwrap();
        let serialized = serialize(&graph);
        let reparsed = parse(&serialized).unwrap();

        assert_eq!(graph.entities.len(), reparsed.entities.len());
        assert_eq!(graph.source_hash, reparsed.source_hash);

        for (orig, re) in graph.entities.iter().zip(reparsed.entities.iter()) {
            assert_eq!(orig.id, re.id);
            assert_eq!(orig.anchor.span, re.anchor.span);
            assert_eq!(orig.types.len(), re.types.len());
        }
    }

    #[test]
    fn missing_source_hash_errors() {
        let input = "@prefix schema: <http://schema.org/> .\n_:e1 [0..10] a schema:Thing .\n";
        let result = parse(input);
        assert!(result.is_err());
    }

    #[test]
    fn malformed_anchor_end_before_start() {
        let input = "@source-hash \"sha256:0000000000000000000000000000000000000000000000000000000000000000\" .\n_:e1 [10..5] a schema:Thing .\n";
        let result = parse(input);
        assert!(result.is_err());
    }

    #[test]
    fn unterminated_string_errors() {
        let input = "@source-hash \"sha256:0000000000000000000000000000000000000000000000000000000000000000\" .\n@prefix schema: <http://schema.org/> .\n_:e1 [0..10] schema:name \"unterminated .\n";
        let result = parse(input);
        assert!(result.is_err());
    }

    #[test]
    fn unresolved_prefix_errors() {
        let input = "@source-hash \"sha256:0000000000000000000000000000000000000000000000000000000000000000\" .\n_:e1 [0..10] a unknown:Thing .\n";
        let result = parse(input);
        assert!(result.is_err());
    }

    #[test]
    fn open_ended_anchor() {
        let input = "@source-hash \"sha256:0000000000000000000000000000000000000000000000000000000000000000\" .\n@prefix schema: <http://schema.org/> .\n_:doc [0..] a schema:Event .\n";
        let graph = parse(input).unwrap();
        assert_eq!(graph.entities.len(), 1);
        assert!(graph.entities[0].anchor.is_open_ended());
        assert_eq!(graph.entities[0].anchor.span.start, 0);
    }
}
