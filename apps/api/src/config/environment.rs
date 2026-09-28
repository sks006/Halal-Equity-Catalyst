use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    Development,
    Testnet,
    Mainnet,
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Development => write!(f, "development"),
            Self::Testnet => write!(f, "testnet"),
            Self::Mainnet => write!(f, "mainnet"),
        }
    }
}

impl Environment {
    pub fn from_str_loose(s: &str) -> Self {
        match s.to_lowercase().trim() {
            "mainnet" | "mainnet-beta" | "production" | "prod" => Self::Mainnet,
            "dev" | "local" | "localnet" | "development" => Self::Development,
            _ => Self::Testnet,
        }
    }

    pub fn is_mainnet(self) -> bool {
        matches!(self, Self::Mainnet)
    }

    pub fn requires_pyth_api_key(self) -> bool {
        self.is_mainnet()
    }
}
