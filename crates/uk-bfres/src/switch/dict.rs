//! Patricia trie dictionary writer for Switch BFRES.
//!
//! Switch BFRES dictionaries store name lookups as a compact Patricia trie.
//! Each node is 16 bytes:
//!
//! ```text
//! +0x00  u32  reference (bit index for trie navigation)
//! +0x04  u16  left_index  (index into the node array)
//! +0x06  u16  right_index (index into the node array)
//! +0x08  i64  key_offset  (relative offset to the string text in the string pool)
//! ```
//!
//! The dictionary header is:
//!
//! ```text
//! +0x00  i32  signature (0xFFFFFFFF, i.e. -1)
//! +0x04  i32  num_nodes (count of entry nodes, EXCLUDING the root sentinel)
//! ```
//!
//! Followed by `num_nodes + 1` nodes: the root sentinel (index 0) and then the
//! entry nodes (indices 1..=num_nodes).
//!
//! The algorithm is ported from BfresLibrary's `ResDictUpdate.cs`.

use crate::binary::LittleEndianWriter;
use crate::switch::string_table::StringTable;

// ---------------------------------------------------------------------------
// Big-integer helpers (using u128 — BFRES keys are short enough)
// ---------------------------------------------------------------------------

/// Convert a string's UTF-8 bytes to a big-endian integer representation.
/// The first byte is the most significant.
fn string_to_bigint(s: &str) -> u128 {
    let bytes = s.as_bytes();
    let mut result: u128 = 0;
    for &b in bytes {
        result = (result << 8) | (b as u128);
    }
    result
}

/// Get bit `b` of integer `n` (bit 0 = least significant).
fn bit(n: u128, b: i32) -> usize {
    ((n >> (b as u32 & 0x7F)) & 1) as usize
}

/// Find the index of the lowest set bit in `n`. Panics if `n == 0`.
fn first_1bit(n: u128) -> i32 {
    assert_ne!(n, 0, "first_1bit called on zero");
    n.trailing_zeros() as i32
}

/// Find the lowest bit position where `a` and `b` differ.
/// Returns -1 if they are identical.
fn bit_mismatch(a: u128, b: u128) -> i32 {
    let xor = a ^ b;
    if xor == 0 {
        return -1;
    }
    xor.trailing_zeros() as i32
}

/// Number of bits needed to represent `n` (equivalent to floor(log2(n)) + 1).
#[allow(dead_code)]
fn bit_length(n: u128) -> i32 {
    if n == 0 {
        return 1;
    }
    (128 - n.leading_zeros()) as i32
}

// ---------------------------------------------------------------------------
// Internal trie construction
// ---------------------------------------------------------------------------

/// A node in the construction-time Patricia trie.
struct TrieNode {
    data: u128,
    bit_idx: i32,
    parent: usize,   // index into the node vec
    child: [usize; 2], // [left, right]
}

/// A constructed trie that can build the final BFRES dictionary nodes.
struct Trie {
    nodes: Vec<TrieNode>,
    /// Maps data -> (insertion_order_index, node_index)
    entries: Vec<(u128, usize)>,
}

impl Trie {
    fn new() -> Self {
        // Create the root node (index 0). It points to itself initially.
        let root = TrieNode {
            data: 0,
            bit_idx: -1,
            parent: 0,
            child: [0, 0],
        };
        let mut trie = Trie {
            nodes: vec![root],
            entries: Vec::new(),
        };
        trie.entries.push((0, 0));
        trie
    }

    fn entry_index_for_data(&self, data: u128) -> Option<usize> {
        self.entries.iter().position(|(d, _)| *d == data)
    }

    fn search(&self, data: u128, want_prev: bool) -> usize {
        let root_idx = 0;
        if self.nodes[root_idx].child[0] == root_idx {
            return root_idx;
        }

        let mut node_idx = self.nodes[root_idx].child[0];
        let mut prev_idx;
        loop {
            prev_idx = node_idx;
            let b = bit(data, self.nodes[node_idx].bit_idx);
            node_idx = self.nodes[node_idx].child[b];
            if self.nodes[node_idx].bit_idx <= self.nodes[prev_idx].bit_idx {
                break;
            }
        }
        if want_prev {
            prev_idx
        } else {
            node_idx
        }
    }

    fn insert(&mut self, name: &str) {
        let data = string_to_bigint(name);
        let current_idx = self.search(data, true);
        let current_data = self.nodes[current_idx].data;
        let bit_idx = bit_mismatch(current_data, data);

        // Walk up the tree while bit_idx is less than the parent's bit_idx.
        let mut cur = current_idx;
        while bit_idx < self.nodes[self.nodes[cur].parent].bit_idx {
            cur = self.nodes[cur].parent;
        }

        let cur_bit_idx = self.nodes[cur].bit_idx;

        if bit_idx < cur_bit_idx {
            // Insert between cur and its parent.
            let parent_idx = self.nodes[cur].parent;
            let parent_bit_idx = self.nodes[parent_idx].bit_idx;
            let new_idx = self.nodes.len();

            let mut new_node = TrieNode {
                data,
                bit_idx,
                parent: parent_idx,
                child: [new_idx, new_idx], // self-referencing initially
            };

            // The opposite child points to `cur`.
            new_node.child[bit(data, bit_idx) ^ 1] = cur;
            // The same-side child points to self (the new node is a leaf for `data`).
            // (already set above via self-referencing)

            self.nodes.push(new_node);

            // Update parent's child pointer.
            let parent_child_side = bit(data, parent_bit_idx);
            self.nodes[parent_idx].child[parent_child_side] = new_idx;

            // Update cur's parent.
            self.nodes[cur].parent = new_idx;

            self.entries.push((data, new_idx));
        } else if bit_idx > cur_bit_idx {
            // Insert as a child of `cur`.
            let new_idx = self.nodes.len();

            let mut new_node = TrieNode {
                data,
                bit_idx,
                parent: cur,
                child: [new_idx, new_idx], // self-referencing initially
            };

            let cur_data = self.nodes[cur].data;
            let opposite_side = bit(data, bit_idx) ^ 1;
            if bit(cur_data, bit_idx) == opposite_side {
                new_node.child[opposite_side] = cur;
            } else {
                new_node.child[opposite_side] = 0; // root
            }

            self.nodes.push(new_node);

            let child_side = bit(data, cur_bit_idx);
            self.nodes[cur].child[child_side] = new_idx;

            self.entries.push((data, new_idx));
        } else {
            // bit_idx == cur_bit_idx — same level, different subtree.
            let child_side = bit(data, bit_idx);
            let existing_child = self.nodes[cur].child[child_side];

            let new_bit_idx = if existing_child == 0 {
                // Existing child is root.
                first_1bit(data)
            } else {
                bit_mismatch(self.nodes[existing_child].data, data)
            };

            let new_idx = self.nodes.len();
            let mut new_node = TrieNode {
                data,
                bit_idx: new_bit_idx,
                parent: cur,
                child: [new_idx, new_idx],
            };

            let opposite_side_new = bit(data, new_bit_idx) ^ 1;
            new_node.child[opposite_side_new] = existing_child;

            self.nodes.push(new_node);
            self.nodes[cur].child[child_side] = new_idx;

            self.entries.push((data, new_idx));
        }
    }
}

/// Compact the bit index the same way BfresLibrary does:
/// `(byte_index << 3) | bit_within_byte`.
/// Since the raw bit_idx is already a bit position counting from LSB of the
/// big-endian integer, this just returns it as-is (the C# code's
/// GetCompactBitIdx simplifies to identity for this representation).
fn compact_bit_idx(bit_idx: i32) -> u32 {
    let byte_idx = bit_idx / 8;
    let bit_in_byte = bit_idx - 8 * byte_idx;
    ((byte_idx << 3) | bit_in_byte) as u32
}

/// A finalized dictionary node ready for serialization.
#[derive(PartialEq)]
struct DictNode {
    reference: u32,
    left_index: u16,
    right_index: u16,
    key: String,
}

/// Build the finalized node list from a set of keys.
fn build_dict_nodes(keys: &[String]) -> Vec<DictNode> {
    let mut trie = Trie::new();
    for key in keys {
        trie.insert(key);
    }

    let mut result = Vec::with_capacity(trie.entries.len());

    for &(entry_data, node_idx) in &trie.entries {
        let node = &trie.nodes[node_idx];
        let reference = compact_bit_idx(node.bit_idx) & 0xFFFFFFFF;

        let left_data = trie.nodes[node.child[0]].data;
        let right_data = trie.nodes[node.child[1]].data;

        let left_index = trie
            .entry_index_for_data(left_data)
            .expect("left child data not in entries") as u16;
        let right_index = trie
            .entry_index_for_data(right_data)
            .expect("right child data not in entries") as u16;

        // Reconstruct the key name from the data. For the root (entry_data == 0),
        // the key is empty.
        let key = if entry_data == 0 {
            String::new()
        } else {
            // Convert the big-endian integer back to a string.
            let mut bytes = Vec::new();
            let mut val = entry_data;
            while val > 0 {
                bytes.push((val & 0xFF) as u8);
                val >>= 8;
            }
            bytes.reverse();
            String::from_utf8(bytes).unwrap_or_default()
        };

        result.push(DictNode {
            reference,
            left_index,
            right_index,
            key,
        });
    }

    // The root node (index 0) should have an empty key.
    if !result.is_empty() {
        result[0].key = String::new();
    }

    result
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Builder for a Switch BFRES Patricia trie dictionary.
///
/// Register keys with [`add`](DictBuilder::add), then call
/// [`write`](DictBuilder::write) to serialize the dictionary.
pub struct DictBuilder {
    keys: Vec<String>,
}

impl DictBuilder {
    /// Create a new, empty dictionary builder.
    pub fn new() -> Self {
        Self { keys: Vec::new() }
    }

    /// Add a key to the dictionary.
    pub fn add(&mut self, key: &str) {
        self.keys.push(key.to_string());
    }

    /// Write the dictionary to `w`.
    ///
    /// The `string_table` and `pool_start` are used to resolve the relative
    /// offsets from each node's key_offset field to the corresponding string
    /// text in the string pool.
    ///
    /// Returns the absolute byte position where the dictionary starts.
    pub fn write(
        &self,
        w: &mut LittleEndianWriter,
        string_table: &StringTable,
        pool_start: usize,
    ) -> usize {
        let nodes = build_dict_nodes(&self.keys);
        let num_entries = if nodes.is_empty() {
            0
        } else {
            nodes.len() - 1 // exclude root sentinel
        };

        let dict_start = w.pos();

        // -- Header --
        w.write_i32(-1); // signature 0xFFFFFFFF
        w.write_i32(num_entries as i32);

        // -- Nodes --
        for node in &nodes {
            w.write_u32(node.reference);
            w.write_u16(node.left_index);
            w.write_u16(node.right_index);

            // key_offset: i64 relative offset to string text.
            // For the root sentinel (empty key), use offset 0 (null).
            let key_offset_field_pos = w.pos();
            if node.key.is_empty() && node == &nodes[0] {
                // Root sentinel — null offset.
                w.write_u64(0);
            } else {
                // Resolve the string position.
                let string_pos = string_table.get_position(&node.key, pool_start);
                let relative = string_pos as i64 - key_offset_field_pos as i64;
                w.write_bytes(&relative.to_le_bytes());
            }
        }

        dict_start
    }
}

impl Default for DictBuilder {
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
    fn bigint_conversion() {
        // "A" = 0x41
        assert_eq!(string_to_bigint("A"), 0x41);
        // "AB" = 0x4142
        assert_eq!(string_to_bigint("AB"), 0x4142);
        // "" = 0
        assert_eq!(string_to_bigint(""), 0);
    }

    #[test]
    fn bit_mismatch_basic() {
        assert_eq!(bit_mismatch(0b1010, 0b1011), 0);
        assert_eq!(bit_mismatch(0b1010, 0b1000), 1);
        assert_eq!(bit_mismatch(0b1010, 0b1010), -1);
    }

    #[test]
    fn build_nodes_single_key() {
        let keys = vec!["test".to_string()];
        let nodes = build_dict_nodes(&keys);

        // Should have 2 nodes: root sentinel + 1 entry.
        assert_eq!(nodes.len(), 2);

        // Root sentinel.
        assert_eq!(nodes[0].reference, compact_bit_idx(-1));
        assert!(nodes[0].key.is_empty());

        // Entry node.
        assert_eq!(nodes[1].key, "test");
    }

    #[test]
    fn build_nodes_multiple_keys() {
        let keys = vec![
            "alpha".to_string(),
            "beta".to_string(),
            "gamma".to_string(),
        ];
        let nodes = build_dict_nodes(&keys);

        // 1 root + 3 entries = 4 nodes.
        assert_eq!(nodes.len(), 4);

        // All keys should appear in the node list (excluding root).
        let entry_keys: Vec<&str> = nodes[1..].iter().map(|n| n.key.as_str()).collect();
        assert!(entry_keys.contains(&"alpha"));
        assert!(entry_keys.contains(&"beta"));
        assert!(entry_keys.contains(&"gamma"));
    }

    #[test]
    fn dict_write_structure() {
        let mut st = StringTable::new();
        st.add("foo");
        st.add("bar");

        let mut w = LittleEndianWriter::new();
        let pool_start = st.write(&mut w);

        let mut db = DictBuilder::new();
        db.add("foo");
        db.add("bar");

        let dict_start = db.write(&mut w, &st, pool_start);
        let buf = w.as_slice();

        // Verify header.
        let sig = i32::from_le_bytes([
            buf[dict_start],
            buf[dict_start + 1],
            buf[dict_start + 2],
            buf[dict_start + 3],
        ]);
        assert_eq!(sig, -1);

        let num = i32::from_le_bytes([
            buf[dict_start + 4],
            buf[dict_start + 5],
            buf[dict_start + 6],
            buf[dict_start + 7],
        ]);
        assert_eq!(num, 2); // 2 entry nodes (foo, bar)

        // Each node is 16 bytes. Total: 8 (header) + 3*16 (root + 2 entries) = 56 bytes.
        let expected_size = 8 + 3 * 16;
        assert!(buf.len() >= dict_start + expected_size);
    }

    #[test]
    fn dict_write_root_sentinel() {
        let mut st = StringTable::new();
        st.add("x");

        let mut w = LittleEndianWriter::new();
        let pool_start = st.write(&mut w);

        let mut db = DictBuilder::new();
        db.add("x");

        let dict_start = db.write(&mut w, &st, pool_start);
        let buf = w.as_slice();

        // Root sentinel is at dict_start + 8.
        let root_off = dict_start + 8;

        // Root sentinel's reference should be compact_bit_idx(-1) = 0xFFFFFFFF.
        let reference = u32::from_le_bytes([
            buf[root_off],
            buf[root_off + 1],
            buf[root_off + 2],
            buf[root_off + 3],
        ]);
        assert_eq!(reference, 0xFFFFFFFF);

        // In a Patricia trie with one key, the root's left child points to
        // the single entry node (index 1).
        let left = u16::from_le_bytes([buf[root_off + 4], buf[root_off + 5]]);
        let right = u16::from_le_bytes([buf[root_off + 6], buf[root_off + 7]]);
        assert_eq!(left, 1, "root left child should point to the entry");
        assert_eq!(right, 0, "root right child should point to itself");

        // Root sentinel's key offset should be 0 (null).
        let key_off = i64::from_le_bytes([
            buf[root_off + 8],
            buf[root_off + 9],
            buf[root_off + 10],
            buf[root_off + 11],
            buf[root_off + 12],
            buf[root_off + 13],
            buf[root_off + 14],
            buf[root_off + 15],
        ]);
        assert_eq!(key_off, 0);
    }
}
