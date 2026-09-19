//! Safe secret redaction wrapper to prevent credential leakage in logs, errors, and traces.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// Redacted wrapper around sensitive string values (keys, tokens, passwords).
///
/// Implements `Display` and `Debug` displaying `[REDACTED]` exclusively,
/// preventing accidental emission in `tracing::*`, `println!`, or error messages.
#[derive(Clone, PartialEq, Eq)]
pub struct RedactedSecret<T = String>(T);

impl<T> RedactedSecret<T> {
    pub fn new(secret: T) -> Self {
        Self(secret)
    }

    /// Explicitly access the inner secret value for authorized cryptographic or transport use.
    pub fn expose_secret(&self) -> &T {
        &self.0
    }

    /// Consumes the wrapper and returns the inner secret.
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T: AsRef<str>> RedactedSecret<T> {
    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }
}

impl<T> fmt::Debug for RedactedSecret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[REDACTED]")
    }
}

impl<T> fmt::Display for RedactedSecret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[REDACTED]")
    }
}

impl<T: Default> Default for RedactedSecret<T> {
    fn default() -> Self {
        Self(T::default())
    }
}

impl<T: From<String>> From<String> for RedactedSecret<T> {
    fn from(s: String) -> Self {
        Self(T::from(s))
    }
}

impl From<&str> for RedactedSecret<String> {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl Serialize for RedactedSecret<String> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str("[REDACTED]")
    }
}

impl<'de> Deserialize<'de> for RedactedSecret<String> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(RedactedSecret::new(s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_is_never_formatted_in_display_or_debug() {
        let sensitive = "sk_live_very_secret_key_1234567890";
        let secret = RedactedSecret::new(sensitive.to_string());

        assert_eq!(format!("{}", secret), "[REDACTED]");
        assert_eq!(format!("{:?}", secret), "[REDACTED]");
        assert!(!format!("{}", secret).contains(sensitive));
        assert!(!format!("{:?}", secret).contains(sensitive));

        // Authorized access still succeeds
        assert_eq!(secret.expose_secret(), sensitive);
        assert_eq!(secret.as_str(), sensitive);
    }

    #[test]
    fn test_serde_redaction() {
        let sensitive = "my-db-password-super-secret";
        let secret = RedactedSecret::new(sensitive.to_string());

        let json = serde_json::to_string(&secret).unwrap();
        assert_eq!(json, "\"[REDACTED]\"");
        assert!(!json.contains(sensitive));
    }
}
