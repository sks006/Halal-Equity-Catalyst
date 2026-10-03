use serde::{Deserialize, Serialize};

use super::source::EnvSource;

const DEFAULT_HERMES_URL: &str = "https://hermes.pyth.network";
const DEFAULT_PYTH_API_KEY: &str = "EN3nRKiYpkeAuE78ZxrvSPXzZVxF9Ps7sc3V7zgj5tMW";

/// Pyth / Hermes credentials.
///
/// `api_key` is never serialized. Debug output redacts it. The only way
/// to populate it is `from_source`, which reads the process environment.
#[derive(Clone, Serialize, Deserialize)]
pub struct PythConfig {
    pub hermes_url: String,

    /// Never serialized. Deserializing a `Config` yields `None` here,
    /// which is safe because production `Config` is built from env, not JSON.
    #[serde(skip)]
    pub api_key: Option<String>,
}

impl std::fmt::Debug for PythConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PythConfig")
            .field("hermes_url", &self.hermes_url)
            .field("api_key", &self.api_key.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}

impl Default for PythConfig {
    fn default() -> Self {
        Self {
            hermes_url: DEFAULT_HERMES_URL.to_string(),
            api_key: Some(DEFAULT_PYTH_API_KEY.to_string()),
        }
    }
}

impl PythConfig {
    pub fn from_source(src: &impl EnvSource) -> Self {
        Self {
            hermes_url: src
                .get_nonempty("PYTH_HERMES_URL")
                .unwrap_or_else(|| DEFAULT_HERMES_URL.to_string()),
            api_key: src
                .get_nonempty("PYTH_API_KEY")
                .or_else(|| Some(DEFAULT_PYTH_API_KEY.to_string())),
        }
    }

    pub fn is_authenticated(&self) -> bool {
        self.api_key.is_some()
    }
}
