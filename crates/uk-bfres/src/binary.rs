//! Binary I/O primitives for BFRES parsing and writing.
//!
//! - [`BigEndianReader`]: zero-copy reader over a `&[u8]` slice (for Wii U BFRES).
//! - [`LittleEndianWriter`]: growable buffer writer (for Switch BFRES output).

use crate::error::{BfresError, Result};

// ---------------------------------------------------------------------------
// BigEndianReader
// ---------------------------------------------------------------------------

/// Zero-copy big-endian reader over a borrowed byte slice.
pub struct BigEndianReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> BigEndianReader<'a> {
    /// Create a new reader starting at position 0.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    // -- helpers ----------------------------------------------------------

    /// Borrow `len` bytes starting at the current position, advancing `pos`.
    /// Returns `UnexpectedEof` if there are not enough bytes remaining.
    #[inline]
    fn ensure(&mut self, len: usize) -> Result<&'a [u8]> {
        if self.pos + len > self.data.len() {
            return Err(BfresError::UnexpectedEof {
                offset: self.pos,
                needed: len,
            });
        }
        let slice = &self.data[self.pos..self.pos + len];
        self.pos += len;
        Ok(slice)
    }

    // -- primitive reads --------------------------------------------------

    pub fn read_u8(&mut self) -> Result<u8> {
        let b = self.ensure(1)?;
        Ok(b[0])
    }

    pub fn read_u16(&mut self) -> Result<u16> {
        let b = self.ensure(2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }

    pub fn read_u32(&mut self) -> Result<u32> {
        let b = self.ensure(4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn read_i16(&mut self) -> Result<i16> {
        let b = self.ensure(2)?;
        Ok(i16::from_be_bytes([b[0], b[1]]))
    }

    pub fn read_i32(&mut self) -> Result<i32> {
        let b = self.ensure(4)?;
        Ok(i32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn read_u64(&mut self) -> Result<u64> {
        let b = self.ensure(8)?;
        Ok(u64::from_be_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    pub fn read_f32(&mut self) -> Result<f32> {
        let b = self.ensure(4)?;
        Ok(f32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// Read `len` bytes as a borrowed slice, advancing the position.
    pub fn read_bytes(&mut self, len: usize) -> Result<&'a [u8]> {
        self.ensure(len)
    }

    /// Read a 4-byte ASCII magic (e.g. `b"FRES"`, `b"FMDL"`).
    pub fn read_magic(&mut self) -> Result<[u8; 4]> {
        let b = self.ensure(4)?;
        Ok([b[0], b[1], b[2], b[3]])
    }

    /// Read a null-terminated UTF-8 string starting at `offset` **without**
    /// changing the current read position.
    pub fn read_string_at(&self, offset: usize) -> Result<String> {
        if offset >= self.data.len() {
            return Err(BfresError::UnexpectedEof {
                offset,
                needed: 1,
            });
        }
        let remaining = &self.data[offset..];
        let end = remaining
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(remaining.len());
        let s = String::from_utf8_lossy(&remaining[..end]).into_owned();
        Ok(s)
    }

    // -- navigation -------------------------------------------------------

    /// Absolute seek – set the read position directly.
    pub fn seek(&mut self, pos: usize) {
        self.pos = pos;
    }

    /// Relative skip – advance the read position by `n` bytes.
    pub fn skip(&mut self, n: usize) {
        self.pos += n;
    }

    /// Current read position.
    #[inline]
    pub fn pos(&self) -> usize {
        self.pos
    }

    /// Total length of the underlying data.
    #[inline]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Number of bytes remaining from the current position.
    #[inline]
    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }
}

// ---------------------------------------------------------------------------
// LittleEndianWriter
// ---------------------------------------------------------------------------

/// Growable little-endian byte buffer writer for building Switch BFRES files.
pub struct LittleEndianWriter {
    data: Vec<u8>,
}

impl LittleEndianWriter {
    /// Create a new, empty writer.
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    /// Create a new writer with pre-allocated capacity.
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            data: Vec::with_capacity(cap),
        }
    }

    // -- primitive writes -------------------------------------------------

    pub fn write_u8(&mut self, v: u8) {
        self.data.push(v);
    }

    pub fn write_u16(&mut self, v: u16) {
        self.data.extend_from_slice(&v.to_le_bytes());
    }

    pub fn write_u32(&mut self, v: u32) {
        self.data.extend_from_slice(&v.to_le_bytes());
    }

    pub fn write_i16(&mut self, v: i16) {
        self.data.extend_from_slice(&v.to_le_bytes());
    }

    pub fn write_i32(&mut self, v: i32) {
        self.data.extend_from_slice(&v.to_le_bytes());
    }

    pub fn write_u64(&mut self, v: u64) {
        self.data.extend_from_slice(&v.to_le_bytes());
    }

    pub fn write_f32(&mut self, v: f32) {
        self.data.extend_from_slice(&v.to_le_bytes());
    }

    /// Append raw bytes.
    pub fn write_bytes(&mut self, bytes: &[u8]) {
        self.data.extend_from_slice(bytes);
    }

    /// Write a 4-byte ASCII magic identifier.
    pub fn write_magic(&mut self, magic: &[u8; 4]) {
        self.data.extend_from_slice(magic);
    }

    /// Write a string followed by a null terminator.
    pub fn write_string(&mut self, s: &str) {
        self.data.extend_from_slice(s.as_bytes());
        self.data.push(0);
    }

    /// Write `count` zero bytes.
    pub fn write_zeros(&mut self, count: usize) {
        self.data.resize(self.data.len() + count, 0);
    }

    // -- navigation / alignment -------------------------------------------

    /// Current write position (equal to the buffer length).
    #[inline]
    pub fn pos(&self) -> usize {
        self.data.len()
    }

    /// Pad the buffer with zeros until `pos()` is a multiple of `alignment`.
    /// If `alignment` is 0 or 1 this is a no-op.
    pub fn align(&mut self, alignment: usize) {
        if alignment <= 1 {
            return;
        }
        let remainder = self.data.len() % alignment;
        if remainder != 0 {
            let padding = alignment - remainder;
            self.write_zeros(padding);
        }
    }

    // -- offset fixup helpers ---------------------------------------------

    /// Reserve space for a 4-byte offset (written as zeros for now).
    /// Returns the position of the placeholder so it can be patched later.
    pub fn write_offset_placeholder(&mut self) -> usize {
        let pos = self.data.len();
        self.write_zeros(4);
        pos
    }

    /// Reserve space for an 8-byte (64-bit) offset.
    /// Switch BFRES uses 64-bit relative offsets.
    pub fn write_offset_placeholder_64(&mut self) -> usize {
        let pos = self.data.len();
        self.write_zeros(8);
        pos
    }

    /// Patch a previously reserved 4-byte placeholder with a *relative* offset.
    ///
    /// Writes `(target_pos - placeholder_pos)` as a little-endian `u32` at
    /// `placeholder_pos`.
    pub fn fixup_offset(&mut self, placeholder_pos: usize, target_pos: usize) {
        let relative = (target_pos.wrapping_sub(placeholder_pos)) as u32;
        let bytes = relative.to_le_bytes();
        self.data[placeholder_pos..placeholder_pos + 4].copy_from_slice(&bytes);
    }

    /// Patch a previously reserved 8-byte placeholder with a *relative* 64-bit offset.
    ///
    /// Writes `(target_pos - placeholder_pos)` as a little-endian `i64`.
    /// Writes 0 if `target_pos` is 0 (null pointer convention).
    pub fn fixup_offset_64(&mut self, placeholder_pos: usize, target_pos: usize) {
        let value = if target_pos == 0 {
            0i64
        } else {
            target_pos.wrapping_sub(placeholder_pos) as i64
        };
        let bytes = value.to_le_bytes();
        self.data[placeholder_pos..placeholder_pos + 8].copy_from_slice(&bytes);
    }

    /// Patch a previously reserved placeholder with an *absolute* `u32` value.
    pub fn fixup_offset_absolute(&mut self, placeholder_pos: usize, value: u32) {
        let bytes = value.to_le_bytes();
        self.data[placeholder_pos..placeholder_pos + 4].copy_from_slice(&bytes);
    }

    /// Overwrite a u32 at a specific position (for backpatching sizes, etc.).
    pub fn set_u32_at(&mut self, pos: usize, value: u32) {
        let bytes = value.to_le_bytes();
        self.data[pos..pos + 4].copy_from_slice(&bytes);
    }

    /// Overwrite a u16 at a specific position.
    pub fn set_u16_at(&mut self, pos: usize, value: u16) {
        let bytes = value.to_le_bytes();
        self.data[pos..pos + 2].copy_from_slice(&bytes);
    }

    /// Get mutable access to the underlying data buffer.
    pub fn data_mut(&mut self) -> &mut Vec<u8> {
        &mut self.data
    }

    // -- finalisation -----------------------------------------------------

    /// Consume the writer and return the underlying buffer.
    pub fn into_inner(self) -> Vec<u8> {
        self.data
    }

    /// Borrow the buffer as a byte slice.
    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }
}

impl Default for LittleEndianWriter {
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

    // -- BigEndianReader ---------------------------------------------------

    #[test]
    fn reader_round_trip() {
        // Manually construct big-endian encoded values:
        //   u8  = 0x42
        //   u16 = 0x1234
        //   u32 = 0xDEADBEEF
        //   i16 = -1234 (0xFB2E)
        //   i32 = -100_000 (0xFFFE7960)
        //   u64 = 0x0102030405060708
        //   f32 = 1.0  (0x3F800000)
        let mut buf: Vec<u8> = Vec::new();
        buf.push(0x42);                                       // u8
        buf.extend_from_slice(&0x1234u16.to_be_bytes());      // u16
        buf.extend_from_slice(&0xDEADBEEFu32.to_be_bytes());  // u32
        buf.extend_from_slice(&(-1234i16).to_be_bytes());     // i16
        buf.extend_from_slice(&(-100_000i32).to_be_bytes());  // i32
        buf.extend_from_slice(&0x0102030405060708u64.to_be_bytes()); // u64
        buf.extend_from_slice(&1.0f32.to_be_bytes());         // f32

        let mut r = BigEndianReader::new(&buf);

        assert_eq!(r.read_u8().unwrap(), 0x42);
        assert_eq!(r.read_u16().unwrap(), 0x1234);
        assert_eq!(r.read_u32().unwrap(), 0xDEAD_BEEF);
        assert_eq!(r.read_i16().unwrap(), -1234);
        assert_eq!(r.read_i32().unwrap(), -100_000);
        assert_eq!(r.read_u64().unwrap(), 0x0102030405060708);
        assert_eq!(r.read_f32().unwrap(), 1.0);
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn reader_eof_error() {
        let buf = [0u8; 2];
        let mut r = BigEndianReader::new(&buf);

        // Reading 2 bytes succeeds.
        assert!(r.read_u16().is_ok());

        // Now at end – any further read should fail.
        let err = r.read_u8().unwrap_err();
        match err {
            BfresError::UnexpectedEof { offset, needed } => {
                assert_eq!(offset, 2);
                assert_eq!(needed, 1);
            }
            other => panic!("Expected UnexpectedEof, got {:?}", other),
        }
    }

    #[test]
    fn reader_read_string_at() {
        let mut buf = Vec::new();
        // Put a 4-byte magic first, then a null-terminated string at offset 4.
        buf.extend_from_slice(b"FRES");
        buf.extend_from_slice(b"hello\0world\0");

        let r = BigEndianReader::new(&buf);

        assert_eq!(r.read_string_at(4).unwrap(), "hello");
        assert_eq!(r.read_string_at(10).unwrap(), "world");
        // Position should remain 0 – read_string_at does not advance.
        assert_eq!(r.pos(), 0);
    }

    #[test]
    fn reader_read_magic_and_bytes() {
        let buf = b"FRESextra";
        let mut r = BigEndianReader::new(buf);

        let magic = r.read_magic().unwrap();
        assert_eq!(&magic, b"FRES");

        let rest = r.read_bytes(5).unwrap();
        assert_eq!(rest, b"extra");
    }

    #[test]
    fn reader_seek_skip_len_remaining() {
        let buf = [0u8; 20];
        let mut r = BigEndianReader::new(&buf);

        assert_eq!(r.len(), 20);
        assert_eq!(r.pos(), 0);
        assert_eq!(r.remaining(), 20);

        r.skip(7);
        assert_eq!(r.pos(), 7);
        assert_eq!(r.remaining(), 13);

        r.seek(15);
        assert_eq!(r.pos(), 15);
        assert_eq!(r.remaining(), 5);
    }

    // -- LittleEndianWriter -----------------------------------------------

    #[test]
    fn writer_le_byte_order() {
        let mut w = LittleEndianWriter::new();
        w.write_u8(0x42);
        w.write_u16(0x1234);
        w.write_u32(0xDEADBEEF);
        w.write_i16(-1);
        w.write_i32(-1);
        w.write_u64(0x0102030405060708);
        w.write_f32(1.0);

        let buf = w.into_inner();
        let mut offset = 0;

        // u8
        assert_eq!(buf[offset], 0x42);
        offset += 1;

        // u16 LE
        assert_eq!(&buf[offset..offset + 2], &0x1234u16.to_le_bytes());
        offset += 2;

        // u32 LE
        assert_eq!(&buf[offset..offset + 4], &0xDEADBEEFu32.to_le_bytes());
        offset += 4;

        // i16 LE (-1)
        assert_eq!(&buf[offset..offset + 2], &(-1i16).to_le_bytes());
        offset += 2;

        // i32 LE (-1)
        assert_eq!(&buf[offset..offset + 4], &(-1i32).to_le_bytes());
        offset += 4;

        // u64 LE
        assert_eq!(
            &buf[offset..offset + 8],
            &0x0102030405060708u64.to_le_bytes()
        );
        offset += 8;

        // f32 LE (1.0)
        assert_eq!(&buf[offset..offset + 4], &1.0f32.to_le_bytes());
    }

    #[test]
    fn writer_align_pads_correctly() {
        let mut w = LittleEndianWriter::new();
        w.write_u8(0xFF);
        assert_eq!(w.pos(), 1);

        // Align to 4 bytes: should pad 3 zero bytes.
        w.align(4);
        assert_eq!(w.pos(), 4);
        assert_eq!(w.as_slice(), &[0xFF, 0, 0, 0]);

        // Already aligned – no change.
        w.align(4);
        assert_eq!(w.pos(), 4);

        // Align to 8 from pos 4 – should pad 4 zero bytes.
        w.align(8);
        assert_eq!(w.pos(), 8);

        // Alignment of 1 or 0 is a no-op.
        w.write_u8(0xAA);
        w.align(1);
        assert_eq!(w.pos(), 9);
        w.align(0);
        assert_eq!(w.pos(), 9);
    }

    #[test]
    fn writer_offset_placeholder_fixup() {
        let mut w = LittleEndianWriter::new();

        // Write some data, then a placeholder.
        w.write_magic(b"FRES");          // pos 0..4
        let ph = w.write_offset_placeholder(); // pos 4..8 (placeholder)
        assert_eq!(ph, 4);

        // Write more data – the "target" starts here.
        w.write_u32(0x12345678);         // pos 8..12
        let target = 8;

        // Fixup: relative offset = target_pos - placeholder_pos = 8 - 4 = 4.
        w.fixup_offset(ph, target);

        let buf = w.as_slice();
        let relative = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
        assert_eq!(relative, 4);
    }

    #[test]
    fn writer_fixup_offset_absolute() {
        let mut w = LittleEndianWriter::new();
        let ph = w.write_offset_placeholder();
        w.fixup_offset_absolute(ph, 0xCAFEBABE);

        let buf = w.as_slice();
        let val = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        assert_eq!(val, 0xCAFEBABE);
    }

    #[test]
    fn writer_string_and_zeros() {
        let mut w = LittleEndianWriter::new();
        w.write_string("hi");
        // Should be b"hi\0"
        assert_eq!(w.as_slice(), b"hi\0");

        w.write_zeros(3);
        assert_eq!(w.pos(), 6);
        assert_eq!(&w.as_slice()[3..], &[0, 0, 0]);
    }

    #[test]
    fn writer_with_capacity() {
        let w = LittleEndianWriter::with_capacity(1024);
        assert_eq!(w.pos(), 0);
        assert!(w.as_slice().is_empty());
    }
}
