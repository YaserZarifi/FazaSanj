//! Optional AI explanations for unknown folders (bring your own key).
//!
//! Only folder metadata is ever sent (sizes, counts, dates, extension stats and child names),
//! never file contents. The username is always masked and names can be masked too.

mod cache_key;
mod error;
pub mod keys;
pub mod models;
mod payload;
pub mod prompt;
pub mod validate;

pub use cache_key::cache_key;
pub use error::AiError;
pub use keys::ApiKey;
pub use models::{available_models, default_model, provider_id, provider_name, ALL_PROVIDERS};
pub use payload::{build_payload, preview_payload, FolderMeta};
pub use validate::{parse_answer, CheckedAnswer};
