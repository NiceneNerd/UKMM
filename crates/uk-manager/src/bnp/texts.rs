use anyhow_ext::{Context, Result};
use fs_err as fs;
use roead::sarc::SarcWriter;
use rustc_hash::FxHashMap;
use smartstring::alias::String;
use uk_content::{constants::Language, message::*, prelude::Resource};

use super::BnpConverter;

pub(crate) type TextsLog = FxHashMap<Language, FxHashMap<String, FxHashMap<String, Entry>>>;

impl BnpConverter {
    pub fn handle_texts(&self) -> Result<()> {
        let texts_path = self.current_root.join("logs/texts.json");
        if texts_path.exists() {
            log::debug!("Processing texts log");
            let mut diff: TextsLog = serde_json::from_str(&fs::read_to_string(texts_path)?)?;
            if diff.is_empty() {
                log::debug!("Empty text diff, moving on");
                return Ok(());
            }
            let langs = diff.keys().copied().collect::<Vec<_>>();
            let lang = self.game_lang.nearest(&langs);
            let diff = diff.remove(lang).with_context(|| {
                format!(
                    "No match for {lang} in diff, which is weird. Options: {:?}",
                    langs
                )
            })?;
            let base = self.get_from_master_sarc(&format!(
                "Pack/Bootup_{}.pack//Message/Msg_{}.product.ssarc",
                self.game_lang, self.game_lang,
            )).expect("Your language in UKMM's settings should be a language your dump has.");
            if let Ok(mut texts) = MessagePack::from_binary(base) {
                for (file, diff) in diff {
                    let msyt = texts
                        .0
                        .entry(file.trim_end_matches(".msyt").into())
                        .or_insert_with(|| {
                            Msyt {
                                entries: Default::default(),
                                msbt:    MsbtInfo {
                                    group_count: diff.len() as u32,
                                    atr1_unknown: Some(if file.contains("EventFlowMsg") {
                                        0
                                    } else {
                                        4
                                    }),
                                    ato1: None,
                                    tsy1: None,
                                    nli1: None,
                                },
                            }
                        });
                    msyt.entries
                        .extend(diff.into_iter().map(|(k, v)| (k.into(), v)));
                }
                let out = self
                    .current_root
                    .join(self.content)
                    .join(self.game_lang.bootup_path().as_str());
                out.parent().iter().try_for_each(fs::create_dir_all)?;
                let mut sarc = SarcWriter::new(self.platform.into()).with_file(
                    format!("Message/Msg_{}.product.ssarc", self.game_lang),
                    roead::yaz0::compress(texts.into_binary(self.platform.into())),
                );
                fs::write(out, sarc.to_binary())?;
            }
        }
        Ok(())
    }
}
