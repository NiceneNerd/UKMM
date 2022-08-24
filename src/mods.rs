use serde::{Deserialize, Serialize};
use uk_mod::{Manifest, Meta};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mod {
    pub meta: Meta,
    pub manifest: Manifest,
    pub load_order: usize,
    pub enabled: bool,
}
