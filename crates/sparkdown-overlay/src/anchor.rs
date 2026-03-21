//! Anchor types and span arithmetic for positional linking into markdown source.

use std::ops::Range;

/// Byte-span anchor into the markdown source.
///
/// `span.end == usize::MAX` represents an open-ended anchor (e.g., `[0..]`
/// meaning "the entire document" or `[500..]` meaning "from byte 500 to EOF").
#[derive(Debug, Clone)]
pub struct Anchor {
    /// Byte range in source. `end == usize::MAX` means open-ended.
    pub span: Range<usize>,
    /// First ~40 chars of anchored text, used for staleness verification.
    pub snippet: String,
}

/// Status of an anchor relative to the current markdown source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AnchorStatus {
    /// Anchor verified against current source.
    Synced,
    /// Source changed under this anchor; needs AI review.
    Stale,
    /// Anchored text was deleted entirely.
    Detached,
}

impl Anchor {
    /// Create a new anchor with the given byte range and snippet.
    pub fn new(span: Range<usize>, snippet: impl Into<String>) -> Self {
        Self {
            span,
            snippet: snippet.into(),
        }
    }

    /// Whether this anchor is open-ended (extends to EOF).
    pub fn is_open_ended(&self) -> bool {
        self.span.end == usize::MAX
    }

    /// The effective end position, clamped to `source_len` for open-ended anchors.
    pub fn effective_end(&self, source_len: usize) -> usize {
        if self.is_open_ended() {
            source_len
        } else {
            self.span.end
        }
    }

    /// Whether this anchor's span overlaps a given range.
    pub fn overlaps(&self, other: &Range<usize>) -> bool {
        self.span.start < other.end && other.start < self.effective_end_for_overlap()
    }

    /// Whether this anchor fully contains a given range.
    pub fn contains(&self, other: &Range<usize>) -> bool {
        self.span.start <= other.start && other.end <= self.effective_end_for_overlap()
    }

    /// Whether a given range fully contains this anchor.
    pub fn is_contained_by(&self, other: &Range<usize>) -> bool {
        other.start <= self.span.start && self.effective_end_for_overlap() <= other.end
    }

    /// Shift this anchor by a signed delta. Open-ended anchors only shift their start.
    pub fn shift(&mut self, delta: isize) {
        self.span.start = (self.span.start as isize + delta).max(0) as usize;
        if !self.is_open_ended() {
            self.span.end = (self.span.end as isize + delta).max(0) as usize;
        }
    }

    /// Verify that the snippet matches the text at the anchor's span in the source.
    /// Returns `true` if they match, `false` if stale.
    /// Open-ended anchors always pass snippet verification.
    pub fn verify_snippet(&self, source: &str) -> bool {
        if self.is_open_ended() {
            return true;
        }
        if self.snippet.is_empty() {
            return true;
        }
        let end = self.span.end.min(source.len());
        if self.span.start >= source.len() {
            return false;
        }
        let text = &source[self.span.start..end];
        let snippet_len = self.snippet.len().min(text.len());
        text[..snippet_len] == self.snippet[..snippet_len]
    }

    /// For overlap calculations, treat open-ended as a very large value.
    fn effective_end_for_overlap(&self) -> usize {
        self.span.end // usize::MAX already works for comparisons
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_ended_anchor() {
        let a = Anchor::new(0..usize::MAX, "");
        assert!(a.is_open_ended());
        assert_eq!(a.effective_end(500), 500);
    }

    #[test]
    fn closed_anchor() {
        let a = Anchor::new(10..50, "hello");
        assert!(!a.is_open_ended());
        assert_eq!(a.effective_end(500), 50);
    }

    #[test]
    fn overlap_detection() {
        let a = Anchor::new(10..50, "");

        // Overlapping ranges
        assert!(a.overlaps(&(0..20)));
        assert!(a.overlaps(&(40..60)));
        assert!(a.overlaps(&(20..30)));
        assert!(a.overlaps(&(0..100)));

        // Non-overlapping ranges
        assert!(!a.overlaps(&(0..10)));
        assert!(!a.overlaps(&(50..60)));
        assert!(!a.overlaps(&(60..70)));
    }

    #[test]
    fn containment() {
        let a = Anchor::new(10..50, "");

        assert!(a.contains(&(10..50)));
        assert!(a.contains(&(20..30)));
        assert!(!a.contains(&(5..30)));
        assert!(!a.contains(&(20..60)));
    }

    #[test]
    fn shift_closed_anchor() {
        let mut a = Anchor::new(10..50, "");
        a.shift(5);
        assert_eq!(a.span, 15..55);

        a.shift(-20);
        assert_eq!(a.span, 0..35);
    }

    #[test]
    fn shift_open_ended_only_moves_start() {
        let mut a = Anchor::new(100..usize::MAX, "");
        a.shift(50);
        assert_eq!(a.span.start, 150);
        assert!(a.is_open_ended());
    }

    #[test]
    fn snippet_verification() {
        let source = "Hello, world! This is some text.";
        let a = Anchor::new(0..13, "Hello, world!");
        assert!(a.verify_snippet(source));

        let a2 = Anchor::new(0..13, "Goodbye");
        assert!(!a2.verify_snippet(source));
    }

    #[test]
    fn snippet_verification_open_ended_always_passes() {
        let a = Anchor::new(0..usize::MAX, "anything");
        assert!(a.verify_snippet("completely different text"));
    }

    #[test]
    fn empty_span() {
        let a = Anchor::new(10..10, "");
        assert!(!a.overlaps(&(0..5)));
        assert!(!a.overlaps(&(15..20)));
        // Empty range at same position
        assert!(!a.overlaps(&(10..10)));
    }

    #[test]
    fn anchor_status_ordering() {
        // Synced < Stale < Detached — useful for "worst status" propagation
        assert!(AnchorStatus::Synced < AnchorStatus::Stale);
        assert!(AnchorStatus::Stale < AnchorStatus::Detached);
    }
}
