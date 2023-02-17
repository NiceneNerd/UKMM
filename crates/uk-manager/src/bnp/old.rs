use std::{path::PathBuf, str::FromStr};

use anyhow::{bail, Context, Result};
use fs_err as fs;
use roead::{aamp::*, byml::Byml, sarc::Sarc, types::*};
use serde_yaml::{value::TaggedValue, Value};
use uk_content::{
    message::{Entry, Msyt},
    util::HashMap,
};

use super::texts::TextsLog;
use crate::settings::Language;

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
                Parameter::U32(v.value.as_u64().context("Invalid u32 parameter")? as u32)
            } else if v.tag == "str32" {
                Parameter::String32(v.value.as_str().context("Invalid str32 param")?.into())
            } else if v.tag == "str64" {
                Parameter::String64(Box::new(
                    v.value.as_str().context("Invalid str64 param")?.into(),
                ))
            } else if v.tag == "str256" {
                Parameter::String256(Box::new(
                    v.value.as_str().context("Invalid str64 param")?.into(),
                ))
            } else if let Some(seq) = v.value.as_sequence().map(|s| {
                s.iter()
                    .filter_map(|v| v.as_f64().map(|f| f as f32))
                    .collect::<Vec<_>>()
            }) {
                if v.tag == "vec2" {
                    if seq.len() != 2 {
                        bail!("Invalid vec2 param")
                    }
                    Parameter::Vec2(Vector2f {
                        x: seq[0],
                        y: seq[1],
                    })
                } else if v.tag == "vec3" {
                    if seq.len() != 3 {
                        bail!("Invalid vec3 param")
                    }
                    Parameter::Vec3(Vector3f {
                        x: seq[0],
                        y: seq[1],
                        z: seq[2],
                    })
                } else if v.tag == "vec4" {
                    if seq.len() != 4 {
                        bail!("Invalid vec4 param")
                    }
                    Parameter::Vec4(Vector4f {
                        x: seq[0],
                        y: seq[1],
                        z: seq[2],
                        t: seq[3],
                    })
                } else if v.tag == "quat" {
                    if seq.len() != 4 {
                        bail!("Invalid quat param")
                    }
                    Parameter::Quat(Quat {
                        a: seq[0],
                        b: seq[1],
                        c: seq[2],
                        d: seq[3],
                    })
                } else if v.tag == "color" {
                    if seq.len() != 4 {
                        bail!("Invalid color param")
                    }
                    Parameter::Color(Color {
                        r: seq[0],
                        g: seq[1],
                        b: seq[2],
                        a: seq[3],
                    })
                } else {
                    bail!("Unsupported sequence param type")
                }
            } else {
                bail!("Unsupported param type")
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
    let Value::Mapping(map) = value
        .value else {
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

    fn convert_aamp_log(&self) -> Result<()> {
        let aamp_path = self.path.join("logs/deepmerge.yml");
        if aamp_path.exists() {
            let merge_log: Value = serde_yaml::from_str(&fs::read_to_string(aamp_path)?)?;
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
