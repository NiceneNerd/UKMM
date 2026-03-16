//! TegraX1 (Switch) texture swizzle/deswizzle implementation.
//!
//! The Switch GPU uses block-linear tiling based on GOBs (Graphics Output Blocks).
//! A GOB is 64 bytes wide and 8 rows tall (512 bytes). Multiple GOBs are stacked
//! vertically into "blocks" whose height is a power-of-two number of GOBs.

/// GOB (Graphics Output Block) width in bytes.
const GOB_WIDTH: u32 = 64;

/// GOB height in rows.
const GOB_HEIGHT: u32 = 8;

/// GOB size in bytes (64 * 8 = 512).
const GOB_SIZE: u32 = GOB_WIDTH * GOB_HEIGHT;

/// Format information for a Switch texture format.
pub struct FormatInfo {
    /// Bytes per pixel (or bytes per compressed block for BCn).
    pub bytes_per_pixel: u32,
    /// Compression block width in pixels (1 for uncompressed, 4 for BCn).
    pub block_width: u32,
    /// Compression block height in pixels (1 for uncompressed, 4 for BCn).
    pub block_height: u32,
}

/// Get format info for a Switch texture format value.
pub fn get_format_info(format: u32) -> FormatInfo {
    match format {
        // Uncompressed
        0x0201 => FormatInfo {
            bytes_per_pixel: 1,
            block_width: 1,
            block_height: 1,
        }, // R8_UNORM
        0x0301 => FormatInfo {
            bytes_per_pixel: 2,
            block_width: 1,
            block_height: 1,
        }, // R8_G8_UNORM
        0x0501 => FormatInfo {
            bytes_per_pixel: 2,
            block_width: 1,
            block_height: 1,
        }, // R5_G6_B5
        0x0601 => FormatInfo {
            bytes_per_pixel: 2,
            block_width: 1,
            block_height: 1,
        }, // R5_G5_B5_A1
        0x0B01 => FormatInfo {
            bytes_per_pixel: 4,
            block_width: 1,
            block_height: 1,
        }, // R8_G8_B8_A8_UNORM
        0x0B06 => FormatInfo {
            bytes_per_pixel: 4,
            block_width: 1,
            block_height: 1,
        }, // R8_G8_B8_A8_SRGB
        0x0B02 => FormatInfo {
            bytes_per_pixel: 4,
            block_width: 1,
            block_height: 1,
        }, // R8_G8_B8_A8_SNORM
        // BCn compressed
        0x1A01 => FormatInfo {
            bytes_per_pixel: 8,
            block_width: 4,
            block_height: 4,
        }, // BC1_UNORM
        0x1A06 => FormatInfo {
            bytes_per_pixel: 8,
            block_width: 4,
            block_height: 4,
        }, // BC1_SRGB
        0x1B01 => FormatInfo {
            bytes_per_pixel: 16,
            block_width: 4,
            block_height: 4,
        }, // BC2_UNORM
        0x1B06 => FormatInfo {
            bytes_per_pixel: 16,
            block_width: 4,
            block_height: 4,
        }, // BC2_SRGB
        0x1C01 => FormatInfo {
            bytes_per_pixel: 16,
            block_width: 4,
            block_height: 4,
        }, // BC3_UNORM
        0x1C06 => FormatInfo {
            bytes_per_pixel: 16,
            block_width: 4,
            block_height: 4,
        }, // BC3_SRGB
        0x1D01 => FormatInfo {
            bytes_per_pixel: 8,
            block_width: 4,
            block_height: 4,
        }, // BC4_UNORM
        0x1D02 => FormatInfo {
            bytes_per_pixel: 8,
            block_width: 4,
            block_height: 4,
        }, // BC4_SNORM
        0x1E01 => FormatInfo {
            bytes_per_pixel: 16,
            block_width: 4,
            block_height: 4,
        }, // BC5_UNORM
        0x1E02 => FormatInfo {
            bytes_per_pixel: 16,
            block_width: 4,
            block_height: 4,
        }, // BC5_SNORM
        // Default to 4bpp uncompressed
        _ => FormatInfo {
            bytes_per_pixel: 4,
            block_width: 1,
            block_height: 1,
        },
    }
}

/// Integer division rounding up.
fn div_round_up(n: u32, d: u32) -> u32 {
    (n + d - 1) / d
}

/// Round `x` up to the next multiple of `y`. `y` must be a power of two.
fn round_up(x: u32, y: u32) -> u32 {
    ((x.wrapping_sub(1)) | (y - 1)).wrapping_add(1)
}

/// Round up to the next power of two.
fn pow2_round_up(mut x: u32) -> u32 {
    if x == 0 {
        return 1;
    }
    x = x.wrapping_sub(1);
    x |= x >> 1;
    x |= x >> 2;
    x |= x >> 4;
    x |= x >> 8;
    x |= x >> 16;
    x.wrapping_add(1)
}

/// Calculate appropriate block height (in GOBs) for a given texture height in pixels.
///
/// The block height is the number of GOBs stacked vertically. It must be a
/// power of two between 1 and 16. Larger textures benefit from taller blocks.
pub fn get_block_height(height: u32) -> u32 {
    let block_height = pow2_round_up(height / GOB_HEIGHT);
    block_height.clamp(1, 16)
}

/// Calculate log2 of the appropriate block height for a given texture height.
pub fn get_block_height_log2(height: u32) -> u32 {
    let bh = get_block_height(height);
    if bh == 0 {
        0
    } else {
        bh.trailing_zeros()
    }
}

/// Compute the swizzled address of a pixel in block-linear layout.
///
/// This maps a (x, y) pixel coordinate to a byte offset in the tiled texture buffer.
/// The layout is organized as:
/// - GOB rows (groups of `block_height` GOBs)
/// - GOB columns (horizontal GOBs within a row)
/// - Individual GOBs within a block
/// - Bytes within a GOB (with internal address swizzling)
fn get_addr_block_linear(
    x: u32,
    y: u32,
    width: u32,
    bytes_per_pixel: u32,
    base_address: u32,
    block_height: u32,
) -> u32 {
    let image_width_in_gobs = div_round_up(width * bytes_per_pixel, GOB_WIDTH);

    // Which block-row of GOBs this y coordinate falls in
    let gob_address = base_address
        + (y / (GOB_HEIGHT * block_height))
            * GOB_SIZE
            * block_height
            * image_width_in_gobs
        + (x * bytes_per_pixel / GOB_WIDTH) * GOB_SIZE * block_height
        + (y % (GOB_HEIGHT * block_height) / GOB_HEIGHT) * GOB_SIZE;

    let x_bytes = x * bytes_per_pixel;

    // Within-GOB addressing uses a specific bit interleaving pattern
    gob_address
        + ((x_bytes % GOB_WIDTH) / 32) * 256
        + ((y % GOB_HEIGHT) / 2) * 64
        + ((x_bytes % 32) / 16) * 32
        + (y % 2) * 16
        + (x_bytes % 16)
}

/// Swizzle linear pixel data to TegraX1 block-linear layout for Switch GPU.
///
/// # Arguments
/// * `width` - Texture width in pixels
/// * `height` - Texture height in pixels
/// * `depth` - Texture depth (usually 1 for 2D textures)
/// * `blk_width` - Compression block width (1 for uncompressed, 4 for BCn)
/// * `blk_height` - Compression block height (1 for uncompressed, 4 for BCn)
/// * `blk_depth` - Compression block depth (usually 1)
/// * `bpp` - Bytes per pixel (NOT bits)
/// * `tile_mode` - 0 for linear, 1 for block-linear
/// * `block_height_log2` - Log2 of block height in GOBs
/// * `data` - Source linear pixel data
///
/// # Returns
/// Swizzled texture data in block-linear layout.
pub fn swizzle(
    width: u32,
    height: u32,
    depth: u32,
    blk_width: u32,
    blk_height: u32,
    blk_depth: u32,
    bpp: u32,
    tile_mode: u32,
    block_height_log2: u32,
    data: &[u8],
) -> Vec<u8> {
    swizzle_inner(
        width,
        height,
        depth,
        blk_width,
        blk_height,
        blk_depth,
        bpp,
        tile_mode,
        block_height_log2,
        data,
        false, // swizzle: linear -> tiled
    )
}

/// Deswizzle TegraX1 block-linear data back to linear pixel layout.
///
/// This is the reverse of [`swizzle`]: it takes tiled GPU texture data and
/// produces a linear row-major pixel buffer.
pub fn deswizzle(
    width: u32,
    height: u32,
    depth: u32,
    blk_width: u32,
    blk_height: u32,
    blk_depth: u32,
    bpp: u32,
    tile_mode: u32,
    block_height_log2: u32,
    data: &[u8],
) -> Vec<u8> {
    swizzle_inner(
        width,
        height,
        depth,
        blk_width,
        blk_height,
        blk_depth,
        bpp,
        tile_mode,
        block_height_log2,
        data,
        true, // deswizzle: tiled -> linear
    )
}

/// Internal swizzle/deswizzle implementation.
///
/// When `to_linear` is false, copies from linear source to tiled output (swizzle).
/// When `to_linear` is true, copies from tiled source to linear output (deswizzle).
fn swizzle_inner(
    width: u32,
    height: u32,
    depth: u32,
    blk_width: u32,
    blk_height: u32,
    blk_depth: u32,
    bpp: u32,
    tile_mode: u32,
    block_height_log2: u32,
    data: &[u8],
    to_linear: bool,
) -> Vec<u8> {
    let block_height = 1u32 << block_height_log2;

    // Divide pixel dimensions by compression block size to get working dimensions
    let width = div_round_up(width, blk_width);
    let height = div_round_up(height, blk_height);
    let _depth = div_round_up(depth, blk_depth);

    let pitch: u32;
    let surf_size: u32;

    if tile_mode == 0 {
        // Linear layout
        pitch = width * bpp;
        surf_size = pitch * height;
    } else {
        // Block-linear layout
        pitch = round_up(width * bpp, GOB_WIDTH);
        surf_size = pitch * round_up(height, block_height * GOB_HEIGHT);
    }

    let linear_size = (width * height * bpp) as usize;

    let result_size = if to_linear {
        linear_size
    } else {
        surf_size as usize
    };

    let mut result = vec![0u8; result_size];
    let len = bpp as usize;

    for y in 0..height {
        for x in 0..width {
            let tiled_pos = if tile_mode == 0 {
                (y * pitch + x * bpp) as usize
            } else {
                get_addr_block_linear(x, y, width, bpp, 0, block_height) as usize
            };

            let linear_pos = ((y * width + x) * bpp) as usize;

            if to_linear {
                // Copy from tiled (data) to linear (result)
                if tiled_pos + len <= data.len() && linear_pos + len <= result.len() {
                    result[linear_pos..linear_pos + len]
                        .copy_from_slice(&data[tiled_pos..tiled_pos + len]);
                }
            } else {
                // Copy from linear (data) to tiled (result)
                if linear_pos + len <= data.len() && tiled_pos + len <= result.len() {
                    result[tiled_pos..tiled_pos + len]
                        .copy_from_slice(&data[linear_pos..linear_pos + len]);
                }
            }
        }
    }

    result
}

/// Swizzle all mip levels for a texture.
///
/// Returns a tuple of (swizzled_data, mip_offsets) where mip_offsets[i] is the
/// byte offset of mip level i within the output buffer.
///
/// # Arguments
/// * `width` - Base mip width in pixels
/// * `height` - Base mip height in pixels
/// * `depth` - Base mip depth
/// * `blk_width` - Compression block width
/// * `blk_height` - Compression block height
/// * `blk_depth` - Compression block depth
/// * `bpp` - Bytes per pixel
/// * `tile_mode` - Tile mode (0=linear, 1=block-linear)
/// * `block_height_log2` - Base block height log2
/// * `mip_count` - Number of mip levels
/// * `linear_data` - All mip levels concatenated in linear layout
pub fn swizzle_mip_maps(
    width: u32,
    height: u32,
    depth: u32,
    blk_width: u32,
    blk_height: u32,
    blk_depth: u32,
    bpp: u32,
    tile_mode: u32,
    block_height_log2: u32,
    mip_count: u32,
    linear_data: &[u8],
) -> (Vec<u8>, Vec<u64>) {
    let mut output = Vec::new();
    let mut mip_offsets = Vec::with_capacity(mip_count as usize);
    let mut src_offset: usize = 0;

    for level in 0..mip_count {
        // Calculate mip dimensions (halve each level, minimum 1)
        let mip_width = (width >> level).max(1);
        let mip_height = (height >> level).max(1);
        let mip_depth = (depth >> level).max(1);

        // Adjust block height for smaller mip levels
        let mip_block_height_log2 = {
            let h = div_round_up(mip_height, blk_height);
            let bh_log2 = get_block_height_log2(h);
            bh_log2.min(block_height_log2)
        };

        // Calculate linear mip size
        let mip_w = div_round_up(mip_width, blk_width);
        let mip_h = div_round_up(mip_height, blk_height);
        let linear_mip_size = (mip_w * mip_h * bpp) as usize;

        // Extract this mip level's linear data
        let mip_end = (src_offset + linear_mip_size).min(linear_data.len());
        let mip_data = if src_offset < linear_data.len() {
            &linear_data[src_offset..mip_end]
        } else {
            &[]
        };

        // Pad if needed (source might be short)
        let padded_data;
        let final_mip_data = if mip_data.len() < linear_mip_size {
            padded_data = {
                let mut v = mip_data.to_vec();
                v.resize(linear_mip_size, 0);
                v
            };
            &padded_data
        } else {
            mip_data
        };

        // Swizzle this mip level
        let swizzled = swizzle(
            mip_width,
            mip_height,
            mip_depth,
            blk_width,
            blk_height,
            blk_depth,
            bpp,
            tile_mode,
            mip_block_height_log2,
            final_mip_data,
        );

        // Align mip offset (mip levels after the first are typically aligned to GOB_SIZE)
        if level > 0 {
            let alignment = GOB_SIZE as usize;
            let aligned = (output.len() + alignment - 1) & !(alignment - 1);
            output.resize(aligned, 0);
        }

        mip_offsets.push(output.len() as u64);
        output.extend_from_slice(&swizzled);
        src_offset += linear_mip_size;
    }

    (output, mip_offsets)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_div_round_up() {
        assert_eq!(div_round_up(0, 4), 0);
        assert_eq!(div_round_up(1, 4), 1);
        assert_eq!(div_round_up(4, 4), 1);
        assert_eq!(div_round_up(5, 4), 2);
        assert_eq!(div_round_up(8, 4), 2);
        assert_eq!(div_round_up(9, 4), 3);
    }

    #[test]
    fn test_round_up() {
        assert_eq!(round_up(1, 64), 64);
        assert_eq!(round_up(64, 64), 64);
        assert_eq!(round_up(65, 64), 128);
        assert_eq!(round_up(128, 64), 128);
    }

    #[test]
    fn test_pow2_round_up() {
        assert_eq!(pow2_round_up(0), 1);
        assert_eq!(pow2_round_up(1), 1);
        assert_eq!(pow2_round_up(2), 2);
        assert_eq!(pow2_round_up(3), 4);
        assert_eq!(pow2_round_up(5), 8);
        assert_eq!(pow2_round_up(16), 16);
        assert_eq!(pow2_round_up(17), 32);
    }

    #[test]
    fn test_get_block_height() {
        assert_eq!(get_block_height(8), 1);   // 8/8=1 => pow2(1)=1
        assert_eq!(get_block_height(16), 2);  // 16/8=2 => pow2(2)=2
        assert_eq!(get_block_height(32), 4);  // 32/8=4 => pow2(4)=4
        assert_eq!(get_block_height(64), 8);  // 64/8=8 => pow2(8)=8
        assert_eq!(get_block_height(128), 16); // 128/8=16 => pow2(16)=16
        assert_eq!(get_block_height(256), 16); // 256/8=32 => clamped to 16
    }

    #[test]
    fn test_get_block_height_log2() {
        assert_eq!(get_block_height_log2(8), 0);   // block_height=1 => log2=0
        assert_eq!(get_block_height_log2(16), 1);  // block_height=2 => log2=1
        assert_eq!(get_block_height_log2(32), 2);  // block_height=4 => log2=2
        assert_eq!(get_block_height_log2(64), 3);  // block_height=8 => log2=3
        assert_eq!(get_block_height_log2(128), 4); // block_height=16 => log2=4
    }

    #[test]
    fn test_get_format_info() {
        let info = get_format_info(0x0B01);
        assert_eq!(info.bytes_per_pixel, 4);
        assert_eq!(info.block_width, 1);
        assert_eq!(info.block_height, 1);

        let info = get_format_info(0x1A01);
        assert_eq!(info.bytes_per_pixel, 8);
        assert_eq!(info.block_width, 4);
        assert_eq!(info.block_height, 4);

        let info = get_format_info(0x1C01);
        assert_eq!(info.bytes_per_pixel, 16);
        assert_eq!(info.block_width, 4);
        assert_eq!(info.block_height, 4);
    }

    #[test]
    fn test_linear_mode_passthrough() {
        // In linear mode (tile_mode=0), swizzle should produce data
        // that deswizzle recovers exactly.
        let width = 16u32;
        let height = 16u32;
        let bpp = 4u32;
        let size = (width * height * bpp) as usize;
        let mut data = vec![0u8; size];
        for i in 0..size {
            data[i] = (i % 251) as u8; // prime modulus for variety
        }

        let swizzled = swizzle(width, height, 1, 1, 1, 1, bpp, 0, 0, &data);
        let recovered = deswizzle(width, height, 1, 1, 1, 1, bpp, 0, 0, &swizzled);

        assert_eq!(recovered, data);
    }

    #[test]
    fn test_block_linear_roundtrip() {
        // Swizzle then deswizzle should recover the original data for
        // block-linear mode.
        let width = 64u32;
        let height = 64u32;
        let bpp = 4u32; // R8G8B8A8
        let block_height_log2 = get_block_height_log2(height);
        let size = (width * height * bpp) as usize;
        let mut data = vec![0u8; size];
        for i in 0..size {
            data[i] = ((i * 7 + 13) % 256) as u8;
        }

        let swizzled = swizzle(width, height, 1, 1, 1, 1, bpp, 1, block_height_log2, &data);
        let recovered =
            deswizzle(width, height, 1, 1, 1, 1, bpp, 1, block_height_log2, &swizzled);

        assert_eq!(
            recovered.len(),
            data.len(),
            "Recovered data length mismatch"
        );
        assert_eq!(recovered, data, "Swizzle/deswizzle round-trip failed");
    }

    #[test]
    fn test_block_linear_roundtrip_small() {
        // Test with minimum interesting size: 8x8 pixels
        let width = 8u32;
        let height = 8u32;
        let bpp = 4u32;
        let block_height_log2 = 0; // block_height = 1 GOB
        let size = (width * height * bpp) as usize;
        let mut data = vec![0u8; size];
        for i in 0..size {
            data[i] = i as u8;
        }

        let swizzled = swizzle(width, height, 1, 1, 1, 1, bpp, 1, block_height_log2, &data);
        let recovered =
            deswizzle(width, height, 1, 1, 1, 1, bpp, 1, block_height_log2, &swizzled);

        assert_eq!(recovered, data);
    }

    #[test]
    fn test_block_linear_roundtrip_bcn() {
        // Test with BCn-style compressed format (4x4 blocks, 8 bytes per block)
        let width = 64u32;
        let height = 64u32;
        let blk_width = 4u32;
        let blk_height = 4u32;
        let bpp = 8u32; // BC1: 8 bytes per 4x4 block

        // In block coordinates: 16x16 blocks
        let block_w = div_round_up(width, blk_width);
        let block_h = div_round_up(height, blk_height);
        let block_height_log2 = get_block_height_log2(block_h);

        let size = (block_w * block_h * bpp) as usize;
        let mut data = vec![0u8; size];
        for i in 0..size {
            data[i] = ((i * 3 + 5) % 256) as u8;
        }

        let swizzled = swizzle(
            width,
            height,
            1,
            blk_width,
            blk_height,
            1,
            bpp,
            1,
            block_height_log2,
            &data,
        );
        let recovered = deswizzle(
            width,
            height,
            1,
            blk_width,
            blk_height,
            1,
            bpp,
            1,
            block_height_log2,
            &swizzled,
        );

        assert_eq!(recovered.len(), data.len());
        assert_eq!(recovered, data);
    }

    #[test]
    fn test_block_linear_data_is_different() {
        // Verify that block-linear swizzling actually rearranges the data
        let width = 64u32;
        let height = 64u32;
        let bpp = 4u32;
        let block_height_log2 = get_block_height_log2(height);
        let size = (width * height * bpp) as usize;
        let mut data = vec![0u8; size];
        for i in 0..size {
            data[i] = i as u8;
        }

        let swizzled = swizzle(width, height, 1, 1, 1, 1, bpp, 1, block_height_log2, &data);

        // The swizzled data should differ from the linear data
        // (unless the swizzle is a no-op, which it shouldn't be for block-linear)
        assert_ne!(swizzled[..size], data[..]);
    }

    #[test]
    fn test_gob_address_known_values() {
        // Pixel (0,0) with block_height=1 should map to offset 0
        assert_eq!(get_addr_block_linear(0, 0, 64, 4, 0, 1), 0);

        // Pixel (1,0) should be at byte offset 4 (bpp=4)
        assert_eq!(get_addr_block_linear(1, 0, 64, 4, 0, 1), 4);
    }

    #[test]
    fn test_swizzle_mip_maps_offsets() {
        let width = 64u32;
        let height = 64u32;
        let bpp = 4u32;
        let block_height_log2 = get_block_height_log2(height);

        // Create linear data for 4 mip levels: 64x64, 32x32, 16x16, 8x8
        let total_size: usize = (0..4)
            .map(|l| {
                let w = (width >> l).max(1);
                let h = (height >> l).max(1);
                (w * h * bpp) as usize
            })
            .sum();
        let data = vec![0xABu8; total_size];

        let (swizzled, offsets) =
            swizzle_mip_maps(width, height, 1, 1, 1, 1, bpp, 1, block_height_log2, 4, &data);

        // Should have 4 mip offsets
        assert_eq!(offsets.len(), 4);

        // First mip should start at 0
        assert_eq!(offsets[0], 0);

        // Each subsequent mip offset should be greater than the previous
        for i in 1..offsets.len() {
            assert!(
                offsets[i] > offsets[i - 1],
                "Mip offset {} ({}) should be > mip offset {} ({})",
                i,
                offsets[i],
                i - 1,
                offsets[i - 1]
            );
        }

        // Total swizzled size should be > 0
        assert!(!swizzled.is_empty());
    }

    #[test]
    fn test_non_power_of_two_dimensions() {
        // Non-power-of-two dimensions should still work (common for mips)
        let width = 48u32;
        let height = 24u32;
        let bpp = 4u32;
        let block_height_log2 = get_block_height_log2(height);
        let size = (width * height * bpp) as usize;
        let mut data = vec![0u8; size];
        for i in 0..size {
            data[i] = (i % 200) as u8;
        }

        let swizzled = swizzle(width, height, 1, 1, 1, 1, bpp, 1, block_height_log2, &data);
        let recovered =
            deswizzle(width, height, 1, 1, 1, 1, bpp, 1, block_height_log2, &swizzled);

        assert_eq!(recovered.len(), size);
        assert_eq!(recovered, data);
    }
}
