use anyhow_ext::{Context, Result};
use fs_err as fs;
use rayon::prelude::*;
use roead::aamp::{Parameter, ParameterIO, ParameterObject};
use rustc_hash::FxHashMap;
use serde::Deserialize;

use super::BnpConverter;

type DropTables = FxHashMap<String, DropTable>;
type DropDiff = FxHashMap<String, DropTables>;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ProbabilityValue {
    Underride(String),
    Value(f32),
}

#[derive(Debug, Deserialize)]
struct DropTable {
    repeat_num_min: i32,
    repeat_num_max: i32,
    approach_type: i32,
    occurrence_speed_type: i32,
    items: FxHashMap<String, ProbabilityValue>,
}

const UNDERRIDE: &str = "UNDERRIDE_CONST";

impl BnpConverter {
    pub fn handle_drops(&self) -> Result<()> {
        let drops_path = self.current_root.join("logs/drops.json");
        if drops_path.exists() {
            log::debug!("Processing drops log");
            let text = fs::read_to_string(drops_path)?;
            let do_refs = text.contains(UNDERRIDE);
            let drops: DropDiff = serde_json::from_str(&text)?;
            drops
                .into_par_iter()
                .try_for_each(|(path, tables)| -> Result<()> {
                    let ref_drop = if do_refs {
                        self.dump
                            .get_data(path.split("//").last().context("Bad drop diff")?)
                            .ok()
                            .and_then(|res| {
                                res.as_mergeable().and_then(|m| {
                                    match m {
                                        uk_content::resource::MergeableResource::DropTable(d) => {
                                            Some(d.clone())
                                        }
                                        _ => None,
                                    }
                                })
                            })
                    } else {
                        None
                    };
                    let pio = ParameterIO::new()
                        .with_object(
                            "Header",
                            ParameterObject::new()
                                .with_parameter("TableNum", Parameter::I32(tables.len() as i32))
                                .with_parameters(tables.keys().enumerate().map(|(i, name)| {
                                    (
                                        format!("Table{:02}", i + 1),
                                        Parameter::String64(Box::new(name.as_str().into())),
                                    )
                                })),
                        )
                        .with_objects(tables.into_iter().map(|(table_name, table)| {
                            (
                                table_name.clone(),
                                ParameterObject::new()
                                    .with_parameter(
                                        "RepeatNumMin",
                                        Parameter::I32(table.repeat_num_min),
                                    )
                                    .with_parameter(
                                        "RepeatNumMax",
                                        Parameter::I32(table.repeat_num_max),
                                    )
                                    .with_parameter(
                                        "ApproachType",
                                        Parameter::I32(table.approach_type),
                                    )
                                    .with_parameter(
                                        "OccurrenceSpeedType",
                                        Parameter::I32(table.occurrence_speed_type),
                                    )
                                    .with_parameter(
                                        "ColumnNum",
                                        Parameter::I32(table.items.len() as i32),
                                    )
                                    .with_parameters(table.items.into_iter().enumerate().flat_map(
                                        |(i, (item_name, prob))| {
                                            let i = i + 1;
                                            let prob = match prob {
                                                ProbabilityValue::Underride(_) => {
                                                    let underride = ref_drop
                                                        .as_ref()
                                                        .and_then(|r| r.0.get(table_name.as_str()))
                                                        .and_then(|table| {
                                                            let p = table.0.iter().position(
                                                                |(_, v)| {
                                                                    v.as_str()
                                                                        .map(|v| v == item_name)
                                                                        .unwrap_or(false)
                                                                },
                                                            );
                                                            p.and_then(|i| {
                                                                table.0.values().nth(i + 1)
                                                            })
                                                            .and_then(|v| v.as_f32().ok())
                                                        });
                                                    match underride {
                                                        Some(v) => v,
                                                        None => return vec![],
                                                    }
                                                }
                                                ProbabilityValue::Value(v) => v,
                                            };
                                            vec![
                                                (
                                                    format!("ItemName{i:02}"),
                                                    Parameter::String64(Box::new(
                                                        item_name.as_str().into(),
                                                    )),
                                                ),
                                                (
                                                    format!("ItemProbability{i:02}"),
                                                    Parameter::F32(prob),
                                                ),
                                            ]
                                        },
                                    )),
                            )
                        }));
                    self.inject_into_sarc(
                        self.trim_prefixes(&path),
                        pio.to_binary(),
                        path.starts_with(self.aoc),
                    )
                    .with_context(|| format!("Failed to save drop table from diff to {path}"))?;
                    Ok(())
                })?;
        }
        Ok(())
    }
}
