//! GX2 (Wii U) texture deswizzle implementation.
//!
//! The Wii U GPU uses AMD GCN-style tiling with micro-tiles (8x8 pixel blocks)
//! and macro-tiles composed of multiple micro-tiles. This module converts tiled
//! GPU texture data back to a linear pixel layout.

/// Get bits-per-pixel for a GX2 surface format value.
pub fn get_format_bpp(format: u32) -> u32 {
    match format & 0x3F {
        0x01 => 8,   // R8, L8
        0x02 => 16,  // R4_G4_B4_A4
        0x05 => 16,  // R8_G8, L8_A8
        0x07 => 16,  // R5_G6_B5
        0x08 => 16,  // R5_G5_B5_A1
        0x0A => 32,  // R8_G8_B8_A8
        0x0B => 32,  // R10_G10_B10_A2
        0x19 => 32,  // R32, D32
        0x1A => 64,  // BC1/DXT1
        0x1B => 128, // BC2/DXT3
        0x1C => 128, // BC3/DXT5
        0x1D => 64,  // BC4/ATI1
        0x1E => 128, // BC5/ATI2
        _ => 32,     // default
    }
}

/// Returns true if the GX2 surface format is a BCn block-compressed format.
pub fn is_bcn_format(format: u32) -> bool {
    let f = format & 0x3F;
    (0x1A..=0x20).contains(&f)
}

/// Compute the linear (non-tiled) address for a pixel.
fn compute_linear(
    x: u32,
    y: u32,
    slice: u32,
    sample: u32,
    bytes_per_pixel: u32,
    pitch: u32,
    height: u32,
    num_slices: u32,
) -> usize {
    let slice_offset = pitch * height * (slice + sample * num_slices);
    ((y * pitch + x + slice_offset) * bytes_per_pixel) as usize
}

/// Compute pixel index within a micro tile for thin (non-thick) tile modes.
///
/// The pixel index determines the position of a pixel within the 8x8 micro tile.
/// The bit interleaving pattern depends on the bits-per-pixel of the format.
fn compute_pixel_index_micro(x: u32, y: u32, _z: u32, bpp: u32) -> u32 {
    let x0 = x & 1;
    let x1 = (x >> 1) & 1;
    let x2 = (x >> 2) & 1;
    let y0 = y & 1;
    let y1 = (y >> 1) & 1;
    let y2 = (y >> 2) & 1;

    match bpp {
        8 => x0 | (x1 << 1) | (x2 << 2) | (y1 << 3) | (y0 << 4) | (y2 << 5),
        16 => x0 | (x1 << 1) | (x2 << 2) | (y0 << 3) | (y1 << 4) | (y2 << 5),
        32 | 64 => x0 | (x1 << 1) | (y0 << 2) | (x2 << 3) | (y1 << 4) | (y2 << 5),
        128 => x0 | (y0 << 1) | (x1 << 2) | (x2 << 3) | (y1 << 4) | (y2 << 5),
        _ => x0 | (x1 << 1) | (x2 << 2) | (y0 << 3) | (y1 << 4) | (y2 << 5),
    }
}

/// Compute the byte offset of a pixel in a micro-tiled surface.
fn compute_micro_tiled(
    x: u32,
    y: u32,
    slice: u32,
    bpp: u32,
    pitch: u32,
    height: u32,
    tile_mode: u32,
    _is_depth: bool,
) -> usize {
    let thickness: u32 = if tile_mode == 3 { 4 } else { 1 };
    let micro_tile_bytes = (64 * thickness * bpp + 7) / 8;
    let micro_tiles_per_row = pitch >> 3; // pitch / 8
    let micro_tile_x = x >> 3;
    let micro_tile_y = y >> 3;
    let micro_tile_z = slice / thickness;

    let micro_tile_offset =
        micro_tile_bytes * (micro_tile_x + micro_tile_y * micro_tiles_per_row);
    let slice_bytes = (pitch * height * thickness * bpp + 7) / 8;
    let slice_offset = micro_tile_z * slice_bytes;

    let pixel_index = compute_pixel_index_micro(x & 7, y & 7, slice % thickness, bpp);
    let pixel_offset = (bpp * pixel_index) >> 3;

    (pixel_offset + micro_tile_offset + slice_offset) as usize
}

/// Compute pipe index for a 2-pipe configuration (Wii U).
fn compute_pipe(x: u32, y: u32) -> u32 {
    ((x >> 3) ^ (y >> 3)) & 1
}

/// Compute bank index for a 4-bank configuration (Wii U).
fn compute_bank(x: u32, y: u32) -> u32 {
    let x3 = (x >> 3) & 1;
    let x4 = (x >> 4) & 1;
    let y3 = (y >> 3) & 1;
    let y4 = (y >> 4) & 1;
    ((x3 ^ y4) | ((x4 ^ y3) << 1)) & 3
}

/// Get macro tile dimensions (in micro-tiles, i.e., units of 8 pixels) for a tile mode.
/// Returns (macro_tile_width, macro_tile_height) in micro-tile units.
fn get_macro_tile_dim(tile_mode: u32) -> (u32, u32) {
    match tile_mode {
        4 | 7 | 8 | 11 | 12 | 13 | 14 | 15 => (4, 2),
        5 | 9 => (2, 4),
        6 | 10 => (1, 8),
        _ => (4, 2), // fallback
    }
}

/// Get tile thickness for a tile mode.
fn get_thickness(tile_mode: u32) -> u32 {
    match tile_mode {
        7 | 11 | 13 => 4,
        15 => 8,
        _ => 1,
    }
}

/// Apply bank swapping for 2B/3B tile modes.
fn apply_bank_swap(tile_mode: u32, macro_tile_index: u32, bank: u32, num_banks: u32) -> u32 {
    match tile_mode {
        8 | 9 | 10 | 11 | 14 | 15 => {
            static BANK_SWAP_ORDER: [u32; 4] = [0, 1, 3, 2];
            let swap_index = (macro_tile_index % num_banks) as usize;
            bank ^ BANK_SWAP_ORDER[swap_index]
        }
        _ => bank,
    }
}

/// Compute the byte offset of a pixel in a macro-tiled surface.
fn compute_macro_tiled(
    x: u32,
    y: u32,
    slice: u32,
    _sample: u32,
    bpp: u32,
    pitch: u32,
    height: u32,
    _num_samples: u32,
    tile_mode: u32,
    _is_depth: bool,
    pipe_swizzle: u32,
    bank_swizzle: u32,
) -> usize {
    let num_banks: u32 = 4;
    let pipe_interleave_bytes: u32 = 256;
    let pipe_bits = 1u32; // log2(num_pipes)
    let bank_bits = 2u32; // log2(num_banks)

    let thickness = get_thickness(tile_mode);
    let (macro_tile_w, macro_tile_h) = get_macro_tile_dim(tile_mode);

    // Macro tile dimensions in pixels
    let macro_tile_width_pixels = macro_tile_w * 8;
    let macro_tile_height_pixels = macro_tile_h * 8;

    let macro_tile_bytes =
        (macro_tile_w * 8 * macro_tile_h * 8 * thickness * bpp + 7) / 8;

    let macro_tiles_per_row = pitch / macro_tile_width_pixels;
    let macro_tile_row = y / macro_tile_height_pixels;
    let macro_tile_col = x / macro_tile_width_pixels;
    let macro_tile_index = macro_tile_row * macro_tiles_per_row + macro_tile_col;
    let macro_tile_offset = (macro_tile_index as u64) * (macro_tile_bytes as u64);

    // Pixel offset within micro tile
    let pixel_index = compute_pixel_index_micro(x & 7, y & 7, slice % thickness, bpp);
    let pixel_offset = bpp * pixel_index;

    // Element offset from start of micro tile, in bits
    let elem_offset = pixel_offset;

    // Pipe and bank selection
    let pipe = compute_pipe(x, y) ^ pipe_swizzle;
    let mut bank = compute_bank(x, y) ^ bank_swizzle;

    // Bank swapping for certain tile modes
    bank = apply_bank_swap(tile_mode, macro_tile_index, bank, num_banks);

    // Slice offset
    let slice_bytes = (pitch * height * thickness * bpp + 7) / 8;
    let slice_offset = (slice / thickness) as u64 * slice_bytes as u64;

    // The micro tile offset within the macro tile is encoded in the element offset.
    // We need to figure out which micro tile within the macro tile this pixel belongs to.
    let micro_tile_x = (x % macro_tile_width_pixels) >> 3;
    let micro_tile_y = (y % macro_tile_height_pixels) >> 3;
    let micro_tile_index = micro_tile_x + micro_tile_y * macro_tile_w;
    let micro_tile_bytes = (64 * thickness * bpp + 7) / 8;
    let micro_tile_offset = micro_tile_index * micro_tile_bytes;

    // Combine: the final offset has the pipe/bank interleaving
    // The low bits (within pipe_interleave_bytes) come from the pixel offset within
    // the micro tile. The pipe and bank bits are inserted above that.
    let elem_offset_bytes = elem_offset / 8;
    let micro_offset = elem_offset_bytes + micro_tile_offset;

    // Standard GX2/addrlib formula:
    // final = (elem_offset_in_byte % pipe_interleave_bytes)
    //       | (pipe << pipe_interleave_log2)
    //       | (bank << (pipe_interleave_log2 + pipe_log2))
    //       | ((elem_offset_in_byte / pipe_interleave_bytes) << (pipe_interleave_log2 + pipe_log2 + bank_log2))
    // where elem_offset_in_byte includes the micro_tile_offset within the macro tile.
    // Then add the macro_tile base and slice offset.
    let group_bits = pipe_bits + bank_bits;
    let low_mask = pipe_interleave_bytes - 1;
    let pipe_interleave_log2 = 8u32; // log2(256)

    let final_offset = (micro_offset & low_mask) as u64
        | ((pipe as u64) << pipe_interleave_log2)
        | ((bank as u64) << (pipe_interleave_log2 + pipe_bits))
        | (((micro_offset >> pipe_interleave_log2) as u64) << (pipe_interleave_log2 + group_bits))
        + macro_tile_offset
        + slice_offset;

    final_offset as usize
}

/// Deswizzle GX2 surface data from Wii U GPU tiled layout to linear pixel data.
///
/// # Arguments
/// * `width` - Surface width in pixels
/// * `height` - Surface height in pixels
/// * `depth` - Surface depth (number of slices, usually 1)
/// * `format` - GX2 surface format enum value
/// * `aa` - Anti-aliasing mode (usually 0)
/// * `use_flags` - Surface use flags (bit 1 = depth buffer)
/// * `tile_mode` - GX2 tile mode (0-15)
/// * `swizzle` - Surface swizzle value
/// * `pitch` - Surface pitch in pixels
/// * `bpp` - Bits per pixel
/// * `slice` - Depth slice index
/// * `sample` - Sample index (for MSAA)
/// * `data` - Source tiled texture data
///
/// # Returns
/// Linear pixel data with row-major ordering.
pub fn deswizzle(
    width: u32,
    height: u32,
    _depth: u32,
    format: u32,
    _aa: u32,
    use_flags: u32,
    tile_mode: u32,
    swizzle: u32,
    pitch: u32,
    bpp: u32,
    slice: u32,
    sample: u32,
    data: &[u8],
) -> Vec<u8> {
    let pipe_swizzle = (swizzle >> 8) & 1;
    let bank_swizzle = (swizzle >> 9) & 3;

    let is_depth = (use_flags & 2) != 0;

    // For BCn formats, work in block coordinates
    let (w, h) = if is_bcn_format(format) {
        ((width + 3) / 4, (height + 3) / 4)
    } else {
        (width, height)
    };

    let bytes_per_pixel = bpp / 8;
    let linear_size = (w * h * bytes_per_pixel) as usize;
    let mut result = vec![0u8; linear_size];
    let bpp_val = bytes_per_pixel as usize;

    for y in 0..h {
        for x in 0..w {
            let pos = match tile_mode {
                0 | 1 => compute_linear(x, y, slice, sample, bytes_per_pixel, pitch, h, _depth),
                2 | 3 => compute_micro_tiled(x, y, slice, bpp, pitch, h, tile_mode, is_depth),
                4..=15 => compute_macro_tiled(
                    x,
                    y,
                    slice,
                    sample,
                    bpp,
                    pitch,
                    h,
                    1, // num_samples
                    tile_mode,
                    is_depth,
                    pipe_swizzle,
                    bank_swizzle,
                ),
                _ => {
                    log::warn!("Unknown GX2 tile mode {}, treating as linear", tile_mode);
                    compute_linear(x, y, slice, sample, bytes_per_pixel, pitch, h, _depth)
                }
            };

            let linear_pos = ((y * w + x) * bytes_per_pixel) as usize;

            if pos + bpp_val <= data.len() && linear_pos + bpp_val <= result.len() {
                result[linear_pos..linear_pos + bpp_val]
                    .copy_from_slice(&data[pos..pos + bpp_val]);
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_format_bpp() {
        assert_eq!(get_format_bpp(0x01), 8);
        assert_eq!(get_format_bpp(0x0A), 32);
        assert_eq!(get_format_bpp(0x1A), 64); // BC1
        assert_eq!(get_format_bpp(0x1C), 128); // BC3
        assert_eq!(get_format_bpp(0xFF), 32); // default
    }

    #[test]
    fn test_is_bcn_format() {
        assert!(is_bcn_format(0x1A));
        assert!(is_bcn_format(0x1B));
        assert!(is_bcn_format(0x1C));
        assert!(is_bcn_format(0x1D));
        assert!(is_bcn_format(0x1E));
        assert!(!is_bcn_format(0x0A));
        assert!(!is_bcn_format(0x01));
    }

    #[test]
    fn test_compute_pipe() {
        // pipe = ((x>>3) ^ (y>>3)) & 1
        assert_eq!(compute_pipe(0, 0), 0);
        assert_eq!(compute_pipe(8, 0), 1);
        assert_eq!(compute_pipe(0, 8), 1);
        assert_eq!(compute_pipe(8, 8), 0);
    }

    #[test]
    fn test_compute_bank() {
        assert_eq!(compute_bank(0, 0), 0);
        // x3 = (8>>3)&1 = 1, x4 = (8>>4)&1 = 0, y3=0, y4=0
        // (1^0) | ((0^0)<<1) = 1
        assert_eq!(compute_bank(8, 0), 1);
    }

    #[test]
    fn test_pixel_index_micro_32bpp() {
        // For 32bpp: x0 | (x1<<1) | (y0<<2) | (x2<<3) | (y1<<4) | (y2<<5)
        // Pixel (0,0) => 0
        assert_eq!(compute_pixel_index_micro(0, 0, 0, 32), 0);
        // Pixel (1,0) => 1
        assert_eq!(compute_pixel_index_micro(1, 0, 0, 32), 1);
        // Pixel (0,1) => 4 (y0=1 => bit2)
        assert_eq!(compute_pixel_index_micro(0, 1, 0, 32), 4);
    }

    #[test]
    fn test_linear_deswizzle_roundtrip() {
        // Linear tile mode should just be a straight copy
        let width = 16u32;
        let height = 16u32;
        let bpp = 32u32;
        let bytes_per_pixel = bpp / 8;
        let size = (width * height * bytes_per_pixel) as usize;
        let mut data = vec![0u8; size];
        for i in 0..size {
            data[i] = (i & 0xFF) as u8;
        }

        let result = deswizzle(
            width,
            height,
            1,     // depth
            0x0A,  // R8G8B8A8
            0,     // aa
            0,     // use_flags
            0,     // tile_mode: linear
            0,     // swizzle
            width, // pitch = width for linear
            bpp,
            0, // slice
            0, // sample
            &data,
        );

        assert_eq!(result.len(), size);
        assert_eq!(result, data);
    }

    #[test]
    fn test_micro_tiled_deswizzle_produces_output() {
        // Micro-tiled mode should produce non-empty output of the right size
        let width = 16u32;
        let height = 16u32;
        let bpp = 32u32;
        let bytes_per_pixel = bpp / 8;
        // Need enough source data for micro-tiled addressing
        let data_size = (width * height * bytes_per_pixel) as usize;
        let data = vec![0xABu8; data_size];

        let result = deswizzle(
            width,
            height,
            1,
            0x0A,
            0,
            0,
            2, // tile_mode: micro-tiled
            0,
            width,
            bpp,
            0,
            0,
            &data,
        );

        assert_eq!(result.len(), data_size);
        // Should have some non-zero bytes (we filled source with 0xAB)
        assert!(result.iter().any(|&b| b != 0));
    }

    #[test]
    fn test_get_macro_tile_dim() {
        assert_eq!(get_macro_tile_dim(4), (4, 2));
        assert_eq!(get_macro_tile_dim(5), (2, 4));
        assert_eq!(get_macro_tile_dim(6), (1, 8));
        assert_eq!(get_macro_tile_dim(7), (4, 2));
    }

    #[test]
    fn test_get_thickness() {
        assert_eq!(get_thickness(4), 1);
        assert_eq!(get_thickness(7), 4);
        assert_eq!(get_thickness(11), 4);
        assert_eq!(get_thickness(13), 4);
        assert_eq!(get_thickness(15), 8);
    }

    #[test]
    fn test_apply_bank_swap() {
        // Non-swapping tile modes should return bank unchanged
        assert_eq!(apply_bank_swap(4, 0, 2, 4), 2);
        assert_eq!(apply_bank_swap(7, 0, 3, 4), 3);

        // Swapping tile modes should XOR with bank_swap_order
        // bank_swap_order = [0, 1, 3, 2]
        // index 0 => XOR 0
        assert_eq!(apply_bank_swap(8, 0, 2, 4), 2 ^ 0);
        // index 1 => XOR 1
        assert_eq!(apply_bank_swap(8, 1, 2, 4), 2 ^ 1);
        // index 2 => XOR 3
        assert_eq!(apply_bank_swap(8, 2, 2, 4), 2 ^ 3);
        // index 3 => XOR 2
        assert_eq!(apply_bank_swap(8, 3, 2, 4), 2 ^ 2);
    }

    #[test]
    fn test_bcn_format_block_dimensions() {
        // BCn formats should divide width/height by 4
        let width = 256u32;
        let height = 256u32;
        let bpp = 64u32; // BC1
        let format = 0x1A; // BC1

        // For BCn, the deswizzle function works with block coordinates
        // so the effective dimensions are (256/4, 256/4) = (64, 64)
        let bytes_per_block = bpp / 8; // 8 bytes per BC1 block
        let expected_size = (64 * 64 * bytes_per_block) as usize;

        let data = vec![0u8; expected_size * 2]; // provide enough data
        let result = deswizzle(
            width, height, 1, format, 0, 0,
            0, // linear
            0, width / 4, bpp, 0, 0, &data,
        );

        assert_eq!(result.len(), expected_size);
    }
}
