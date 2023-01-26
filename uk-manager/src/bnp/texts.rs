use std::{fmt::format, sync::Arc};

use anyhow::{Context, Result};
use fs_err as fs;
use roead::sarc::SarcWriter;
use rustc_hash::FxHashMap;
use smartstring::alias::String;
use uk_content::{message::*, prelude::Resource, resource::MergeableResource};

use super::BnpConverter;
use crate::settings::Language;

type TextsLog = FxHashMap<Language, FxHashMap<String, FxHashMap<String, Entry>>>;

impl BnpConverter {
    pub fn handle_texts(&self) -> Result<()> {
        let texts_path = self.path.join("logs/texts.json");
        if texts_path.exists() {
            let mut diff: TextsLog = serde_json::from_str(&fs::read_to_string(texts_path)?)?;
            let langs = diff.keys().copied().collect::<Vec<_>>();
            let lang = self.game_lang.nearest(&langs);
            let diff = unsafe { diff.remove(lang).unwrap_unchecked() };
            let base = self.dump.get_from_sarc(
                &format!("Message/Msg_{}.product.sarc", self.game_lang),
                &format!(
                    "Pack/Bootup_{}.pack//Message/Msg_{}.product.ssarc",
                    self.game_lang, self.game_lang,
                ),
            )?;
            if let Some(MergeableResource::MessagePack(texts)) = (*base).clone().take_mergeable() {
                let mut texts = *texts;
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
                    .path
                    .join(self.content)
                    .join(format!("Pack/Bootup_{}.pack", self.game_lang));
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