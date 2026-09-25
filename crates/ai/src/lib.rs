//! Optional AI explanations for unknown folders (bring your own key).
//!
//! Only folder metadata is ever sent (sizes, counts, dates, extension stats and child names),
//! never file contents. The username is always masked and names can be masked too.

mod cache_key;
mod client;
mod error;
pub mod keys;
pub mod models;
mod payload;
pub mod prompt;
mod providers;
pub mod validate;

pub use cache_key::cache_key;
pub use client::{explain, test_key, AiClient, CONNECT_TIMEOUT, TOTAL_TIMEOUT};
pub use error::AiError;
pub use keys::ApiKey;
pub use models::{available_models, default_model, provider_id, provider_name, ALL_PROVIDERS};
pub use payload::{build_payload, preview_payload, FolderMeta};
pub use providers::default_base_url;
pub use validate::{parse_answer, CheckedAnswer};
