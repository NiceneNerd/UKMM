use anyhow::Result;
use rustc_hash::FxHashMap;

use super::BnpConverter;

type DropTables = FxHashMap<String, DropTable>;
type DropDiff = FxHashMap<String, DropTables>;

struct DropTable {
    repeat_num_min: i32,
    repeat_num_max: i32,
    approach_type: i32,
    occurence_speed_type: i32,
    items: FxHashMap<String, f32>,
}

impl BnpConverter<'_> {
    pub fn handle_drops(&self) -> Result<()> {
        let drops_path = self.path.join("logs/drops.json");
        Ok(())
    }
}
