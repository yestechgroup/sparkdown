use sparkdown_core::ast::SparkdownDocument;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("render error: {0}")]
    Other(String),
}

/// Trait for output format renderers.
pub trait OutputRenderer {
    /// Render the document to the given writer.
    fn render(&self, doc: &SparkdownDocument, out: &mut dyn std::io::Write)
        -> Result<(), RenderError>;

    /// MIME content type.
    fn content_type(&self) -> &str;

    /// File extension (without dot).
    fn file_extension(&self) -> &str;
}
