use crate::prelude::Endian;
use once_cell::sync::Lazy;
use xxhash_rust::xxh3::xxh3_64;

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
        let data = zstd::decode_all(std::io::Cursor::new(data)).unwrap();
        let count = (data.len() as f64 / 2. / 8.) as usize;
        let value_offset = (data.len() as f64 / 2.) as usize;
        Self(
            (0..count)
                .into_iter()
                .map(|i| {
                    let file = u64::from_be_bytes(data[i * 8..(i * 8 + 8)].try_into().unwrap());
                    let hash = u64::from_be_bytes(
                        data[i * 8 + value_offset..(i * 8 + 8 + value_offset)]
                            .try_into()
                            .unwrap(),
                    );
                    (file, hash)
                })
                .collect(),
        )
    }

    pub fn is_modified(&self, file: impl AsRef<str>, data: impl AsRef<[u8]>) -> bool {
        match roead::yaz0::decompress_if(data.as_ref()) {
            Ok(data) => {
                let file = xxh3_64(file.as_ref().as_bytes());
                let hash = xxh3_64(data.as_ref());
                self.0.get(&file) != Some(&hash)
            }
            Err(_) => true,
        }
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

#[cfg(test)]
mod tests {
    #[test]
    fn wiiu_hash_table() {
        let table = super::ROMHashTable::new(crate::prelude::Endian::Little);
        let stock_data = std::fs::read("test/WorldMgr/normal.bwinfo").unwrap();
        assert!(!table.is_modified("WorldMgr/normal.bwinfo", &stock_data));
        let modded_data = std::fs::read("test/WorldMgr/normal.mod.bwinfo").unwrap();
        assert!(table.is_modified("WorldMgr/normal.bwinfo", &modded_data));
    }
}
