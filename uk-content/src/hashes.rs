static HASH_BIN: &[u8] = include_bytes!("../data/hashes_u.bin");

pub struct ROMHashTable(std::collections::BTreeMap<u64, u64>);

impl ROMHashTable {
    pub fn new() -> Self {
        todo!()
    }
}
