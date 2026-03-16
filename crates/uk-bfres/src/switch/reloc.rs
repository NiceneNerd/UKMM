//! Relocation table writer for Switch BFRES.
//!
//! The relocation table (`_RLT`) tells the Switch loader where all pointer
//! fields are located in the file so it can fix them up at load time.
//!
//! # Binary layout
//!
//! ```text
//! -- Header (16 bytes) --
//! +0x00  [u8; 4]  magic "_RLT"
//! +0x04  u32      position (byte offset of this table in the file)
//! +0x08  u32      num_sections
//! +0x0C  u32      padding (0)
//!
//! -- Section descriptors (24 bytes each, repeated num_sections times) --
//! +0x00  i64      pointer (base pointer / offset to section data; 0 for unused)
//! +0x08  u32      position (byte offset where the section starts)
//! +0x0C  u32      size (byte length of the section)
//! +0x10  u32      entry_index (index of the first relocation entry for this section)
//! +0x14  u32      num_entries (number of relocation entries in this section)
//!
//! -- Relocation entries (8 bytes each) --
//! +0x00  u32      position (byte offset of the pointer field in the file)
//! +0x04  u16      struct_count
//! +0x06  u8       offset_count (number of consecutive pointer fields)
//! +0x07  u8       padding_count (non-pointer fields between structs)
//! ```
//!
//! BotW Switch BFRES files use 5 sections:
//!
//! | Index | Purpose                          |
//! |-------|----------------------------------|
//! | 0     | Main data (headers, strings, dicts) |
//! | 1     | Buffer info                      |
//! | 2     | Vertex / index buffer data       |
//! | 3     | Memory pool                      |
//! | 4     | External files                   |

use crate::binary::LittleEndianWriter;

/// Number of relocation sections in a BotW Switch BFRES file.
const NUM_SECTIONS: usize = 5;

/// A single relocation entry describing where pointer fields live in the file.
#[derive(Debug, Clone)]
struct RelocEntry {
    /// Byte offset of the first pointer field in this entry.
    position: u32,
    /// Number of structs that follow this pattern.
    struct_count: u16,
    /// Number of consecutive 64-bit pointer fields at this position.
    offset_count: u8,
    /// Number of non-pointer (padding) fields between structs.
    padding_count: u8,
}

/// Builder for the Switch BFRES relocation table (`_RLT` block).
///
/// Register pointer locations with [`add`](RelocationTable::add), then call
/// [`write`](RelocationTable::write) to serialize the complete table.
pub struct RelocationTable {
    /// One entry list per section (indices 0..4).
    entries: [Vec<RelocEntry>; NUM_SECTIONS],
}

impl RelocationTable {
    /// Create a new, empty relocation table.
    pub fn new() -> Self {
        Self {
            entries: [
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ],
        }
    }

    /// Register a relocation entry.
    ///
    /// # Parameters
    ///
    /// - `section`: section index (0..4).
    /// - `position`: byte offset in the file where the pointer field starts.
    /// - `offset_count`: number of consecutive 64-bit pointer fields at `position`.
    /// - `struct_count`: how many structs follow this pattern.
    /// - `padding_count`: number of non-pointer (8-byte) fields between repeated structs.
    ///
    /// # Panics
    ///
    /// Panics if `section >= NUM_SECTIONS`.
    pub fn add(
        &mut self,
        section: usize,
        position: u32,
        offset_count: u8,
        struct_count: u16,
        padding_count: u8,
    ) {
        assert!(
            section < NUM_SECTIONS,
            "RelocationTable::add: section {} out of range (max {})",
            section,
            NUM_SECTIONS - 1
        );
        self.entries[section].push(RelocEntry {
            position,
            struct_count,
            offset_count,
            padding_count,
        });
    }

    /// Write the complete relocation table to `w`.
    ///
    /// `section_boundaries` contains the end-of-section byte offsets for each
    /// of the 5 sections: `[end_0, end_1, end_2, end_3, end_4]`.
    ///
    /// The sections are assumed to be contiguous: section 0 starts at offset 0,
    /// section 1 starts at `end_0`, section 2 starts at `end_1`, and so on.
    pub fn write(&self, w: &mut LittleEndianWriter, section_boundaries: &[u32; NUM_SECTIONS]) {
        // Sort entries within each section by position.
        let mut sorted_entries: [Vec<RelocEntry>; NUM_SECTIONS] = [
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        ];
        for i in 0..NUM_SECTIONS {
            sorted_entries[i] = self.entries[i].clone();
            sorted_entries[i].sort_by_key(|e| e.position);
        }

        // Compute total number of entries and per-section entry_index.
        let mut entry_indices = [0u32; NUM_SECTIONS];
        let mut running_index = 0u32;
        for i in 0..NUM_SECTIONS {
            entry_indices[i] = running_index;
            running_index += sorted_entries[i].len() as u32;
        }

        let rlt_pos = w.pos() as u32;

        // -- Header (16 bytes) --
        w.write_magic(b"_RLT");
        w.write_u32(rlt_pos);
        w.write_u32(NUM_SECTIONS as u32);
        w.write_u32(0); // padding

        // -- Section descriptors (24 bytes each) --
        for i in 0..NUM_SECTIONS {
            let section_start = if i == 0 {
                0u32
            } else {
                section_boundaries[i - 1]
            };
            let section_end = section_boundaries[i];
            let section_size = section_end.saturating_sub(section_start);

            // Pointer field: relative offset from this field to the section data.
            // For simplicity, store 0 for empty sections or the section_start.
            let pointer_field_pos = w.pos();
            if section_size > 0 {
                let relative = section_start as i64 - pointer_field_pos as i64;
                w.write_bytes(&relative.to_le_bytes());
            } else {
                w.write_u64(0); // null pointer for empty section
            }

            w.write_u32(section_start);
            w.write_u32(section_size);
            w.write_u32(entry_indices[i]);
            w.write_u32(sorted_entries[i].len() as u32);
        }

        // -- Relocation entries (8 bytes each) --
        for i in 0..NUM_SECTIONS {
            for entry in &sorted_entries[i] {
                w.write_u32(entry.position);
                w.write_u16(entry.struct_count);
                w.write_u8(entry.offset_count);
                w.write_u8(entry.padding_count);
            }
        }
    }
}

impl Default for RelocationTable {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reloc_table_magic_and_header() {
        let rt = RelocationTable::new();
        let mut w = LittleEndianWriter::new();
        let boundaries = [0x100, 0x200, 0x300, 0x400, 0x500];

        rt.write(&mut w, &boundaries);
        let buf = w.as_slice();

        // Verify magic.
        assert_eq!(&buf[0..4], b"_RLT");

        // Position field should be 0 (written at start of buffer).
        let pos = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
        assert_eq!(pos, 0);

        // Num sections.
        let num_sec = u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]);
        assert_eq!(num_sec, 5);

        // Padding.
        let pad = u32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]);
        assert_eq!(pad, 0);
    }

    #[test]
    fn reloc_table_section_descriptors() {
        let rt = RelocationTable::new();
        let mut w = LittleEndianWriter::new();
        let boundaries = [0x100, 0x200, 0x300, 0x400, 0x500];

        rt.write(&mut w, &boundaries);
        let buf = w.as_slice();

        // Header is 16 bytes, then 5 sections at 24 bytes each = 120 bytes.
        // Total minimum: 16 + 120 = 136 bytes.
        assert!(buf.len() >= 136);

        // Check section 0: starts at 0, size = 0x100.
        let sec0_off = 16; // after header
        // Skip the 8-byte pointer field.
        let sec0_position =
            u32::from_le_bytes([buf[sec0_off + 8], buf[sec0_off + 9], buf[sec0_off + 10], buf[sec0_off + 11]]);
        assert_eq!(sec0_position, 0);
        let sec0_size =
            u32::from_le_bytes([buf[sec0_off + 12], buf[sec0_off + 13], buf[sec0_off + 14], buf[sec0_off + 15]]);
        assert_eq!(sec0_size, 0x100);

        // Check section 1: starts at 0x100, size = 0x100.
        let sec1_off = 16 + 24;
        let sec1_position =
            u32::from_le_bytes([buf[sec1_off + 8], buf[sec1_off + 9], buf[sec1_off + 10], buf[sec1_off + 11]]);
        assert_eq!(sec1_position, 0x100);
        let sec1_size =
            u32::from_le_bytes([buf[sec1_off + 12], buf[sec1_off + 13], buf[sec1_off + 14], buf[sec1_off + 15]]);
        assert_eq!(sec1_size, 0x100);
    }

    #[test]
    fn reloc_table_entries() {
        let mut rt = RelocationTable::new();
        // Add some entries to section 0.
        rt.add(0, 0x20, 1, 1, 0);
        rt.add(0, 0x10, 2, 1, 0);

        let mut w = LittleEndianWriter::new();
        let boundaries = [0x100, 0x200, 0x300, 0x400, 0x500];

        rt.write(&mut w, &boundaries);
        let buf = w.as_slice();

        // Entries start after header (16) + sections (5 * 24 = 120) = offset 136.
        let entries_off = 136;

        // Entries are sorted by position, so 0x10 comes first.
        let e0_pos = u32::from_le_bytes([
            buf[entries_off],
            buf[entries_off + 1],
            buf[entries_off + 2],
            buf[entries_off + 3],
        ]);
        assert_eq!(e0_pos, 0x10);

        let e0_struct_count = u16::from_le_bytes([buf[entries_off + 4], buf[entries_off + 5]]);
        assert_eq!(e0_struct_count, 1);

        let e0_offset_count = buf[entries_off + 6];
        assert_eq!(e0_offset_count, 2);

        let e0_padding_count = buf[entries_off + 7];
        assert_eq!(e0_padding_count, 0);

        // Second entry at offset 136 + 8.
        let e1_pos = u32::from_le_bytes([
            buf[entries_off + 8],
            buf[entries_off + 9],
            buf[entries_off + 10],
            buf[entries_off + 11],
        ]);
        assert_eq!(e1_pos, 0x20);
    }

    #[test]
    fn reloc_table_entry_indices() {
        let mut rt = RelocationTable::new();
        rt.add(0, 0x10, 1, 1, 0);
        rt.add(0, 0x20, 1, 1, 0);
        rt.add(1, 0x110, 1, 1, 0);

        let mut w = LittleEndianWriter::new();
        let boundaries = [0x100, 0x200, 0x300, 0x400, 0x500];

        rt.write(&mut w, &boundaries);
        let buf = w.as_slice();

        // Section 0 descriptor: entry_index = 0, num_entries = 2.
        let sec0_off = 16;
        let sec0_entry_idx = u32::from_le_bytes([
            buf[sec0_off + 16],
            buf[sec0_off + 17],
            buf[sec0_off + 18],
            buf[sec0_off + 19],
        ]);
        assert_eq!(sec0_entry_idx, 0);
        let sec0_num = u32::from_le_bytes([
            buf[sec0_off + 20],
            buf[sec0_off + 21],
            buf[sec0_off + 22],
            buf[sec0_off + 23],
        ]);
        assert_eq!(sec0_num, 2);

        // Section 1 descriptor: entry_index = 2, num_entries = 1.
        let sec1_off = 16 + 24;
        let sec1_entry_idx = u32::from_le_bytes([
            buf[sec1_off + 16],
            buf[sec1_off + 17],
            buf[sec1_off + 18],
            buf[sec1_off + 19],
        ]);
        assert_eq!(sec1_entry_idx, 2);
        let sec1_num = u32::from_le_bytes([
            buf[sec1_off + 20],
            buf[sec1_off + 21],
            buf[sec1_off + 22],
            buf[sec1_off + 23],
        ]);
        assert_eq!(sec1_num, 1);
    }

    #[test]
    #[should_panic(expected = "section 5 out of range")]
    fn reloc_table_invalid_section() {
        let mut rt = RelocationTable::new();
        rt.add(5, 0, 1, 1, 0);
    }
}
