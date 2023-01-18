use anyhow::{Context, Result};
use fs_err as fs;
use rustc_hash::FxHashMap;
use uk_content::message::*;

use super::BnpConverter;

type TextsLog = FxHashMap<String, FxHashMap<String, FxHashMap<String, Entry>>>;

impl BnpConverter {
    pub fn handle_texts(&self) -> Result<()> {
        let texts_path = self.path.join("logs/texts.json");
        if texts_path.exists() {
            let diff: TextsLog = serde_json::from_str(&fs::read_to_string(texts_path)?)?;
        }
        Ok(())
    }
}
