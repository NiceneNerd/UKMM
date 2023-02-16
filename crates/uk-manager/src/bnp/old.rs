use std::{path::PathBuf, str::FromStr};

use anyhow::{Context, Result};
use fs_err as fs;
use roead::{aamp::*, byml::Byml, sarc::Sarc};
use serde_yaml::Value;
use uk_content::{
    message::{Entry, Msyt},
    util::HashMap,
};

use super::texts::TextsLog;
use crate::settings::Language;

fn byml_from_value(value: Value) -> Byml {
    match value {
        Value::Null => Byml::Null,
        Value::Bool(bool) => Byml::Bool(bool),
        Value::Number(num) => Byml::I32(num.as_i64().unwrap_or_default() as i32),
        Value::String(string) => Byml::String(string.into()),
        Value::Sequence(seq) => seq.into_iter().map(byml_from_value).collect(),
        Value::Mapping(map) => {
            map.into_iter()
                .filter_map(|(k, v)| -> Option<(String, Byml)> {
                    k.as_str().map(|k| (k.into(), byml_from_value(v)))
                })
                .collect()
        }
        Value::Tagged(value) => {
            if value.tag == "u" {
                Byml::U32(value.value.as_u64().unwrap_or_default() as u32)
            } else {
                todo!()
            }
        }
    }
}

struct Bnp2xConverter {
    path: PathBuf,
}

impl Bnp2xConverter {
    fn convert_pack_log(&self) -> Result<()> {
        let packs_path = self.path.join("logs/packs.log");
        if packs_path.exists() {
            let text = fs::read_to_string(packs_path)?;
            let json: HashMap<String, String> = text
                .lines()
                .skip(1)
                .filter_map(|line| {
                    let mut iter = line.split(',').map(|p| p.replace('\\', "/"));
                    let canon = iter.next();
                    let path = iter.next();
                    canon.and_then(|c| path.map(|p| (c, p)))
                })
                .collect();
            fs::write(
                self.path.join("logs/packs.json"),
                serde_json::to_string_pretty(&json)?,
            )?;
        }
        Ok(())
    }

    fn convert_text_logs(&self) -> Result<()> {
        use smartstring::alias::String;
        let yaml_logs = jwalk::WalkDir::new(self.path.join("logs"))
            .into_iter()
            .filter_map(|e| {
                e.ok().and_then(|e| {
                    e.file_name()
                        .to_str()
                        .and_then(|n| n.starts_with("texts_").then(|| e.path()))
                })
            })
            .collect::<Vec<_>>();
        let sarc_logs = jwalk::WalkDir::new(self.path.join("logs"))
            .into_iter()
            .filter_map(|e| {
                e.ok().and_then(|e| {
                    e.file_name()
                        .to_str()
                        .and_then(|n| n.starts_with("newtexts_").then(|| e.path()))
                })
            })
            .collect::<Vec<_>>();
        if !(yaml_logs.is_empty() && sarc_logs.is_empty()) {
            let mut diff: TextsLog = TextsLog::default();
            for yaml_log in yaml_logs {
                let lang: Language = Language::from_str(
                    yaml_log
                        .file_stem()
                        .and_then(|n| n.to_str())
                        .map(|n| &n[6..])
                        .context("Bad file language")?,
                )?;
                let log: HashMap<String, HashMap<String, Entry>> =
                    serde_yaml::from_str(&fs::read_to_string(yaml_log)?)?;
                diff.insert(lang, log);
            }
            for sarc_log in sarc_logs {
                let lang: Language = Language::from_str(
                    sarc_log
                        .file_stem()
                        .and_then(|n| n.to_str())
                        .map(|n| &n[6..])
                        .context("Bad file language")?,
                )?;
                let lang_diff = diff.entry(lang).or_default();
                let sarc = Sarc::new(fs::read(sarc_log)?)?;
                for file in sarc.files().filter(|f| f.name.is_some()) {
                    let msyt = Msyt::from_msbt_bytes(file.data)?;
                    lang_diff.insert(
                        file.unwrap_name().into(),
                        msyt.entries
                            .into_iter()
                            .map(|(k, v)| (k.into(), v))
                            .collect(),
                    );
                }
            }
            fs::write(
                self.path.join("logs/texts.json"),
                serde_json::to_string(&diff)?,
            )?;
        }
        Ok(())
    }
}
