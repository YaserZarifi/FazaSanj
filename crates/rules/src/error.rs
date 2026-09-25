use thiserror::Error;

#[derive(Debug, Error)]
pub enum RulesError {
    #[error("cannot read rule file {file}: {source}")]
    Io {
        file: String,
        #[source]
        source: std::io::Error,
    },
    #[error("bad JSON in {file}: {source}")]
    Json {
        file: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("rule {id}: bad pattern {pattern:?}: {reason}")]
    Pattern { id: String, pattern: String, reason: String },
    #[error("invalid rules: {}", .0.join("; "))]
    Invalid(Vec<String>),
}
