use anyhow::Result;
use fs_err as fs;
use rayon::prelude::*;
use roead::aamp::{Parameter, ParameterIO, ParameterObject};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use super::BnpConverter;

type DropTables = FxHashMap<String, DropTable>;
type DropDiff = FxHashMap<String, DropTables>;

#[derive(Debug, Serialize, Deserialize)]
struct DropTable {
    repeat_num_min: i32,
    repeat_num_max: i32,
    approach_type: i32,
    occurrence_speed_type: i32,
    items: FxHashMap<String, f32>,
}

impl BnpConverter<'_> {
    pub fn handle_drops(&self) -> Result<()> {
        let drops_path = self.path.join("logs/drops.json");
        if drops_path.exists() {
            let drops: DropDiff = serde_json::from_str(&fs::read_to_string(drops_path)?)?;
            drops
                .into_par_iter()
                .try_for_each(|(path, tables)| -> Result<()> {
                    let pio = ParameterIO::new()
                        .with_object(
                            "Header",
                            ParameterObject::new()
                                .with_parameter("TableNum", Parameter::Int(tables.len() as i32))
                                .with_parameters(tables.keys().enumerate().map(|(i, name)| {
                                    (
                                        format!("Table{:02}", i + 1),
                                        Parameter::String64(Box::new(name.as_str().into())),
                                    )
                                })),
                        )
                        .with_objects(tables.into_iter().map(|(name, table)| {
                            (
                                name,
                                ParameterObject::new()
                                    .with_parameter(
                                        "RepeatNumMin",
                                        Parameter::Int(table.repeat_num_min),
                                    )
                                    .with_parameter(
                                        "RepeatNumMax",
                                        Parameter::Int(table.repeat_num_max),
                                    )
                                    .with_parameter(
                                        "ApproachType",
                                        Parameter::Int(table.approach_type),
                                    )
                                    .with_parameter(
                                        "OccurrenceSpeedType",
                                        Parameter::Int(table.occurrence_speed_type),
                                    )
                                    .with_parameter(
                                        "ColumnNum",
                                        Parameter::Int(table.items.len() as i32),
                                    )
                                    .with_parameters(table.items.into_iter().enumerate().flat_map(
                                        |(i, (name, prob))| {
                                            let i = i + 1;
                                            [
                                                (
                                                    format!("ItemName{i:02}"),
                                                    Parameter::String64(Box::new(
                                                        name.as_str().into(),
                                                    )),
                                                ),
                                                (
                                                    format!("ItemProbability{i:02}"),
                                                    Parameter::F32(prob),
                                                ),
                                            ]
                                            .into_iter()
                                        },
                                    )),
                            )
                        }));
                    self.inject_into_sarc(
                        self.trim_prefixes(&path),
                        pio.to_binary(),
                        path.starts_with(self.aoc),
                    )?;
                    Ok(())
                })?;
        }
        Ok(())
    }
}
