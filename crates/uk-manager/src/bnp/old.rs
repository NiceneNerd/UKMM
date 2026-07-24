use std::{path::Path, str::FromStr};

use anyhow_ext::{bail, Context, Result};
use fs_err as fs;
use roead::{
    aamp::*,
    byml::{map, Byml},
    sarc::Sarc,
    types::*,
};
use serde::Deserialize;
use serde_yaml::Value;
use uk_content::{
    constants::Language,
    message::{Entry, Msyt},
    util::{HashMap, IteratorExt},
};

use super::texts::TextsLog;

fn value_to_byml(value: Value) -> Result<Byml> {
    let by = match value {
        Value::Null => Byml::Null,
        Value::Bool(v) => Byml::Bool(v),
        Value::Number(v) => {
            match v.as_i64() {
                Some(v) => Byml::I32(v as i32),
                None => {
                    v.as_f64()
                        .map(|v| Byml::Float(v as f32))
                        .context("Invalid numeric value")?
                }
            }
        }
        Value::String(v) => Byml::String(v.into()),
        Value::Sequence(seq) => {
            Byml::Array(seq.into_iter().map(value_to_byml).collect::<Result<_>>()?)
        }
        Value::Mapping(map) => {
            map.into_iter()
                .map(|(k, v)| -> Result<(smartstring::alias::String, Byml)> {
                    let k = k.as_str().context("Bad BYML key")?.into();
                    let v = value_to_byml(v)?;
                    Ok((k, v))
                })
                .collect::<Result<Byml>>()?
        }
        Value::Tagged(v) => {
            if v.tag == "u" {
                Byml::U32(v.value.as_u64().context("Invalid u32")? as u32)
            } else {
                bail!("Unsupported BYML type")
            }
        }
    };
    Ok(by)
}

fn param_from_value(value: Value) -> Result<Parameter> {
    let param = match value {
        Value::Null => bail!("AAMP parameters cannot be null"),
        Value::Bool(v) => Parameter::Bool(v),
        Value::Number(v) => {
            if let Some(v) = v.as_f64() {
                Parameter::F32(v as f32)
            } else if let Some(v) = v.as_i64() {
                Parameter::I32(v as i32)
            } else {
                bail!("Invalid number for AAMP parameter")
            }
        }
        Value::String(v) => Parameter::StringRef(v.into()),
        Value::Sequence(_) => bail!("AAMP parameters cannot be untagged arrays"),
        Value::Mapping(_) => bail!("AAMP parameters cannot be maps"),
        Value::Tagged(v) => {
            if v.tag == "u" {
                Parameter::U32(
                    v.value
                        .as_u64()
                        .context("Invalid u32 parameter in deepmerge log")?
                        as u32,
                )
            } else if v.tag == "str32" {
                Parameter::String32(
                    v.value
                        .as_str()
                        .map(FixedSafeString::from)
                        .or_else(|| {
                            v.value
                                .as_i64()
                                .map(|i| FixedSafeString::from(i.to_string()))
                        })
                        .context("Invalid str32 param in deepmerge log")?,
                )
            } else if v.tag == "str64" {
                Parameter::String64(Box::new(
                    v.value
                        .as_str()
                        .context("Invalid str64 param in deepmerge log")?
                        .into(),
                ))
            } else if v.tag == "str256" {
                Parameter::String256(Box::new(
                    v.value
                        .as_str()
                        .context("Invalid str64 param in deepmerge log")?
                        .into(),
                ))
            } else if let Some(seq) = v.value.as_sequence().map(|s| {
                s.iter()
                    .filter_map(|v| v.as_f64().map(|f| f as f32))
                    .collect::<Vec<_>>()
            }) {
                if v.tag == "vec2" {
                    if seq.len() != 2 {
                        bail!("Invalid vec2 param in deepmerge log")
                    }
                    Parameter::Vec2(Vector2f {
                        x: seq[0],
                        y: seq[1],
                    })
                } else if v.tag == "vec3" {
                    if seq.len() != 3 {
                        bail!("Invalid vec3 param in deepmerge log")
                    }
                    Parameter::Vec3(Vector3f {
                        x: seq[0],
                        y: seq[1],
                        z: seq[2],
                    })
                } else if v.tag == "vec4" {
                    if seq.len() != 4 {
                        bail!("Invalid vec4 param in deepmerge log")
                    }
                    Parameter::Vec4(Vector4f {
                        x: seq[0],
                        y: seq[1],
                        z: seq[2],
                        t: seq[3],
                    })
                } else if v.tag == "quat" {
                    if seq.len() != 4 {
                        bail!("Invalid quat param in deepmerge log")
                    }
                    Parameter::Quat(Quat {
                        a: seq[0],
                        b: seq[1],
                        c: seq[2],
                        d: seq[3],
                    })
                } else if v.tag == "color" {
                    if seq.len() != 4 {
                        bail!("Invalid color param in deepmerge log")
                    }
                    Parameter::Color(Color {
                        r: seq[0],
                        g: seq[1],
                        b: seq[2],
                        a: seq[3],
                    })
                } else {
                    bail!("Unsupported sequence param type in deepmerge log")
                }
            } else {
                bail!("Unsupported param type in deepmerge log")
            }
        }
    };
    Ok(param)
}

#[inline]
fn handle_aamp_pair<T>(
    (key, value): (Value, Value),
    from: impl Fn(Value) -> Result<T>,
) -> Option<Result<(Name, T)>> {
    match key.as_str() {
        Some(k) => Some(from(value).map(|v| (Name::from_str(k), v))),
        None => {
            key.as_u64()
                .map(|k| from(value).map(|v| (Name::from(k as u32), v)))
        }
    }
}

fn pobj_from_value(value: Value) -> Result<ParameterObject> {
    let Value::Tagged(value) = value else {
        bail!("Not a parameter object")
    };
    if value.tag != "obj" {
        bail!("Not a parameter object")
    }
    let Value::Mapping(map) = value.value else {
        bail!("Invalid parameter object: not a map")
    };
    let obj = map
        .into_iter()
        .filter_map(|(k, v)| handle_aamp_pair((k, v), param_from_value))
        .collect::<Result<_>>()?;
    Ok(obj)
}

fn plist_from_value(value: Value) -> Result<ParameterList> {
    let Value::Tagged(value) = value else {
        bail!("Not a parameter list")
    };
    if value.tag != "list" {
        bail!("Not a parameter list")
    };
    let Value::Mapping(mut map) = value.value else {
        bail!("Invalid parameter list: not a map")
    };
    let Some(Value::Mapping(lists)) = map.remove("lists") else {
        bail!("Invalid parameter list: missing lists")
    };
    let Some(Value::Mapping(objects)) = map.remove("objects") else {
        bail!("Invalid parameter list: missing objects")
    };
    Ok(ParameterList::new()
        .with_lists(
            lists
                .into_iter()
                .filter_map(|(k, v)| handle_aamp_pair((k, v), plist_from_value))
                .collect::<Result<ParameterListMap>>()?
                .0,
        )
        .with_objects(
            objects
                .into_iter()
                .filter_map(|(k, v)| handle_aamp_pair((k, v), pobj_from_value))
                .collect::<Result<ParameterObjectMap>>()?
                .0,
        ))
}

pub struct Bnp2xConverter<'a> {
    path: &'a Path,
}

impl<'a> Bnp2xConverter<'a> {
    pub fn new(path: &'a Path) -> Self {
        Self { path }
    }

    pub fn convert(&self) -> Result<()> {
        log::info!("Converting old BNP logs…");
        std::thread::scope(|s| -> Result<()> {
            let jobs = [
                s.spawn(|| self.convert_pack_log()),
                s.spawn(|| self.convert_aamp_log()),
                s.spawn(|| self.convert_text_logs()),
                s.spawn(|| self.convert_gamedata_log()),
                s.spawn(|| self.convert_savedata_log()),
                s.spawn(|| self.convert_map_log()),
            ];
            for job in jobs {
                match job.join() {
                    Ok(Err(e)) => anyhow_ext::bail!(e),
                    Ok(Ok(_)) => (),
                    Err(e) => {
                        anyhow::bail!(
                            e.downcast::<String>()
                                .or_else(|e| {
                                    e.downcast::<&'static str>().map(|s| Box::new((*s).into()))
                                })
                                .unwrap_or_else(|_| {
                                    Box::new(
                                        "An unknown error occured, check the log for possible \
                                         details."
                                            .to_string(),
                                    )
                                })
                        )
                    }
                }
            }
            Ok(())
        })?;
        log::info!("Finished converting old BNP logs");
        Ok(())
    }

    fn convert_pack_log(&self) -> Result<()> {
        let packs_path = self.path.join("logs/packs.log");
        if packs_path.exists() {
            log::debug!("Converting old pack log");
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

    fn convert_aamp_log(&self) -> Result<()> {
        let aamp_path = self.path.join("logs/deepmerge.yml");
        if aamp_path.exists() {
            log::debug!("Converting old deepmerge log");
            let merge_log: Value = serde_yaml::from_str(&fs::read_to_string(aamp_path)?)?;
            let Value::Mapping(merge_log) = merge_log else {
                bail!("Invalid deepmerge log")
            };
            let mut new_log = ParameterIO::new();
            let file_table = new_log.param_root.objects.entry("FileTable").or_default();
            for (index, (k, v)) in merge_log.into_iter().named_enumerate("File") {
                let key = k.as_str().context("Invalid deepmerge log entry")?;
                file_table.insert(index, Parameter::StringRef(key.into()));
                new_log.param_root.lists.insert(key, plist_from_value(v)?);
            }
            fs::write(self.path.join("logs/deepmerge.aamp"), new_log.to_binary())?;
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

        #[derive(Debug, Deserialize)]
        struct MsbtEntries {
            #[serde(with = "serde_yaml::with::singleton_map_recursive")]
            entries: HashMap<String, Entry>,
        }

        if !(yaml_logs.is_empty() && sarc_logs.is_empty()) {
            log::debug!("Converting old text log");
            let mut diff: TextsLog = TextsLog::default();
            for yaml_log in yaml_logs {
                let lang: Language = Language::from_str(
                    yaml_log
                        .file_stem()
                        .and_then(|n| n.to_str())
                        .map(|n| &n[6..])
                        .context("Bad file language")?,
                )?;
                let log: HashMap<String, MsbtEntries> =
                    serde_yaml::from_str(&fs::read_to_string(yaml_log)?)?;
                diff.insert(lang, log.into_iter().map(|(k, v)| (k, v.entries)).collect());
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

    fn convert_gamedata_log(&self) -> Result<()> {
        let gdata_log = self.path.join("logs/gamedata.yml");
        if gdata_log.exists() {
            log::debug!("Converting old gamedata log");
            let log = Byml::from_text(fs::read_to_string(&gdata_log)?)?.into_map()?;
            let new_log = log
                .into_iter()
                .map(|(data_type, diff)| {
                    (data_type, map!("add" => diff, "del" => Byml::Array(vec![])))
                })
                .collect::<Byml>();
            fs::write(gdata_log, new_log.to_text())?;
        }
        Ok(())
    }

    fn convert_savedata_log(&self) -> Result<()> {
        let sdata_log = self.path.join("logs/savedata.yml");
        if sdata_log.exists() {
            log::debug!("Converting old savedata log");
            let log = Byml::from_text(fs::read_to_string(&sdata_log)?)?;
            fs::write(
                sdata_log,
                map!("add" => log, "del" => Byml::Array(vec![])).to_text(),
            )?;
        }
        Ok(())
    }

    fn convert_map_log(&self) -> Result<()> {
        let map_log = self.path.join("logs/map.yml");
        if map_log.exists() {
            log::debug!("Converting old map log");
            let Value::Mapping(log) = serde_yaml::from_str(&fs::read_to_string(&map_log)?)? else {
                bail!("Invalid map log")
            };
            let new_log = log
                .into_iter()
                .map(|(unit, diff)| -> Result<(String, Byml)> {
                    let unit: String = unit.as_str().context("Invalid map unit")?.into();
                    let Value::Mapping(mut diff) = diff else {
                        bail!("Bad map log")
                    };
                    let new_diff = map!(
                        "Objs" => map!(
                            "add" => diff.remove("add")
                                .map(value_to_byml)
                                .transpose()?
                                .unwrap_or_default(),
                            "del" => diff.remove("del")
                                .map(value_to_byml)
                                .transpose()?
                                .unwrap_or_default(),
                            "mod" => diff.remove("mod")
                                .map(|mod_diff| -> Result<Byml> {
                                    let Value::Mapping(mod_diff) = mod_diff else {
                                        bail!("Invalid map diff entry")
                                    };
                                    mod_diff.into_iter().map(|(k, v)| -> Result<(String, Byml)> {
                                        let k = k.as_u64().context("Invalid hash ID for map diff entry")?;
                                        let v = value_to_byml(v)?;
                                        Ok((k.to_string(), v))
                                    }).collect::<Result<_>>()
                                })
                                .transpose()?
                                .unwrap_or_default()
                        ),
                        "Rails" => map!(
                            "add" => Byml::Array(vec![]),
                            "del" => Byml::Array(vec![]),
                            "mod" => Byml::Map(Default::default())
                        )
                    );
                    Ok((unit, new_diff))
                })
                .collect::<Result<Byml>>()?;
            fs::write(map_log, new_log.to_text())?;
        }
        Ok(())
    }
}
