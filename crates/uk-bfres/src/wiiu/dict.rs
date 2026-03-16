//! Wii U BFRES dictionary (Index Group / Patricia Trie) parser.
//!
//! Wii U BFRES uses "Index Groups" — a Patricia trie stored as a flat array.
//! Each node has a search value, left/right child indices, a name, and a data offset.
//!
//! Layout (big-endian):
//! 0x00  4  Total size of the dictionary block
//! 0x04  4  Number of entries (excluding root sentinel)
//!
//! Per entry (including root at index 0):
//! 0x00  4  Search value (bit reference for Patricia trie)
//! 0x04  2  Left child index
//! 0x06  2  Right child index
//! 0x08  4  Name offset (relative to this field)
//! 0x0C  4  Data offset (relative to this field)

use crate::binary::BigEndianReader;
use crate::error::Result;

/// A single entry from a Wii U BFRES dictionary.
#[derive(Debug, Clone)]
pub struct DictEntry {
    pub name: String,
    /// Absolute offset to the data for this entry.
    pub data_offset: usize,
}

/// Parse a Wii U Index Group dictionary at the given absolute offset.
/// Returns the entries (excluding the root sentinel node).
pub fn parse_dict(reader: &mut BigEndianReader, offset: usize) -> Result<Vec<DictEntry>> {
    reader.seek(offset);

    let _total_size = reader.read_u32()?;
    let num_entries = reader.read_i32()? as usize;

    let mut entries = Vec::with_capacity(num_entries);

    // Read root sentinel (index 0) — skip it
    let _root_search = reader.read_u32()?;
    let _root_left = reader.read_u16()?;
    let _root_right = reader.read_u16()?;
    let _root_name_field_pos = reader.pos();
    let _root_name_rel = reader.read_u32()?;
    let _root_data_field_pos = reader.pos();
    let _root_data_rel = reader.read_u32()?;

    // Read actual entries (index 1..=num_entries)
    for _ in 0..num_entries {
        let _search_value = reader.read_u32()?;
        let _left_index = reader.read_u16()?;
        let _right_index = reader.read_u16()?;

        let name_field_pos = reader.pos();
        let name_rel = reader.read_u32()?;
        let name = if name_rel > 0 {
            let name_abs = (name_field_pos as u32).wrapping_add(name_rel) as usize;
            reader.read_string_at(name_abs).unwrap_or_default()
        } else {
            String::new()
        };

        let data_field_pos = reader.pos();
        let data_rel = reader.read_u32()?;
        let data_offset = if data_rel > 0 {
            (data_field_pos as u32).wrapping_add(data_rel) as usize
        } else {
            0
        };

        entries.push(DictEntry { name, data_offset });
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wiiu::header;

    #[test]
    fn parse_model_dict() {
        let data = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/Animal_Boar_Big.wiiu.bfres"
        ))
        .expect("fixture file should exist");

        let mut reader = BigEndianReader::new(&data);
        let hdr = header::parse_header(&mut reader).expect("header should parse");

        if hdr.model_dict_offset > 0 {
            let entries = parse_dict(&mut reader, hdr.model_dict_offset as usize)
                .expect("model dict should parse");
            assert_eq!(entries.len(), hdr.model_count as usize);
            for entry in &entries {
                assert!(!entry.name.is_empty(), "model name should not be empty");
                assert!(entry.data_offset > 0, "model data offset should be nonzero");
                log::info!("Model: {} at {:#x}", entry.name, entry.data_offset);
            }
        }
    }
}
