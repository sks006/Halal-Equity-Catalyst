use std::collections::HashMap;
use std::env;

/// Abstract source of string configuration values.
///
/// Production uses `ProcessEnv`; tests use `MapEnv`. Nothing below
/// this trait calls `env::var` directly, which is the DIP win.
pub trait EnvSource {
    fn get(&self, key: &str) -> Option<String>;

    fn get_nonempty(&self, key: &str) -> Option<String> {
        self.get(key)
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
    }

    fn get_or(&self, key: &str, default: String) -> String {
        self.get_nonempty(key).unwrap_or(default)
    }

    fn get_parsed<T: std::str::FromStr>(&self, key: &str) -> Option<T> {
        self.get_nonempty(key).and_then(|v| v.parse().ok())
    }

    fn get_first(&self, keys: &[&str]) -> Option<String> {
        keys.iter().find_map(|k| self.get_nonempty(k))
    }
}

pub struct ProcessEnv;

impl EnvSource for ProcessEnv {
    fn get(&self, key: &str) -> Option<String> {
        env::var(key).ok()
    }
}

/// In-memory source for tests. No global state, no `serial_test` needed.
#[derive(Default, Clone)]
pub struct MapEnv(pub HashMap<String, String>);

impl MapEnv {
    pub fn new() -> Self {
        Self(HashMap::new())
    }
    pub fn with(mut self, k: impl Into<String>, v: impl Into<String>) -> Self {
        self.0.insert(k.into(), v.into());
        self
    }
}

impl EnvSource for MapEnv {
    fn get(&self, key: &str) -> Option<String> {
        self.0.get(key).cloned()
    }
}
