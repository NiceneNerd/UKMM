//! Wii U FTEX (texture) sub-file parser.
//!
//! Each FTEX entry contains a GX2Surface header followed by references to
//! surface data and mipmap data stored elsewhere in the BFRES file.
//!
//! The raw pixel data is tiled/swizzled in GX2 format and is stored as-is;
//! deswizzling happens in a separate conversion step.

use crate::binary::BigEndianReader;
use crate::error::{BfresError, Result};
use crate::model::TextureInfo;

/// Read a relative offset field: reads a u32, then converts to an absolute
/// position by adding the field's own position. Returns 0 if the raw value is 0.
fn read_rel_offset(reader: &mut BigEndianReader) -> Result<u32> {
    let field_pos = reader.pos();
    let rel = reader.read_u32()?;
    if rel == 0 {
        Ok(0)
    } else {
        Ok((field_pos as u32).wrapping_add(rel))
    }
}

/// Parse a single FTEX texture sub-file at the given absolute offset.
///
/// Field order follows the C# BfresLibrary `Texture.Load()` method exactly:
///
/// ```text
/// FTEX magic (4 bytes)
/// GX2Surface:
///   Dim           u32
///   Width         u32
///   Height        u32
///   Depth         u32
///   MipCount      u32
///   Format        u32   (GX2SurfaceFormat)
///   AAMode        u32
///   Use           u32   (GX2SurfaceUse)
///   sizData       u32
///   imagePointer  u32
///   sizMipData    u32
///   mipPointer    u32
///   TileMode      u32
///   Swizzle       u32
///   Alignment     u32
///   Pitch         u32
///   MipOffsets    13x u32
/// ViewMipFirst    u32
/// ViewMipCount    u32
/// ViewSliceFirst  u32
/// ViewSliceCount  u32
/// CompSelR        u8   (GX2CompSel, byte-sized enum)
/// CompSelG        u8
/// CompSelB        u8
/// CompSelA        u8
/// Regs            5x u32
/// Handle          u32
/// ArrayLength     u8
/// (3 bytes padding)
/// Name            relative offset -> null-terminated string
/// Path            relative offset -> null-terminated string
/// Data offset     relative offset -> sizData bytes of surface data
/// MipData offset  relative offset -> sizMipData bytes of mip data
/// UserData dict   relative offset (skipped)
/// numUserData     u16
/// (2 bytes padding)
/// ```
pub fn parse_texture(reader: &mut BigEndianReader, offset: usize) -> Result<TextureInfo> {
    reader.seek(offset);

    // -- FTEX magic --
    let magic = reader.read_magic()?;
    if magic != *b"FTEX" {
        return Err(BfresError::InvalidMagic(magic));
    }

    // -- GX2Surface fields --
    let dim = reader.read_u32()?;
    let width = reader.read_u32()?;
    let height = reader.read_u32()?;
    let depth = reader.read_u32()?;
    let mip_count = reader.read_u32()?;
    let format = reader.read_u32()?;
    let aa_mode = reader.read_u32()?;
    let use_flags = reader.read_u32()?;
    let siz_data = reader.read_u32()?;
    let _image_pointer = reader.read_u32()?;
    let siz_mip_data = reader.read_u32()?;
    let _mip_pointer = reader.read_u32()?;
    let tile_mode = reader.read_u32()?;
    let swizzle = reader.read_u32()?;
    let alignment = reader.read_u32()?;
    let pitch = reader.read_u32()?;

    // 13 mip-level offsets within the mip data buffer
    let mut mip_offsets = Vec::with_capacity(13);
    for _ in 0..13 {
        mip_offsets.push(reader.read_u32()?);
    }

    // View parameters (not stored in TextureInfo but must be read to advance)
    let _view_mip_first = reader.read_u32()?;
    let _view_mip_count = reader.read_u32()?;
    let _view_slice_first = reader.read_u32()?;
    let _view_slice_count = reader.read_u32()?;

    // Component selectors (GX2CompSel is a byte-sized enum: 0=R, 1=G, 2=B, 3=A, 4=0, 5=1)
    let comp_r = reader.read_u8()?;
    let comp_g = reader.read_u8()?;
    let comp_b = reader.read_u8()?;
    let comp_a = reader.read_u8()?;

    // Hardware registers (5x u32) — not stored, just skip
    for _ in 0..5 {
        let _reg = reader.read_u32()?;
    }

    // Handle (runtime pointer, always 0 on disk)
    let _handle = reader.read_u32()?;

    // Array length stored as a single byte, followed by 3 bytes padding
    let array_length = reader.read_u8()? as u32;
    reader.skip(3);

    // -- Name and Path (LoadString = relative offset -> null-terminated string) --
    let name_offset = read_rel_offset(reader)?;
    let name = if name_offset > 0 {
        reader.read_string_at(name_offset as usize).unwrap_or_default()
    } else {
        String::new()
    };

    let path_offset = read_rel_offset(reader)?;
    let path = if path_offset > 0 {
        reader.read_string_at(path_offset as usize).unwrap_or_default()
    } else {
        String::new()
    };

    // -- Surface data (LoadCustom: reads a relative offset, then seeks and reads bytes) --
    let data_abs = read_rel_offset(reader)?;
    let surface_data = if data_abs > 0 && siz_data > 0 {
        let saved = reader.pos();
        reader.seek(data_abs as usize);
        let bytes = reader.read_bytes(siz_data as usize)?.to_vec();
        reader.seek(saved);
        bytes
    } else {
        Vec::new()
    };

    // -- Mip data (same pattern) --
    let mip_abs = read_rel_offset(reader)?;
    let extra_mip_data = if mip_abs > 0 && siz_mip_data > 0 {
        let saved = reader.pos();
        reader.seek(mip_abs as usize);
        let bytes = reader.read_bytes(siz_mip_data as usize)?.to_vec();
        reader.seek(saved);
        bytes
    } else {
        Vec::new()
    };

    // -- UserData dict offset (skip), count (u16), 2 bytes padding --
    // We don't parse user data for textures in this pass.
    let _user_data_offset = read_rel_offset(reader)?;
    let _num_user_data = reader.read_u16()?;
    reader.skip(2);

    Ok(TextureInfo {
        name,
        path,
        width,
        height,
        depth,
        mip_count,
        format,
        dim,
        tile_mode,
        swizzle,
        array_length,
        pitch,
        channel_selectors: [comp_r, comp_g, comp_b, comp_a],
        aa_mode,
        use_flags,
        surface_data,
        extra_mip_data,
        mip_offsets,
        mip_swizzle: 0,
        alignment,
        image_size: siz_data,
        mip_size: siz_mip_data,
        mip_data: Vec::new(), // filled during deswizzle/conversion
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binary::BigEndianReader;
    use crate::wiiu::{dict, header};

    #[test]
    fn parse_tex1_textures() {
        let data = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/Animal_Boar_Big.Tex1.wiiu.bfres"
        ))
        .expect("fixture file should exist");

        let mut reader = BigEndianReader::new(&data);
        let hdr = header::parse_header(&mut reader).expect("header should parse");

        assert!(
            hdr.texture_dict_offset > 0,
            "Tex1 file should have a texture dict"
        );

        let entries = dict::parse_dict(&mut reader, hdr.texture_dict_offset as usize)
            .expect("texture dict should parse");
        assert!(
            !entries.is_empty(),
            "should have at least 1 texture entry in the dict"
        );

        for entry in &entries {
            let tex = parse_texture(&mut reader, entry.data_offset)
                .expect("texture should parse");

            // Basic sanity checks
            assert!(tex.width > 0, "texture width should be nonzero");
            assert!(tex.height > 0, "texture height should be nonzero");
            assert!(
                !tex.surface_data.is_empty(),
                "texture should have non-empty surface_data"
            );
            assert!(
                tex.format != 0,
                "texture format should be a known GX2 format (nonzero)"
            );
            assert!(
                !tex.name.is_empty(),
                "texture name should not be empty"
            );

            eprintln!(
                "Texture: name={}, {}x{}, fmt={:#x}, tile_mode={}, mip_count={}, \
                 data_size={}, mip_data_size={}, selectors={:?}",
                tex.name,
                tex.width,
                tex.height,
                tex.format,
                tex.tile_mode,
                tex.mip_count,
                tex.surface_data.len(),
                tex.extra_mip_data.len(),
                tex.channel_selectors,
            );
        }
    }
}
