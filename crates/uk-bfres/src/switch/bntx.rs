//! BNTX (Binary NX Texture) container writer.
//!
//! Builds a BNTX binary from converted texture data. The BNTX container
//! holds all textures in a single file with its own string table, dict,
//! and relocation table.
//!
//! # Binary layout
//!
//! ```text
//! BNTX Header (0x20 bytes)
//! NX Header (0x24 bytes)
//! Memory Pool (0x150 bytes of zeros)
//! Texture Info Pointer Array (8 bytes per texture)
//! _STR String Table
//! _DIC Texture Dict
//! BRTI entries (one per texture, 0xA0 bytes each, plus mip offset arrays)
//! BRTD Header (0x10 bytes)
//! Texture pixel data (all textures concatenated, aligned)
//! _RLT Relocation Table
//! ```
//!
//! All i64 pointer fields in the BNTX store offsets relative to the start
//! of the BNTX container itself (not relative to the field position). The
//! relocation table marks these so the runtime loader can fix them up.

use crate::binary::LittleEndianWriter;
use crate::model::TextureInfo;
use crate::convert::{map_gx2_to_switch_format, map_channel_selector};
use crate::gx2;
use crate::tegra;

/// Converted texture data ready for BNTX packaging.
#[allow(dead_code)]
struct ConvertedTexture {
    /// Texture name.
    name: String,
    /// Switch surface format.
    format: u32,
    /// Width in pixels.
    width: u32,
    /// Height in pixels.
    height: u32,
    /// Depth (usually 1).
    depth: u32,
    /// Number of mip levels.
    mip_count: u32,
    /// Array length.
    array_length: u32,
    /// Block height log2 for TegraX1 swizzling.
    block_height_log2: u32,
    /// Channel selectors mapped to Switch convention.
    channel_types: [u8; 4],
    /// Swizzled texture data (all mip levels concatenated).
    swizzled_data: Vec<u8>,
    /// Byte offset of each mip level within `swizzled_data`.
    mip_offsets: Vec<u64>,
    /// Total image size (= swizzled_data.len()).
    image_size: u32,
    /// Alignment.
    alignment: u32,
    /// Texture layout value (block_height_log2).
    texture_layout: u32,
}

/// Memory pool size in the BNTX (filled with zeros).
const MEMORY_POOL_SIZE: usize = 0x150;

/// BNTX alignment for texture data.
const TEXTURE_DATA_ALIGNMENT: usize = 0x1000;

/// Convert a single Wii U texture to Switch format.
///
/// Deswizzles the GX2 tiled data, maps the format and channels, then
/// re-swizzles to TegraX1 block-linear layout.
fn convert_texture(tex: &TextureInfo) -> ConvertedTexture {
    let switch_format = map_gx2_to_switch_format(tex.format);
    let fmt_info = tegra::get_format_info(switch_format);

    let channel_types = [
        map_channel_selector(tex.channel_selectors[0]),
        map_channel_selector(tex.channel_selectors[1]),
        map_channel_selector(tex.channel_selectors[2]),
        map_channel_selector(tex.channel_selectors[3]),
    ];

    // Compute GX2 bpp for deswizzle
    let gx2_bpp = gx2::get_format_bpp(tex.format);

    // Deswizzle all mip levels from GX2 tiled layout to linear
    let mut linear_mip_data: Vec<Vec<u8>> = Vec::new();

    // Deswizzle base mip level from surface_data
    if !tex.surface_data.is_empty() {
        let linear = gx2::deswizzle(
            tex.width,
            tex.height,
            tex.depth,
            tex.format,
            tex.aa_mode,
            tex.use_flags,
            tex.tile_mode,
            tex.swizzle,
            tex.pitch,
            gx2_bpp,
            0, // slice
            0, // sample
            &tex.surface_data,
        );
        linear_mip_data.push(linear);
    }

    // Deswizzle additional mip levels from extra_mip_data
    if tex.mip_count > 1 && !tex.extra_mip_data.is_empty() {
        for level in 1..tex.mip_count {
            let mip_width = (tex.width >> level).max(1);
            let mip_height = (tex.height >> level).max(1);

            // Calculate pitch for this mip level
            let mip_pitch = if gx2::is_bcn_format(tex.format) {
                ((mip_width + 3) / 4).max(1)
            } else {
                mip_width
            };

            // Get the mip offset within extra_mip_data
            let mip_offset = if level >= 2 && (level - 1) < tex.mip_offsets.len() as u32 {
                tex.mip_offsets[(level - 1) as usize] as usize
            } else if level == 1 {
                0
            } else {
                continue;
            };

            // Use mip swizzle if available, otherwise derive from base swizzle
            let mip_swizzle = if tex.mip_swizzle != 0 {
                tex.mip_swizzle
            } else {
                tex.swizzle
            };

            if mip_offset < tex.extra_mip_data.len() {
                let mip_data = &tex.extra_mip_data[mip_offset..];
                let linear = gx2::deswizzle(
                    mip_width,
                    mip_height,
                    tex.depth,
                    tex.format,
                    tex.aa_mode,
                    tex.use_flags,
                    tex.tile_mode,
                    mip_swizzle,
                    mip_pitch,
                    gx2_bpp,
                    0,
                    0,
                    mip_data,
                );
                linear_mip_data.push(linear);
            }
        }
    }

    // If we got no mip data at all, create a minimal placeholder
    if linear_mip_data.is_empty() {
        let size = if gx2::is_bcn_format(tex.format) {
            let bw = ((tex.width + 3) / 4).max(1);
            let bh = ((tex.height + 3) / 4).max(1);
            (bw * bh * (gx2_bpp / 8)) as usize
        } else {
            (tex.width * tex.height * (gx2_bpp / 8)) as usize
        };
        linear_mip_data.push(vec![0u8; size]);
    }

    // Concatenate all linear mip levels
    let total_linear_size: usize = linear_mip_data.iter().map(|m| m.len()).sum();
    let mut all_linear = Vec::with_capacity(total_linear_size);
    for mip in &linear_mip_data {
        all_linear.extend_from_slice(mip);
    }

    // Calculate block height for TegraX1 swizzling
    let effective_height = if fmt_info.block_height > 1 {
        (tex.height + fmt_info.block_height - 1) / fmt_info.block_height
    } else {
        tex.height
    };
    let block_height_log2 = tegra::get_block_height_log2(effective_height);

    let mip_count = linear_mip_data.len() as u32;

    // Re-swizzle all mip levels to TegraX1 block-linear
    let (swizzled_data, mip_offsets) = tegra::swizzle_mip_maps(
        tex.width,
        tex.height,
        tex.depth,
        fmt_info.block_width,
        fmt_info.block_height,
        1, // block_depth
        fmt_info.bytes_per_pixel,
        1, // tile_mode = block-linear
        block_height_log2,
        mip_count,
        &all_linear,
    );

    let image_size = swizzled_data.len() as u32;

    ConvertedTexture {
        name: tex.name.clone(),
        format: switch_format,
        width: tex.width,
        height: tex.height,
        depth: tex.depth,
        mip_count,
        array_length: tex.array_length.max(1),
        block_height_log2,
        channel_types,
        swizzled_data,
        mip_offsets,
        image_size,
        alignment: 512,
        texture_layout: block_height_log2,
    }
}

/// Build a complete BNTX binary container from the given textures.
///
/// The textures should be the raw Wii U `TextureInfo` entries; this function
/// handles the full conversion pipeline (deswizzle, format map, re-swizzle)
/// internally.
pub fn build_bntx(textures: &[TextureInfo]) -> Vec<u8> {
    if textures.is_empty() {
        return Vec::new();
    }

    // Phase 0: Convert all textures
    let converted: Vec<ConvertedTexture> = textures.iter().map(convert_texture).collect();
    let tex_count = converted.len();

    // Collect all strings needed
    let mut all_names: Vec<&str> = converted.iter().map(|t| t.name.as_str()).collect();
    all_names.sort();

    let mut w = LittleEndianWriter::with_capacity(128 * 1024);

    // Track positions of all i64 pointer fields for the relocation table.
    // Each entry is (section, position) where section is 0 for main, 1 for tex data.
    let mut reloc_entries: Vec<(u8, u32, u8, u16, u8)> = Vec::new();
    // (section, position, offset_count, struct_count, padding_count)

    // -----------------------------------------------------------------------
    // Phase 1: BNTX Header (0x20 bytes)
    // -----------------------------------------------------------------------
    w.write_magic(b"BNTX");          // +0x00
    w.write_u32(0);                  // +0x04 padding
    w.write_u32(0x00040000);         // +0x08 version
    w.write_u16(0xFEFF);             // +0x0C BOM (little-endian)
    w.write_u8(0x0C);               // +0x0E alignment (log2(4096))
    w.write_u8(0x40);               // +0x0F target_addr_size (64-bit)
    let file_name_offset_pos = w.pos();
    w.write_u32(0);                  // +0x10 file_name_offset (patched later)
    w.write_u16(0);                  // +0x14 flag
    let block_offset_pos = w.pos();
    w.write_u16(0);                  // +0x16 block_offset (patched later)
    let reloc_table_offset_pos = w.pos();
    w.write_u32(0);                  // +0x18 relocation_table_offset (patched later)
    let file_size_pos = w.pos();
    w.write_u32(0);                  // +0x1C file_size (patched later)

    // -----------------------------------------------------------------------
    // Phase 2: NX Header (0x24 bytes, starting at +0x20)
    // -----------------------------------------------------------------------
    let nx_header_off = w.pos();
    w.write_magic(b"NX  ");          // +0x00 magic
    w.write_u32(tex_count as u32);   // +0x04 texture_count

    // i64 offsets (BNTX-relative, will be relocated)
    let nx_tex_info_arr_off_pos = w.pos();
    w.write_u64(0);                  // +0x08 texture_info_array_offset
    reloc_entries.push((0, nx_tex_info_arr_off_pos as u32, 1, 2, 1));
    // struct_count=2 means: this field and the next pointer field share a pattern
    // Actually the NX header has: ptr, non-ptr, ptr, non-ptr pattern
    // Let me handle individually

    let nx_tex_data_off_pos = w.pos();
    w.write_u64(0);                  // +0x10 texture_data_offset

    let nx_tex_dict_off_pos = w.pos();
    w.write_u64(0);                  // +0x18 texture_dict_offset

    w.write_u32(0);                  // +0x20 memory_pool_info / str_dict_size
    w.write_u32(0);                  // padding to round NX header

    // -----------------------------------------------------------------------
    // Phase 3: Memory Pool (0x150 bytes of zeros)
    // -----------------------------------------------------------------------
    w.write_zeros(MEMORY_POOL_SIZE);

    // -----------------------------------------------------------------------
    // Phase 4: Texture Info Pointer Array
    // -----------------------------------------------------------------------
    let tex_info_arr_start = w.pos();
    // Write placeholders for each texture pointer (BNTX-relative offsets to BRTI)
    let mut tex_ptr_positions: Vec<usize> = Vec::new();
    for _ in 0..tex_count {
        let pos = w.pos();
        tex_ptr_positions.push(pos);
        w.write_u64(0);
    }

    // -----------------------------------------------------------------------
    // Phase 5: String Table (_STR)
    // -----------------------------------------------------------------------
    let str_block_start = w.pos();
    w.write_magic(b"_STR");          // magic
    let str_block_header_off_pos = w.pos();
    w.write_u32(0);                  // block header offset (patched)
    let str_block_size_pos = w.pos();
    w.write_u32(0);                  // block size (patched)
    w.write_u32(0);                  // padding

    let str_entries_start = w.pos();
    w.set_u32_at(str_block_header_off_pos, (str_entries_start - str_block_start) as u32);

    // Write string count as u32 first, then entries
    // BNTX _STR format: u32 string_count, then per entry: u16 len + bytes + null + pad
    w.write_u32(all_names.len() as u32 + 1); // +1 for "textures"

    // Build string position map (maps string text to absolute position of u16 len prefix)
    let mut string_positions: Vec<(String, usize)> = Vec::new();

    // Write texture name entries (sorted)
    for name in &all_names {
        let entry_pos = w.pos();
        let bytes = name.as_bytes();
        w.write_u16(bytes.len() as u16);
        w.write_bytes(bytes);
        w.write_u8(0); // null terminator
        if (bytes.len() + 1) % 2 != 0 {
            w.write_u8(0); // align to 2
        }
        string_positions.push((name.to_string(), entry_pos));
    }

    // Write "textures" entry (the BNTX file name)
    let textures_entry_pos = w.pos();
    let textures_bytes = b"textures";
    w.write_u16(textures_bytes.len() as u16);
    w.write_bytes(textures_bytes);
    w.write_u8(0);
    if (textures_bytes.len() + 1) % 2 != 0 {
        w.write_u8(0);
    }
    string_positions.push(("textures".to_string(), textures_entry_pos));

    // Pad to alignment
    w.align(4);

    let str_block_size = (w.pos() - str_entries_start) as u32;
    w.set_u32_at(str_block_size_pos, str_block_size);

    // Helper to find string position (returns BNTX-relative offset to u16 len prefix)
    let bntx_start = 0usize; // BNTX starts at position 0 in our writer
    let find_string_pos = |name: &str| -> u64 {
        for (s, pos) in &string_positions {
            if s == name {
                return (*pos - bntx_start) as u64;
            }
        }
        0
    };

    // Fix up BNTX header file_name_offset
    // This offset points to the TEXT of "textures" (past u16 len prefix)
    let textures_text_pos = textures_entry_pos + 2; // past u16 len
    w.set_u32_at(file_name_offset_pos, (textures_text_pos - bntx_start) as u32);

    // -----------------------------------------------------------------------
    // Phase 6: Texture Dict (_DIC)
    // -----------------------------------------------------------------------
    w.align(4);
    let dict_start = w.pos();

    // Build a simple Patricia trie dict
    // _DIC header: magic(4) + num_entries(4)
    w.write_magic(b"_DIC");
    w.write_u32(tex_count as u32);

    // Build dict nodes using the existing DictBuilder logic
    // We need to create dict nodes with BNTX-relative key offsets
    // For simplicity, build nodes using the same trie algorithm as the BFRES dict

    // The dict keys should be ordered by the texture names
    // Build trie nodes the same way as dict.rs but with "_DIC" header
    // and BNTX-relative key offsets instead of field-relative

    // Root sentinel + entry nodes
    // For now, use a simplified approach: build the trie with the same algorithm
    let dict_node_positions = write_bntx_dict(&mut w, &all_names, &string_positions, bntx_start);

    // -----------------------------------------------------------------------
    // Phase 7: BRTI entries (one per texture)
    // -----------------------------------------------------------------------
    // BRTI entries are written in the order they appear in all_names (sorted)
    // We need to map from sorted names to converted textures
    let mut brti_starts: Vec<usize> = Vec::new();
    let mut brti_mip_arr_positions: Vec<usize> = Vec::new();
    let mut brti_tex_ptr_positions: Vec<usize> = Vec::new();
    let mut brti_tex_view_positions: Vec<usize> = Vec::new();
    let mut brti_name_off_positions: Vec<usize> = Vec::new();
    let mut brti_parent_off_positions: Vec<usize> = Vec::new();
    let mut brti_mip_ptr_positions: Vec<usize> = Vec::new();

    // Build index mapping: sorted name -> original texture index
    let name_to_tex: Vec<usize> = all_names.iter().map(|n| {
        converted.iter().position(|t| t.name == *n).unwrap()
    }).collect();

    for (_sorted_idx, &tex_idx) in name_to_tex.iter().enumerate() {
        let tex = &converted[tex_idx];

        w.align(8);
        let brti_start = w.pos();
        brti_starts.push(brti_start);

        // BRTI header
        let brti_size: u32 = 0xA0; // fixed size for the BRTI header block
        w.write_magic(b"BRTI");      // +0x00
        w.write_u32(brti_size);      // +0x04 size (u32)
        w.write_u64(brti_size as u64); // +0x08 size (u64)

        // Texture properties
        w.write_u8(0x01);            // +0x10 flags (tile mode = 1)
        w.write_u8(0x02);            // +0x11 dim (2D)
        w.write_u16(0);              // +0x12 tile_mode
        w.write_u16(0);              // +0x14 swizzle
        w.write_u16(tex.mip_count as u16); // +0x16 mip_count

        w.write_u32(1);              // +0x18 sample_count
        w.write_u32(tex.format);     // +0x1C format
        w.write_u32(0x20);           // +0x20 access_flags (texture)
        w.write_u32(tex.width);      // +0x24 width
        w.write_u32(tex.height);     // +0x28 height
        w.write_u32(tex.depth);      // +0x2C depth
        w.write_u32(tex.array_length); // +0x30 array_length
        w.write_u32(tex.texture_layout); // +0x34 texture_layout (block_height_log2)
        w.write_u32(0x00010007);     // +0x38 texture_layout2

        // Reserved (20 bytes)
        w.write_zeros(20);           // +0x3C reserved[20] -> through +0x4F

        w.write_u32(tex.image_size); // +0x50 image_size
        w.write_u32(tex.alignment);  // +0x54 alignment

        // Channel types
        for &ch in &tex.channel_types {
            w.write_u8(ch);          // +0x58..+0x5B
        }

        w.write_u32(0x01);           // +0x5C texture_type (Dim2D = 1)

        // i64 offset fields (BNTX-relative, patched later)
        let name_off_pos = w.pos();
        brti_name_off_positions.push(name_off_pos);
        w.write_u64(0);              // +0x60 name_offset

        let parent_off_pos = w.pos();
        brti_parent_off_positions.push(parent_off_pos);
        w.write_u64(0);              // +0x68 parent_offset (points to NX header)

        let mip_ptr_pos = w.pos();
        brti_mip_ptr_positions.push(mip_ptr_pos);
        w.write_u64(0);              // +0x70 mip_offsets_ptr

        w.write_u64(0);              // +0x78 user_data_ptr (null)

        let tex_ptr_pos = w.pos();
        brti_tex_ptr_positions.push(tex_ptr_pos);
        w.write_u64(0);              // +0x80 tex_ptr (runtime)

        let tex_view_pos = w.pos();
        brti_tex_view_positions.push(tex_view_pos);
        w.write_u64(0);              // +0x88 tex_view (runtime)

        w.write_u32(0);              // +0x90 reserved/padding
        w.write_u32(0);              // +0x94 padding
        w.write_u64(0);              // +0x98 user_data_dict_ptr (null)

        assert_eq!(w.pos() - brti_start, brti_size as usize,
            "BRTI size mismatch: expected {}, got {}", brti_size, w.pos() - brti_start);
    }

    // Write mip offset arrays (one per texture, after all BRTI entries)
    for (_sorted_idx, &tex_idx) in name_to_tex.iter().enumerate() {
        let tex = &converted[tex_idx];

        w.align(8);
        let mip_arr_start = w.pos();
        brti_mip_arr_positions.push(mip_arr_start);

        // Mip offsets will be patched to be BNTX-relative once we know
        // where the texture data lands
        for mip_off in &tex.mip_offsets {
            w.write_u64(*mip_off); // placeholder, patched in Phase 9
        }
    }

    // -----------------------------------------------------------------------
    // Phase 8: Fix up all BNTX-relative offsets in headers
    // -----------------------------------------------------------------------

    // Fix up NX header offsets
    set_bntx_offset(&mut w, nx_tex_info_arr_off_pos, tex_info_arr_start, bntx_start);
    set_bntx_offset(&mut w, nx_tex_dict_off_pos, dict_start, bntx_start);

    // Fix up texture info pointer array entries
    for (sorted_idx, brti_start) in brti_starts.iter().enumerate() {
        set_bntx_offset(&mut w, tex_ptr_positions[sorted_idx], *brti_start, bntx_start);
    }

    // Fix up BRTI name offsets (point to string entry u16 len prefix)
    for (sorted_idx, _) in name_to_tex.iter().enumerate() {
        let name = all_names[sorted_idx];
        let str_pos = find_string_pos(name);
        set_bntx_offset_raw(&mut w, brti_name_off_positions[sorted_idx], str_pos);
    }

    // Fix up BRTI parent offsets (point to NX header)
    for pos in &brti_parent_off_positions {
        set_bntx_offset(&mut w, *pos, nx_header_off, bntx_start);
    }

    // Fix up BRTI mip_offsets_ptr
    for (sorted_idx, mip_arr_pos) in brti_mip_arr_positions.iter().enumerate() {
        set_bntx_offset(&mut w, brti_mip_ptr_positions[sorted_idx], *mip_arr_pos, bntx_start);
    }

    // Fix up block_offset (points to NX header relative to BNTX start)
    w.set_u16_at(block_offset_pos, (nx_header_off - bntx_start) as u16);

    // -----------------------------------------------------------------------
    // Phase 9: BRTD (texture data block)
    // -----------------------------------------------------------------------
    // Align to texture data alignment (typically 0x1000 = 4096)
    let pre_align = w.pos();
    let aligned_pos = (pre_align + TEXTURE_DATA_ALIGNMENT - 1) & !(TEXTURE_DATA_ALIGNMENT - 1);
    w.write_zeros(aligned_pos - pre_align);

    // BRTD header
    let brtd_start = w.pos();
    w.write_magic(b"BRTD");
    w.write_u32(0); // padding

    // Calculate total texture data size and write the size field
    let total_tex_data_size: usize = {
        let mut total = 0usize;
        for (sorted_idx, &tex_idx) in name_to_tex.iter().enumerate() {
            if sorted_idx > 0 {
                total = (total + 511) & !511; // align each texture to 512
            }
            total += converted[tex_idx].swizzled_data.len();
        }
        total
    };

    // BRTD size field includes the BRTD header (0x10) + all texture data
    let brtd_total = 0x10 + total_tex_data_size;
    w.write_u64(brtd_total as u64);

    // Write texture data
    let _tex_data_start = w.pos();
    let mut tex_data_offsets: Vec<usize> = Vec::new();

    for (sorted_idx, &tex_idx) in name_to_tex.iter().enumerate() {
        let tex = &converted[tex_idx];
        if sorted_idx > 0 {
            // Align to 512 between textures
            let cur = w.pos();
            let aligned = (cur + 511) & !511;
            if aligned > cur {
                w.write_zeros(aligned - cur);
            }
        }
        let data_offset = w.pos();
        tex_data_offsets.push(data_offset);
        w.write_bytes(&tex.swizzled_data);
    }

    // Patch mip offset arrays with BNTX-relative offsets to actual texture data
    for (sorted_idx, &tex_idx) in name_to_tex.iter().enumerate() {
        let tex = &converted[tex_idx];
        let base_data_offset = tex_data_offsets[sorted_idx] - bntx_start;
        let mip_arr_start = brti_mip_arr_positions[sorted_idx];

        for (mip_idx, &mip_off) in tex.mip_offsets.iter().enumerate() {
            let abs_mip_offset = base_data_offset as u64 + mip_off;
            let field_pos = mip_arr_start + mip_idx * 8;
            set_bntx_offset_raw(&mut w, field_pos, abs_mip_offset);
        }
    }

    // Fix up NX texture_data_offset (points to BRTD section start as BNTX-relative)
    set_bntx_offset(&mut w, nx_tex_data_off_pos, brtd_start, bntx_start);

    // Fix up BRTI tex_ptr and tex_view (they point into the runtime descriptor area,
    // just store zeros for now as they're runtime-only)
    // Actually, looking at the reference, these seem to point to allocated areas
    // within the BRTI padding/reserved section. For a fresh build, leave as 0.

    // -----------------------------------------------------------------------
    // Phase 10: Build relocation table
    // -----------------------------------------------------------------------
    w.align(8);
    let reloc_start = w.pos();

    // Build the relocation table entries
    // Section 0: main data (from 0 to brtd_start)
    // Section 1: texture data (from brtd_start to reloc_start)

    let mut sec0_entries: Vec<(u32, u8, u16, u8)> = Vec::new();
    let mut sec1_entries: Vec<(u32, u8, u16, u8)> = Vec::new();

    // NX header pointer fields at nx_header_off + 8, +0x10, +0x18
    // These are 3 consecutive i64 fields with 0 non-pointer fields between them
    // Actually: +0x08 = tex_info_arr, +0x10 = tex_data, +0x18 = tex_dict
    // But between them there are NO non-pointer fields, they're consecutive
    // So: pos=nx_header_off+8, offset_count=2, struct_count=1, padding_count=1
    // (2 consecutive pointers, then 1 non-pointer, repeat 1 time)
    // Actually looking at reference: entry at pos=0x28 has struct_count=2, offset_count=1, padding_count=1
    // That means: at 0x28, there's 1 pointer, then 1 non-pointer (8 bytes each), repeated 2 times
    // 0x28 = NX+0x08 -> tex_info_arr (pointer)
    // 0x30 = NX+0x10 -> tex_data (non-pointer?!) wait...
    // Actually struct_count=2 means the PATTERN repeats 2 times:
    // offset 0x28: 1 pointer (tex_info_arr_off)
    // offset 0x30: 1 non-ptr (tex_data - but this IS a pointer)
    // offset 0x38: 1 pointer (tex_dict_off)
    // offset 0x40: 1 non-ptr (str_dict_size)
    // Hmm, the tex_data_off at 0x30 is section 1, so it's in sec1_entries!

    // NX header: tex_info_arr at NX+0x08 and tex_dict at NX+0x18
    // These are in section 0 (pointing to section 0 data)
    sec0_entries.push(((nx_tex_info_arr_off_pos - bntx_start) as u32, 1, 2, 1));

    // tex_data at NX+0x10 - this points to section 1 (texture data)
    // In the reference, this is in section 1 entries
    sec1_entries.push(((nx_tex_data_off_pos - bntx_start) as u32, 1, 1, 0));

    // File name offset at 0x10 is a u32, not an i64, so it's NOT relocated
    // (the loader handles it differently)

    // Texture info pointer array: 6 consecutive pointers
    sec0_entries.push(((tex_info_arr_start - bntx_start) as u32,
                        tex_count as u8, 1, 0));

    // Dict nodes: each has 1 pointer (key_offset) after 8 bytes of non-pointer data
    // Root + entries = tex_count + 1 nodes
    // First node at dict_start + 8 (past _DIC header)
    // Each node: reference(4) + left(2) + right(2) = 8 bytes non-ptr, then key_offset(8) = ptr
    // Pattern: 1 pointer, preceded by 8 bytes of non-pointer
    // In reloc terms: offset_count=1, padding_count=1 (1 non-ptr 8-byte field before each ptr)
    // struct_count = tex_count + 1 (root + entries)
    let dict_first_node = dict_node_positions;
    sec0_entries.push(((dict_first_node - bntx_start) as u32 + 8, // +8 to skip ref/left/right to get to key_offset
                        1, (tex_count + 1) as u16, 1));

    // BRTI entries: each has pointers at specific offsets
    // Looking at reference reloc: each BRTI has entries like:
    // pos=0x320 struct_count=1 offset_count=3 padding_count=0
    // That's 3 consecutive pointers: name_offset, parent_offset, mip_offsets_ptr at +0x60, +0x68, +0x70
    // Then: pos=0x338 offset_count=1 -> user_data_ptr at +0x78 (but it's null...)
    // Actually user_data is null but the reloc table still marks it
    // Then: pos=0x340 offset_count=2 -> tex_ptr, tex_view at +0x80, +0x88
    // Then: pos=0x358 offset_count=1 -> user_data_dict_ptr at +0x98 (past desc_slot at +0x90..+0x97)

    for sorted_idx in 0..tex_count {
        let brti_start = brti_starts[sorted_idx];
        let base = (brti_start - bntx_start) as u32;

        // +0x60: name_offset, +0x68: parent_offset, +0x70: mip_offsets_ptr (3 consecutive)
        sec0_entries.push((base + 0x60, 3, 1, 0));

        // +0x78: user_data_ptr (1 pointer, even though null)
        sec0_entries.push((base + 0x78, 1, 1, 0));

        // +0x80: tex_ptr, +0x88: tex_view (2 consecutive)
        sec0_entries.push((base + 0x80, 2, 1, 0));

        // +0x98: user_data_dict_ptr (after desc_slot at +0x90 which is 8 bytes)
        sec0_entries.push((base + 0x98, 1, 1, 0));
    }

    // Mip offset arrays: each mip offset is a pointer to texture data (section 1)
    for (sorted_idx, &tex_idx) in name_to_tex.iter().enumerate() {
        let tex = &converted[tex_idx];
        let mip_arr_start = brti_mip_arr_positions[sorted_idx];
        sec1_entries.push(((mip_arr_start - bntx_start) as u32,
                           tex.mip_count as u8, 1, 0));
    }

    // Sort entries by position
    sec0_entries.sort_by_key(|e| e.0);
    sec1_entries.sort_by_key(|e| e.0);

    // Write _RLT
    let section0_end = brtd_start - bntx_start;
    let section1_end = reloc_start - bntx_start;

    w.write_magic(b"_RLT");
    w.write_u32((reloc_start - bntx_start) as u32); // position
    w.write_u32(2);                  // num_sections
    w.write_u32(0);                  // padding

    // Section 0 descriptor
    {
        // pointer field (i64 relative to section start from this field)
        let ptr_field_pos = w.pos();
        let relative = 0i64 - ptr_field_pos as i64 + bntx_start as i64;
        w.write_bytes(&relative.to_le_bytes());
        w.write_u32(0);              // section position
        w.write_u32(section0_end as u32); // section size
        w.write_u32(0);              // entry_index
        w.write_u32(sec0_entries.len() as u32); // num_entries
    }

    // Section 1 descriptor
    {
        let ptr_field_pos = w.pos();
        let sec1_start = brtd_start - bntx_start;
        let relative = sec1_start as i64 - ptr_field_pos as i64 + bntx_start as i64;
        w.write_bytes(&relative.to_le_bytes());
        w.write_u32(sec1_start as u32); // section position
        w.write_u32((section1_end - sec1_start) as u32); // section size
        w.write_u32(sec0_entries.len() as u32); // entry_index
        w.write_u32(sec1_entries.len() as u32); // num_entries
    }

    // Write relocation entries
    for (pos, offset_count, struct_count, padding_count) in &sec0_entries {
        w.write_u32(*pos);
        w.write_u16(*struct_count);
        w.write_u8(*offset_count);
        w.write_u8(*padding_count);
    }

    for (pos, offset_count, struct_count, padding_count) in &sec1_entries {
        w.write_u32(*pos);
        w.write_u16(*struct_count);
        w.write_u8(*offset_count);
        w.write_u8(*padding_count);
    }

    // Fix up BNTX header reloc offset and file size
    let total_size = w.pos();
    w.set_u32_at(reloc_table_offset_pos, (reloc_start - bntx_start) as u32);
    w.set_u32_at(file_size_pos, (total_size - bntx_start) as u32);

    w.into_inner()
}

/// Write BNTX dict nodes using the same Patricia trie algorithm as the BFRES dict,
/// but with "_DIC" magic header and BNTX-relative key offsets.
///
/// Returns the position of the first node (root sentinel).
fn write_bntx_dict(
    w: &mut LittleEndianWriter,
    names: &[&str],
    string_positions: &[(String, usize)],
    bntx_start: usize,
) -> usize {
    // Build trie nodes using the same algorithm as dict.rs
    let nodes = build_dict_nodes_for_bntx(names);

    let first_node_pos = w.pos();

    // Write nodes: root sentinel + entry nodes
    for node in &nodes {
        w.write_u32(node.reference);
        w.write_u16(node.left_index);
        w.write_u16(node.right_index);

        // Key offset: BNTX-relative offset to string entry
        if node.key.is_empty() {
            // Root sentinel - still write the string position if available
            // In BNTX, root sentinel seems to have a non-zero key offset
            // pointing to the last string ("textures")
            // For compatibility, find "textures" position
            let textures_pos = string_positions.iter()
                .find(|(s, _)| s == "textures")
                .map(|(_, pos)| (*pos - bntx_start) as u64)
                .unwrap_or(0);
            w.write_u64(textures_pos);
        } else {
            let str_pos = string_positions.iter()
                .find(|(s, _)| s == &node.key)
                .map(|(_, pos)| (*pos - bntx_start) as u64)
                .unwrap_or(0);
            w.write_u64(str_pos);
        }
    }

    first_node_pos
}

// ---------------------------------------------------------------------------
// Patricia trie construction (duplicated from dict.rs for BNTX use)
// ---------------------------------------------------------------------------

/// Convert a string to a big-endian integer for Patricia trie building.
fn string_to_bigint(s: &str) -> u128 {
    let bytes = s.as_bytes();
    let mut result: u128 = 0;
    for &b in bytes {
        result = (result << 8) | (b as u128);
    }
    result
}

fn bit(n: u128, b: i32) -> usize {
    ((n >> (b as u32 & 0x7F)) & 1) as usize
}

fn first_1bit(n: u128) -> i32 {
    assert_ne!(n, 0, "first_1bit called on zero");
    n.trailing_zeros() as i32
}

fn bit_mismatch(a: u128, b: u128) -> i32 {
    let xor = a ^ b;
    if xor == 0 { return -1; }
    xor.trailing_zeros() as i32
}

fn compact_bit_idx(bit_idx: i32) -> u32 {
    let byte_idx = bit_idx / 8;
    let bit_in_byte = bit_idx - 8 * byte_idx;
    ((byte_idx << 3) | bit_in_byte) as u32
}

struct TrieNode {
    data: u128,
    bit_idx: i32,
    parent: usize,
    child: [usize; 2],
}

struct Trie {
    nodes: Vec<TrieNode>,
    entries: Vec<(u128, usize, String)>,
}

impl Trie {
    fn new() -> Self {
        let root = TrieNode { data: 0, bit_idx: -1, parent: 0, child: [0, 0] };
        let mut trie = Trie { nodes: vec![root], entries: Vec::new() };
        trie.entries.push((0, 0, String::new()));
        trie
    }

    fn entry_index_for_data(&self, data: u128) -> Option<usize> {
        self.entries.iter().position(|(d, _, _)| *d == data)
    }

    fn search(&self, data: u128, want_prev: bool) -> usize {
        if self.nodes[0].child[0] == 0 { return 0; }
        let mut node_idx = self.nodes[0].child[0];
        let mut prev_idx;
        loop {
            prev_idx = node_idx;
            let b = bit(data, self.nodes[node_idx].bit_idx);
            node_idx = self.nodes[node_idx].child[b];
            if self.nodes[node_idx].bit_idx <= self.nodes[prev_idx].bit_idx { break; }
        }
        if want_prev { prev_idx } else { node_idx }
    }

    fn insert(&mut self, name: &str) {
        let data = string_to_bigint(name);
        let original_key = name.to_string();
        let current_idx = self.search(data, true);
        let current_data = self.nodes[current_idx].data;
        let bit_idx = bit_mismatch(current_data, data);
        let mut cur = current_idx;
        while bit_idx < self.nodes[self.nodes[cur].parent].bit_idx {
            cur = self.nodes[cur].parent;
        }
        let cur_bit_idx = self.nodes[cur].bit_idx;

        if bit_idx < cur_bit_idx {
            let parent_idx = self.nodes[cur].parent;
            let parent_bit_idx = self.nodes[parent_idx].bit_idx;
            let new_idx = self.nodes.len();
            let mut new_node = TrieNode { data, bit_idx, parent: parent_idx, child: [new_idx, new_idx] };
            new_node.child[bit(data, bit_idx) ^ 1] = cur;
            self.nodes.push(new_node);
            let parent_child_side = bit(data, parent_bit_idx);
            self.nodes[parent_idx].child[parent_child_side] = new_idx;
            self.nodes[cur].parent = new_idx;
            self.entries.push((data, new_idx, original_key));
        } else if bit_idx > cur_bit_idx {
            let new_idx = self.nodes.len();
            let mut new_node = TrieNode { data, bit_idx, parent: cur, child: [new_idx, new_idx] };
            let cur_data = self.nodes[cur].data;
            let opposite_side = bit(data, bit_idx) ^ 1;
            if bit(cur_data, bit_idx) == opposite_side {
                new_node.child[opposite_side] = cur;
            } else {
                new_node.child[opposite_side] = 0;
            }
            self.nodes.push(new_node);
            let child_side = bit(data, cur_bit_idx);
            self.nodes[cur].child[child_side] = new_idx;
            self.entries.push((data, new_idx, original_key));
        } else {
            let child_side = bit(data, bit_idx);
            let existing_child = self.nodes[cur].child[child_side];
            let new_bit_idx = if existing_child == 0 {
                first_1bit(data)
            } else {
                bit_mismatch(self.nodes[existing_child].data, data)
            };
            let new_idx = self.nodes.len();
            let mut new_node = TrieNode { data, bit_idx: new_bit_idx, parent: cur, child: [new_idx, new_idx] };
            let opposite_side_new = bit(data, new_bit_idx) ^ 1;
            new_node.child[opposite_side_new] = existing_child;
            self.nodes.push(new_node);
            self.nodes[cur].child[child_side] = new_idx;
            self.entries.push((data, new_idx, original_key));
        }
    }
}

struct DictNode {
    reference: u32,
    left_index: u16,
    right_index: u16,
    key: String,
}

fn build_dict_nodes_for_bntx(keys: &[&str]) -> Vec<DictNode> {
    let mut trie = Trie::new();
    for key in keys {
        trie.insert(key);
    }

    let mut result = Vec::with_capacity(trie.entries.len());
    for (_data, node_idx, original_key) in &trie.entries {
        let node = &trie.nodes[*node_idx];
        let reference = compact_bit_idx(node.bit_idx) & 0xFFFFFFFF;
        let left_data = trie.nodes[node.child[0]].data;
        let right_data = trie.nodes[node.child[1]].data;
        let left_index = trie.entry_index_for_data(left_data)
            .expect("left child data not in entries") as u16;
        let right_index = trie.entry_index_for_data(right_data)
            .expect("right child data not in entries") as u16;
        let key = if *_data == 0 { String::new() } else { original_key.clone() };
        result.push(DictNode { reference, left_index, right_index, key });
    }
    if !result.is_empty() {
        result[0].key = String::new();
    }
    result
}

/// Set a BNTX-relative i64 offset at the given field position.
fn set_bntx_offset(w: &mut LittleEndianWriter, field_pos: usize, target: usize, bntx_start: usize) {
    let value = (target - bntx_start) as u64;
    let bytes = value.to_le_bytes();
    w.data_mut()[field_pos..field_pos + 8].copy_from_slice(&bytes);
}

/// Set a raw u64 value at the given field position.
fn set_bntx_offset_raw(w: &mut LittleEndianWriter, field_pos: usize, value: u64) {
    let bytes = value.to_le_bytes();
    w.data_mut()[field_pos..field_pos + 8].copy_from_slice(&bytes);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wiiu;

    #[test]
    fn build_bntx_from_wiiu_textures() {
        let data = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/Animal_Boar_Big.Tex1.wiiu.bfres"
        ))
        .expect("fixture file should exist");

        let bfres = wiiu::parse(&data).expect("should parse Wii U Tex1 BFRES");
        assert_eq!(bfres.textures.len(), 6, "should have 6 textures");

        let bntx = build_bntx(&bfres.textures);
        assert!(!bntx.is_empty(), "BNTX should not be empty");

        // Verify BNTX magic
        assert_eq!(&bntx[0..4], b"BNTX", "should have BNTX magic");

        // Verify version
        let version = u32::from_le_bytes([bntx[8], bntx[9], bntx[10], bntx[11]]);
        assert_eq!(version, 0x00040000, "version should be 0.4");

        // Verify BOM
        let bom = u16::from_le_bytes([bntx[0xC], bntx[0xD]]);
        assert_eq!(bom, 0xFEFF, "BOM should be 0xFEFF");

        // Verify NX header magic at offset 0x20
        assert_eq!(&bntx[0x20..0x24], b"NX  ", "should have NX magic at 0x20");

        // Verify texture count
        let tex_count = u32::from_le_bytes([bntx[0x24], bntx[0x25], bntx[0x26], bntx[0x27]]);
        assert_eq!(tex_count, 6, "should have 6 textures");

        // Verify file size matches
        let stored_size = u32::from_le_bytes([bntx[0x1C], bntx[0x1D], bntx[0x1E], bntx[0x1F]]);
        assert_eq!(stored_size as usize, bntx.len(), "stored file size should match");

        // Verify _STR exists
        let has_str = bntx.windows(4).any(|w| w == b"_STR");
        assert!(has_str, "should contain _STR string table");

        // Verify _DIC exists
        let has_dic = bntx.windows(4).any(|w| w == b"_DIC");
        assert!(has_dic, "should contain _DIC dict");

        // Verify BRTI exists
        let brti_count = bntx.windows(4).filter(|w| *w == b"BRTI").count();
        assert_eq!(brti_count, 6, "should have 6 BRTI entries");

        // Verify BRTD exists
        let has_brtd = bntx.windows(4).any(|w| w == b"BRTD");
        assert!(has_brtd, "should contain BRTD texture data");

        // Verify _RLT exists
        let has_rlt = bntx.windows(4).any(|w| w == b"_RLT");
        assert!(has_rlt, "should contain _RLT relocation table");

        // Verify "textures" string appears
        let has_textures = bntx.windows(8).any(|w| w == b"textures");
        assert!(has_textures, "should contain 'textures' string");

        // Verify all texture names appear
        for tex in &bfres.textures {
            let name_bytes = tex.name.as_bytes();
            let has_name = bntx.windows(name_bytes.len()).any(|w| w == name_bytes);
            assert!(has_name, "should contain texture name '{}'", tex.name);
        }

        eprintln!("BNTX built: {} bytes from {} textures", bntx.len(), bfres.textures.len());
    }

    #[test]
    fn build_bntx_from_cat_textures() {
        let data = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/Animal_Cat.Tex1.wiiu.bfres"
        ))
        .expect("fixture file should exist");

        let bfres = wiiu::parse(&data).expect("should parse Wii U Tex1 BFRES");
        assert!(!bfres.textures.is_empty(), "should have textures");

        let bntx = build_bntx(&bfres.textures);
        assert!(!bntx.is_empty(), "BNTX should not be empty");
        assert_eq!(&bntx[0..4], b"BNTX");

        let tex_count = u32::from_le_bytes([bntx[0x24], bntx[0x25], bntx[0x26], bntx[0x27]]);
        assert_eq!(tex_count, bfres.textures.len() as u32);

        eprintln!("Cat BNTX built: {} bytes from {} textures",
            bntx.len(), bfres.textures.len());
    }

    #[test]
    fn build_bntx_empty_textures() {
        let bntx = build_bntx(&[]);
        assert!(bntx.is_empty(), "empty textures should produce empty BNTX");
    }

    #[test]
    fn bntx_brti_format_and_dimensions() {
        let data = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/Animal_Boar_Big.Tex1.wiiu.bfres"
        ))
        .expect("fixture file should exist");

        let bfres = wiiu::parse(&data).expect("should parse");
        let bntx = build_bntx(&bfres.textures);

        // Find all BRTI entries and verify their format/dimension fields
        let mut brti_positions: Vec<usize> = Vec::new();
        for i in 0..bntx.len().saturating_sub(4) {
            if &bntx[i..i+4] == b"BRTI" {
                brti_positions.push(i);
            }
        }

        assert_eq!(brti_positions.len(), 6);

        for brti_pos in &brti_positions {
            let brti = &bntx[*brti_pos..];

            // Format should be a valid Switch format
            let format = u32::from_le_bytes([brti[0x1C], brti[0x1D], brti[0x1E], brti[0x1F]]);
            assert!(format != 0, "format should be non-zero");

            // Width and height should be non-zero
            let width = u32::from_le_bytes([brti[0x24], brti[0x25], brti[0x26], brti[0x27]]);
            let height = u32::from_le_bytes([brti[0x28], brti[0x29], brti[0x2A], brti[0x2B]]);
            assert!(width > 0, "width should be > 0");
            assert!(height > 0, "height should be > 0");

            // Image size should be > 0
            let image_size = u32::from_le_bytes([brti[0x50], brti[0x51], brti[0x52], brti[0x53]]);
            assert!(image_size > 0, "image_size should be > 0");

            // Channels should be valid (0-5)
            for i in 0..4 {
                let ch = brti[0x58 + i];
                assert!(ch <= 5, "channel {} value {} should be <= 5", i, ch);
            }
        }
    }
}
