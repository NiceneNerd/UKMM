//! String table (pool) writer for Switch BFRES.
//!
//! The string table collects, deduplicates, and sorts all strings referenced in
//! the file, then writes them as a single pool with the `_STR` block header.
//!
//! # Binary layout
//!
//! ```text
//! +0x00  [u8; 4]  magic "_STR"
//! +0x04  u32      block header offset (0x10, pointing past this 16-byte header)
//! +0x08  u32      block size (total pool size in bytes, excluding the 16-byte header)
//! +0x0C  u32      padding (0)
//! -- per string entry --
//! +0x00  u16      string byte length (excluding null terminator)
//! +0x02  [u8; N]  UTF-8 string bytes
//! +....  u8       0x00 null terminator
//! +....  [u8; ?]  pad to 2-byte alignment
//! ```

use indexmap::IndexMap;

use crate::binary::LittleEndianWriter;

/// Collects, deduplicates, and writes a BFRES string pool.
///
/// Strings are registered during the collection phase via [`add`](StringTable::add),
/// then written as a single `_STR` block via [`write`](StringTable::write). After
/// writing, the absolute position of any string can be retrieved with
/// [`get_position`](StringTable::get_position).
pub struct StringTable {
    /// Maps string text to the byte offset of the string *text* within the pool
    /// (relative to pool start, past the 16-byte header *and* the per-entry u16
    /// length prefix). Populated during [`write`].
    strings: IndexMap<String, usize>,
}

impl StringTable {
    /// Create a new, empty string table.
    pub fn new() -> Self {
        Self {
            strings: IndexMap::new(),
        }
    }

    /// Register a string for inclusion in the pool.
    ///
    /// Duplicate calls with the same string are harmless — the pool deduplicates
    /// automatically. The empty string `""` is always included as the first entry
    /// and does not need to be added explicitly.
    pub fn add(&mut self, s: &str) {
        if !self.strings.contains_key(s) {
            self.strings.insert(s.to_string(), 0);
        }
    }

    /// Write the complete string pool to `w`.
    ///
    /// Returns the absolute byte position where the pool starts (i.e. the
    /// position of the `_STR` magic).
    ///
    /// After this call, [`get_position`](StringTable::get_position) can be used
    /// to look up the absolute position of any registered string.
    pub fn write(&mut self, w: &mut LittleEndianWriter) -> usize {
        // Ensure the empty string is the first entry.
        if !self.strings.contains_key("") {
            // Insert at the front.
            let mut new_map = IndexMap::new();
            new_map.insert(String::new(), 0usize);
            for (k, v) in self.strings.drain(..) {
                new_map.insert(k, v);
            }
            self.strings = new_map;
        } else {
            // Move "" to position 0 if it is not already there.
            self.strings.move_index(
                self.strings.get_index_of("").unwrap(),
                0,
            );
        }

        // Sort entries 1.. alphabetically (entry 0 is always "").
        // Collect the non-empty keys, sort, rebuild.
        let empty_val = *self.strings.get("").unwrap();
        let mut rest: Vec<(String, usize)> = self
            .strings
            .iter()
            .filter(|(k, _)| !k.is_empty())
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        rest.sort_by(|a, b| a.0.cmp(&b.0));

        self.strings.clear();
        self.strings.insert(String::new(), empty_val);
        for (k, v) in rest {
            self.strings.insert(k, v);
        }

        let pool_start = w.pos();

        // -- Write 16-byte block header --
        w.write_magic(b"_STR");
        let block_header_offset_pos = w.pos();
        w.write_u32(0); // placeholder: block header offset
        let block_size_pos = w.pos();
        w.write_u32(0); // placeholder: block size
        w.write_u32(0); // padding

        let entries_start = w.pos();

        // The block header offset field points to the start of entries.
        w.set_u32_at(block_header_offset_pos, (entries_start - pool_start) as u32);

        // -- Write each string entry --
        for (key, offset_slot) in self.strings.iter_mut() {
            let entry_start = w.pos();
            let bytes = key.as_bytes();
            w.write_u16(bytes.len() as u16);
            // Record the position of the string text (past the u16 length).
            *offset_slot = w.pos() - pool_start;
            w.write_bytes(bytes);
            w.write_u8(0); // null terminator
            // Align to 2 bytes.
            w.align(2);

            // Sanity: entry_start should be 2-byte aligned already because we
            // align each entry, and the header is 16 bytes (aligned).
            debug_assert_eq!(entry_start % 2, 0);
        }

        // Fill in block size (total entries size).
        let block_size = (w.pos() - entries_start) as u32;
        w.set_u32_at(block_size_pos, block_size);

        pool_start
    }

    /// Get the absolute byte position of a string's text in the output buffer.
    ///
    /// This position points past the u16 length prefix, directly to the first
    /// byte of the UTF-8 string data. Must be called *after* [`write`].
    ///
    /// # Panics
    ///
    /// Panics if `s` was never registered or [`write`] has not been called.
    pub fn get_position(&self, s: &str, pool_start: usize) -> usize {
        let relative = self
            .strings
            .get(s)
            .unwrap_or_else(|| panic!("StringTable::get_position: string {:?} not registered", s));
        pool_start + relative
    }
}

impl Default for StringTable {
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
    fn string_table_basic_write() {
        let mut st = StringTable::new();
        st.add("hello");
        st.add("world");
        st.add("hello"); // duplicate – should be ignored

        let mut w = LittleEndianWriter::new();
        let pool_start = st.write(&mut w);
        let buf = w.as_slice();

        // Pool starts at position 0 in this test.
        assert_eq!(pool_start, 0);

        // Verify magic.
        assert_eq!(&buf[0..4], b"_STR");

        // Block header offset should be 0x10 (16).
        let header_off = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
        assert_eq!(header_off, 0x10);

        // Padding at offset 12 should be zero.
        let pad = u32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]);
        assert_eq!(pad, 0);
    }

    #[test]
    fn string_table_empty_string_first() {
        let mut st = StringTable::new();
        st.add("banana");
        st.add("apple");

        let mut w = LittleEndianWriter::new();
        let pool_start = st.write(&mut w);
        let buf = w.as_slice();

        // First string entry starts at offset 16. Its length should be 0 (empty string).
        let first_len = u16::from_le_bytes([buf[16], buf[17]]);
        assert_eq!(first_len, 0);

        // The empty string text position is at offset 18.
        let empty_pos = st.get_position("", pool_start);
        assert_eq!(empty_pos, 18);
    }

    #[test]
    fn string_table_positions_correct() {
        let mut st = StringTable::new();
        st.add("ab");
        st.add("cd");

        let mut w = LittleEndianWriter::new();
        let pool_start = st.write(&mut w);
        let buf = w.as_slice();

        // Verify each string can be found at its reported position.
        for s in ["", "ab", "cd"] {
            let pos = st.get_position(s, pool_start);
            let bytes = s.as_bytes();
            assert_eq!(&buf[pos..pos + bytes.len()], bytes);
            // Null terminator follows.
            assert_eq!(buf[pos + bytes.len()], 0);
        }
    }

    #[test]
    fn string_table_sorted() {
        let mut st = StringTable::new();
        st.add("cherry");
        st.add("apple");
        st.add("banana");

        let mut w = LittleEndianWriter::new();
        let pool_start = st.write(&mut w);

        // Positions should be in sorted order: "" < "apple" < "banana" < "cherry".
        let pos_empty = st.get_position("", pool_start);
        let pos_apple = st.get_position("apple", pool_start);
        let pos_banana = st.get_position("banana", pool_start);
        let pos_cherry = st.get_position("cherry", pool_start);

        assert!(pos_empty < pos_apple);
        assert!(pos_apple < pos_banana);
        assert!(pos_banana < pos_cherry);
    }

    #[test]
    fn string_table_alignment() {
        let mut st = StringTable::new();
        // "a" is 1 byte: u16(1) + 'a' + '\0' = 4 bytes (already 2-aligned).
        // "ab" is 2 bytes: u16(2) + 'a' + 'b' + '\0' = 5 bytes -> padded to 6.
        st.add("a");
        st.add("ab");

        let mut w = LittleEndianWriter::new();
        let pool_start = st.write(&mut w);

        // All positions should be past a u16 length field, so they should be
        // at even-offset + 2.
        let pos_a = st.get_position("a", pool_start);
        let pos_ab = st.get_position("ab", pool_start);

        // The u16 length field is 2 bytes before the text position,
        // and should be at a 2-byte-aligned offset.
        assert_eq!((pos_a - 2) % 2, 0);
        assert_eq!((pos_ab - 2) % 2, 0);
    }
}
