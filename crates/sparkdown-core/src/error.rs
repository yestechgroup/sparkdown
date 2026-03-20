use thiserror::Error;

#[derive(Debug, Error)]
pub enum SparkdownError {
    #[error("parse error: {0}")]
    Parse(String),

    #[error("unknown prefix: {0}")]
    UnknownPrefix(String),

    #[error("invalid IRI: {0}")]
    InvalidIri(String),

    #[error("frontmatter error: {0}")]
    Frontmatter(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
