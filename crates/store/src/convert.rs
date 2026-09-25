//! SQLite integers are signed 64 bit. Sizes never get near 2^63, so clamping is fine.

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::{Result, StoreError};

pub(crate) fn to_i64(v: u64) -> i64 {
    i64::try_from(v).unwrap_or(i64::MAX)
}

pub(crate) fn to_u64(v: i64) -> u64 {
    u64::try_from(v).unwrap_or(0)
}

/// Stores a unit enum by its serde name ("done", "recycle"...) so the db matches the UI names.
pub(crate) fn enum_to_str<T: Serialize>(v: &T) -> Result<String> {
    match serde_json::to_value(v)? {
        serde_json::Value::String(s) => Ok(s),
        other => Err(StoreError::BadValue(other.to_string())),
    }
}

pub(crate) fn enum_from_str<T: DeserializeOwned>(s: &str) -> Result<T> {
    serde_json::from_value(serde_json::Value::String(s.to_owned()))
        .map_err(|_| StoreError::BadValue(s.to_owned()))
}

/// Case-insensitive key for Windows paths. Trailing separators are dropped except for
/// drive roots, so `C:\Users\` and `c:\users` intern to the same row.
pub(crate) fn path_key(path: &str) -> String {
    let mut key = path.replace('/', "\\").to_lowercase();
    while key.len() > 3 && key.ends_with('\\') {
        key.pop();
    }
    key
}

#[cfg(test)]
mod tests {
    use super::*;
    use fazasanj_model::ActionStatus;

    #[test]
    fn enum_roundtrip() {
        let s = enum_to_str(&ActionStatus::SkippedInUse).unwrap();
        assert_eq!(s, "skipped_in_use");
        let back: ActionStatus = enum_from_str(&s).unwrap();
        assert_eq!(back, ActionStatus::SkippedInUse);
        assert!(enum_from_str::<ActionStatus>("nope").is_err());
    }

    #[test]
    fn keys() {
        assert_eq!(path_key("C:\\Users\\Ali\\"), "c:\\users\\ali");
        assert_eq!(path_key("C:\\"), "c:\\");
        assert_eq!(path_key("D:/Games"), "d:\\games");
        assert_eq!(to_i64(u64::MAX), i64::MAX);
        assert_eq!(to_u64(-5), 0);
    }
}
