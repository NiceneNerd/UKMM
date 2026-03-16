//! BFRES platform converter for Breath of the Wild.
//!
//! Converts Wii U BFRES (v0.3, big-endian) to Switch BFRES (v0.5, little-endian).
//! This is a clean-room Rust implementation based on the BFRES format specification.

pub mod error;
pub mod model;
pub mod wiiu;
pub mod switch;
pub mod convert;
mod binary;
mod gx2;
mod tegra;

pub use error::BfresError;

/// Convert a Wii U BFRES file to Switch format.
///
/// Parses the big-endian Wii U BFRES, converts materials (RenderState →
/// RenderInfo), renames animation suffixes, and writes a little-endian
/// Switch BFRES binary.
///
/// Returns the Switch BFRES bytes, or an error if parsing/conversion fails.
pub fn convert_wiiu_to_switch(data: &[u8]) -> error::Result<Vec<u8>> {
    let mut bfres = wiiu::parse(data)?;
    convert::convert_to_switch(&mut bfres);
    switch::write(&bfres)
}

/// Check if data is a Wii U BFRES file (big-endian, v0.3 format).
pub fn is_wiiu_bfres(data: &[u8]) -> bool {
    if data.len() < 12 {
        return false;
    }
    // Magic "FRES" + version bytes (not spaces) + BOM 0xFEFF (big-endian)
    data[0..4] == *b"FRES"
        && data[4..8] != *b"    "
        && data[8] == 0xFE
        && data[9] == 0xFF
}

/// Check if data is a Switch BFRES file (little-endian, v0.5 format).
pub fn is_switch_bfres(data: &[u8]) -> bool {
    if data.len() < 16 {
        return false;
    }
    // Magic "FRES" + padding spaces + BOM at different offset
    data[0..4] == *b"FRES" && data[4..8] == *b"    "
}
