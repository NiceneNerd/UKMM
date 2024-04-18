use once_cell::sync::Lazy;
use xxhash_rust::xxh3::xxh3_64;

use crate::prelude::Endian;

static HASH_BIN_U: &[u8] = include_bytes!("../data/hashes_u.bin");
static HASH_BIN_NX: &[u8] = include_bytes!("../data/hashes_nx.bin");

pub struct ROMHashTable(std::collections::BTreeMap<u64, u64>);

impl ROMHashTable {
    pub fn new(endian: Endian) -> Self {
        match endian {
            Endian::Big => Self::parse(HASH_BIN_U),
            Endian::Little => Self::parse(HASH_BIN_NX),
        }
    }

    fn parse(data: &[u8]) -> Self {
        // We know for sure this can't fail on any supported system because we made and included the
        // data ourselves.
        let data = unsafe { zstd::decode_all(std::io::Cursor::new(data)).unwrap_unchecked() };
        let count = (data.len() as f64 / 2. / 8.) as usize;
        let value_offset = (data.len() as f64 / 2.) as usize;
        Self(
            (0..count)
                .into_iter()
                .map(|i| {
                    // As above.
                    unsafe {
                        let file = u64::from_be_bytes(data[i * 8..(i * 8 + 8)].try_into().unwrap());
                        let hash = u64::from_be_bytes(
                            data[i * 8 + value_offset..(i * 8 + 8 + value_offset)]
                                .try_into()
                                .unwrap_unchecked(),
                        );
                        (file, hash)
                    }
                })
                .collect(),
        )
    }

    pub fn is_modified(&self, file: impl AsRef<str>, data: impl AsRef<[u8]>) -> bool {
        let data = roead::yaz0::decompress_if(data.as_ref());
        let file = xxh3_64(file.as_ref().as_bytes());
        let hash = xxh3_64(data.as_ref());
        self.0.get(&file) != Some(&hash)
    }

    pub fn contains(&self, file: impl AsRef<str>) -> bool {
        let file = xxh3_64(file.as_ref().as_bytes());
        self.0.contains_key(&file)
    }
}

static HASH_TABLE_U: Lazy<ROMHashTable> = Lazy::new(|| ROMHashTable::new(Endian::Big));
static HASH_TABLE_NX: Lazy<ROMHashTable> = Lazy::new(|| ROMHashTable::new(Endian::Little));

#[inline]
pub fn get_hash_table(endian: Endian) -> &'static ROMHashTable {
    match endian {
        Endian::Big => &HASH_TABLE_U,
        Endian::Little => &HASH_TABLE_NX,
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    #[test]
    fn wiiu_hash_table() {
        let table = super::ROMHashTable::new(crate::prelude::Endian::Little);
        let stock_data = std::fs::read("test/WorldMgr/normal.bwinfo").unwrap();
        assert!(!table.is_modified("WorldMgr/normal.bwinfo", stock_data));
        let modded_data = std::fs::read("test/WorldMgr/normal.mod.bwinfo").unwrap();
        assert!(table.is_modified("WorldMgr/normal.bwinfo", modded_data));
    }

    #[test]
    fn convert_bcml_hash_tables() {
        for platform in ["wiiu", "switch"] {
            let file = std::path::Path::new(r"E:\Downloads")
                .join(platform)
                .with_extension("sjson");
            let data = std::fs::read(file).unwrap();
            let hashes: rustc_hash::FxHashMap<String, rustc_hash::FxHashSet<u32>> =
                serde_json::from_slice(&roead::yaz0::decompress(data).unwrap()).unwrap();
            let new_data =
                zstd::encode_all(minicbor_ser::to_vec(&hashes).unwrap().as_slice(), 0).unwrap();
            std::fs::write(
                std::path::Path::new("data/hashes")
                    .join(platform)
                    .with_extension("bin.zst"),
                new_data,
            )
            .unwrap();
        }
    }
}
