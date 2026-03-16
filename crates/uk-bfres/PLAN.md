# uk-bfres: Rust BFRES Platform Converter — Implementation Plan

## Overview

Convert Wii U BFRES (v0.3, big-endian) files to Switch BFRES (v0.5, little-endian)
for Breath of the Wild modding. This enables UKMM's cross-platform mod conversion
to properly handle model, texture, and UI BFRES files.

## Architecture

```
src/
├── lib.rs          # Public API: is_wiiu_bfres(), convert_wiiu_to_switch()
├── error.rs        # Error types
├── binary.rs       # Binary reader/writer utilities (BE/LE)
├── wiiu/
│   ├── mod.rs      # Wii U BFRES parser entry point
│   ├── header.rs   # ResFile header parsing
│   ├── dict.rs     # Index group / dictionary parsing
│   ├── model.rs    # FMDL, FSHP, FSKL, FMAT parsing
│   ├── texture.rs  # GX2 texture info parsing
│   ├── buffer.rs   # Vertex/index buffer parsing
│   └── anim.rs     # Animation data parsing
├── switch/
│   ├── mod.rs      # Switch BFRES writer entry point
│   ├── header.rs   # ResFile header writing
│   ├── dict.rs     # ResDict writing
│   ├── model.rs    # FMDL, FSHP, FSKL, FMAT writing
│   ├── texture.rs  # BNTX texture container writing
│   ├── buffer.rs   # Vertex/index buffer writing
│   ├── anim.rs     # Animation data writing
│   └── reloc.rs    # Relocation table (_RLT) construction
├── convert.rs      # Conversion orchestration + material mapping
├── gx2.rs          # GX2 texture deswizzle algorithm
└── tegra.rs        # TegraX1 texture swizzle algorithm
```

## Internal Data Model

The converter works in 3 stages:
1. **Parse** Wii U BFRES binary → internal `BfresFile` struct
2. **Transform** internal struct (material conversion, animation renaming)
3. **Serialize** internal struct → Switch BFRES binary

```rust
pub struct BfresFile {
    pub name: String,
    pub version: (u8, u8, u8, u8),  // major, minor, micro, patch
    pub models: Vec<Model>,
    pub textures: Vec<TextureInfo>,
    pub texture_data: Vec<Vec<Vec<u8>>>,  // [tex_idx][mip_level] -> pixel data
    pub skeleton_anims: Vec<Animation>,
    pub material_anims: Vec<MaterialAnimation>,
    pub bone_vis_anims: Vec<Animation>,
    pub shape_anims: Vec<Animation>,
    pub scene_anims: Vec<Animation>,
    pub external_files: Vec<(String, Vec<u8>)>,
    pub string_table: Vec<String>,
}

pub struct Model {
    pub name: String,
    pub skeleton: Skeleton,
    pub shapes: Vec<Shape>,
    pub materials: Vec<Material>,
    pub vertex_buffers: Vec<VertexBuffer>,
}

pub struct Shape {
    pub name: String,
    pub meshes: Vec<Mesh>,
    pub vertex_buffer_index: u16,
    pub bounding_boxes: Vec<BoundingBox>,
    // ... other fields
}

pub struct Mesh {
    pub primitive_type: u32,
    pub index_format: u32,
    pub index_count: u32,
    pub index_buffer_offset: u32,
    pub sub_meshes: Vec<SubMesh>,
}

pub struct Material {
    pub name: String,
    pub shader_params: Vec<ShaderParam>,
    pub render_state: Option<RenderState>,   // Wii U
    pub render_infos: Vec<RenderInfo>,       // Switch (generated from RenderState)
    pub texture_refs: Vec<TextureRef>,
    pub samplers: Vec<Sampler>,
}

pub struct TextureInfo {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub mip_count: u32,
    pub format: TextureFormat,
    pub dim: u32,
    pub tile_mode: u32,
    pub swizzle: u32,
    pub array_length: u32,
    pub channel_selectors: [u8; 4],
    // GX2-specific fields
    pub gx2_surface_format: u32,
    pub gx2_aa_mode: u32,
    pub gx2_use: u32,
}

pub struct VertexBuffer {
    pub attributes: Vec<VertexAttribute>,
    pub buffers: Vec<Vec<u8>>,  // raw buffer data
    pub vertex_count: u32,
}
```

## Phase 1: Binary I/O Foundation (binary.rs)

### BigEndianReader
```rust
pub struct BigEndianReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl BigEndianReader {
    fn read_u8(&mut self) -> Result<u8>;
    fn read_u16(&mut self) -> Result<u16>;  // big-endian
    fn read_u32(&mut self) -> Result<u32>;
    fn read_i32(&mut self) -> Result<i32>;
    fn read_u64(&mut self) -> Result<u64>;
    fn read_f32(&mut self) -> Result<f32>;
    fn read_string(&mut self, len: usize) -> Result<String>;
    fn read_bytes(&mut self, len: usize) -> Result<&[u8]>;
    fn seek(&mut self, pos: usize);
    fn pos(&self) -> usize;
    fn remaining(&self) -> usize;

    // Read a null-terminated string at an absolute offset (for string table)
    fn read_string_at(&self, offset: usize) -> Result<String>;
}
```

### LittleEndianWriter
```rust
pub struct LittleEndianWriter {
    data: Vec<u8>,
    // Track positions of offsets that need relocation table entries
    reloc_entries: Vec<RelocEntry>,
}

impl LittleEndianWriter {
    fn write_u8(&mut self, v: u8);
    fn write_u16(&mut self, v: u16);  // little-endian
    fn write_u32(&mut self, v: u32);
    fn write_i32(&mut self, v: i32);
    fn write_u64(&mut self, v: u64);
    fn write_f32(&mut self, v: f32);
    fn write_bytes(&mut self, data: &[u8]);
    fn write_string(&mut self, s: &str);  // null-terminated
    fn pos(&self) -> usize;
    fn align(&mut self, alignment: usize);  // pad to alignment

    // Write a placeholder offset, returning position for later fixup
    fn write_offset_placeholder(&mut self) -> usize;
    fn fixup_offset(&mut self, placeholder_pos: usize, target: usize);
}
```

## Phase 2: Wii U BFRES Parser (wiiu/)

### Header Layout (big-endian)
```
Offset  Size  Field
0x00    4     Magic "FRES"
0x04    1     Version A (e.g., 0x04 or 0x03)
0x05    1     Version B (e.g., 0x05)
0x06    1     Version C (e.g., 0x00)
0x07    1     Version D (e.g., 0x03)
0x08    2     BOM (0xFEFF = big-endian)
0x0A    2     Header size (0x000C or 0x0010)
0x0C    4     File size
0x10    4     Alignment
0x14    4     Name offset (relative to start of string table)
0x18    4     String table size
0x1C    4     String table offset
0x20    4*12  Sub-file offset/count pairs for:
              [0] Models (FMDL)
              [1] Textures
              [2] Skeleton Animations (FSKA)
              [3] Shader Param Animations
              [4] Color Animations
              [5] Tex SRT Animations
              [6] Tex Pattern Animations
              [7] Bone Visibility Animations
              [8] Material Visibility Animations
              [9] Shape Animations
              [10] Scene Animations
              [11] External Files
```

Each sub-file entry is: offset (4 bytes) + dict offset (4 bytes).

### Dictionary (Index Group) Layout
```
Offset  Size  Field
0x00    4     Size (total bytes)
0x04    4     Entry count

Per entry (16 bytes each):
0x00    4     Search value
0x04    2     Left index
0x06    2     Right index
0x08    4     Name offset (relative)
0x0C    4     Data offset (relative)
```

The first entry (index 0) is the root sentinel.

## Phase 3: Model Data

### FMDL Header (Wii U, big-endian)
```
0x00    4     Magic "FMDL"
0x04    4     Header size
0x08    4     Filename offset
0x0C    4     End-of-string offset
0x10    4     FSKL offset
0x14    4     FVTX array offset
0x18    4     FSHP offset / dict
0x1C    4     FMAT offset / dict
0x20    4     User data offset / dict
0x24    2     FVTX count
0x26    2     FSHP count
0x28    2     FMAT count
0x2A    2     User data count
0x2C    4     Total vertex count
```

### Vertex Buffer Endian Conversion
Vertex buffer data must be byte-swapped per attribute based on the attribute format:
- Format_32_32_32_32_Float → swap each 4-byte float
- Format_16_16_16_16_Float → swap each 2-byte half
- Format_8_8_8_8_UNorm → no swap needed (single bytes)
- etc.

The attribute format table determines swap granularity (1, 2, or 4 bytes per component).

### Index Buffer Endian Conversion
Index buffers need byte-swapping based on index format:
- UInt16 → swap 2 bytes
- UInt32 → swap 4 bytes

## Phase 4: Texture Conversion

### GX2 Deswizzle (gx2.rs)
The GX2 GPU uses a tiled memory layout. Key parameters:
- `tileMode`: Determines the tiling pattern (e.g., Macro, Micro, Linear)
- `swizzle`: Surface swizzle value
- `pitch`: Surface pitch in pixels
- `bpp`: Bits per pixel

Key tile modes for BotW:
- `ADDR_TM_2D_TILED_THIN1` (4): Most common for 2D textures
- `ADDR_TM_LINEAR_ALIGNED` (1): Linear layout

The deswizzle produces a linear pixel buffer.

### TegraX1 Swizzle (tegra.rs)
The Switch uses GOB (Graphics Output Block) based tiling:
- GOB size: 64 bytes wide × 8 rows
- Block height: Configurable (1, 2, 4, 8, 16, 32 GOBs)
- `blockHeightLog2`: log2 of block height

Algorithm:
```
For each pixel (x, y):
  gob_x = x * bpp / (8 * 64)
  gob_y = y / 8
  block_y = gob_y / block_height

  Position within GOB uses a specific bit interleaving pattern
  Final offset = block_offset + gob_offset + intra_gob_offset
```

### Texture Format Mapping
```
GX2 Surface Format          → Switch Texture Format
GX2_SURFACE_FORMAT_TC_R8_G8_B8_A8_UNORM    → R8_G8_B8_A8_UNORM
GX2_SURFACE_FORMAT_TC_R8_G8_B8_A8_SRGB     → R8_G8_B8_A8_SRGB
GX2_SURFACE_FORMAT_T_BC1_UNORM             → BC1_UNORM
GX2_SURFACE_FORMAT_T_BC1_SRGB              → BC1_SRGB
GX2_SURFACE_FORMAT_T_BC3_UNORM             → BC3_UNORM
GX2_SURFACE_FORMAT_T_BC3_SRGB              → BC3_SRGB
GX2_SURFACE_FORMAT_T_BC4_UNORM             → BC4_UNORM
GX2_SURFACE_FORMAT_T_BC4_SNORM             → BC4_SNORM
GX2_SURFACE_FORMAT_T_BC5_UNORM             → BC5_UNORM
GX2_SURFACE_FORMAT_T_BC5_SNORM             → BC5_SNORM
```

### BNTX Container (Switch texture format)
The Switch stores textures in a BNTX container within the BFRES external files:

```
BNTX Header:
0x00    8     Magic "BNTX\0\0\0\0"
0x08    4     Version (0x00040000)
0x0C    2     BOM (0xFEFF)
0x0E    1     Alignment shift
0x0F    1     Target address size (0x40)
0x10    4     Filename offset
0x14    2     Flag
0x16    2     First block offset
0x18    4     Relocation table offset
0x1C    4     File size
```

Each texture within BNTX has an NX texture info header describing:
- Dimensions, format, mip count, array length
- Swizzle parameters (blockHeightLog2)
- Channel selectors (RGBA mapping)
- Data offset within the texture data section

## Phase 5: Material Conversion (convert.rs)

### BotW RenderState → RenderInfo Mapping

When converting from Wii U to Switch, the material's `RenderState` struct is
converted to a set of `RenderInfo` key-value string pairs:

```rust
fn convert_render_state(state: &RenderState) -> Vec<RenderInfo> {
    let mut infos = Vec::new();

    // Render mode
    infos.push(match state.mode {
        Mode::Opaque => ("gsys_render_state_mode", "opaque"),
        Mode::AlphaMask => ("gsys_render_state_mode", "mask"),
        Mode::Translucent => ("gsys_render_state_mode", "translucent"),
        Mode::Custom => ("gsys_render_state_mode", "custom"),
    });

    // Face culling
    let display_face = match (state.cull_front, state.cull_back) {
        (false, false) => "both",
        (true, false) => "back",
        (false, true) => "front",
        (true, true) => "none",
    };
    infos.push(("gsys_render_state_display_face", display_face));

    // Blend mode
    infos.push(("gsys_render_state_blend_mode", match state.blend_mode {
        BlendMode::None => "none",
        BlendMode::Color => "color",
        BlendMode::Logical => "logic",
    }));

    // Depth test
    infos.push(("gsys_depth_test_enable", bool_str(state.depth_test_enabled)));
    infos.push(("gsys_depth_test_write", bool_str(state.depth_write_enabled)));
    infos.push(("gsys_depth_test_func", compare_func_str(state.depth_func)));

    // Color blend functions
    infos.push(("gsys_color_blend_rgb_src_func", blend_func_str(state.color_src_blend)));
    infos.push(("gsys_color_blend_rgb_dst_func", blend_func_str(state.color_dst_blend)));
    infos.push(("gsys_color_blend_rgb_op", blend_op_str(state.color_combine)));
    infos.push(("gsys_color_blend_alpha_src_func", blend_func_str(state.alpha_src_blend)));
    infos.push(("gsys_color_blend_alpha_dst_func", blend_func_str(state.alpha_dst_blend)));
    infos.push(("gsys_color_blend_alpha_op", blend_op_str(state.alpha_combine)));

    // Alpha test
    infos.push(("gsys_alpha_test_enable", bool_str(state.alpha_test_enabled)));
    infos.push(("gsys_alpha_test_func", compare_func_str(state.alpha_func)));
    // gsys_alpha_test_value is a float

    infos
}
```

### Blend Function String Mapping
```
SourceAlpha         → "src_alpha"
OneMinusSourceAlpha → "one_minus_src_alpha"
SourceColor         → "src_color"
DestinationAlpha    → "dst_alpha"
Zero                → "zero"
One                 → "one"
ConstantColor       → "const_color"
ConstantAlpha       → "const_alpha"
```

### Compare Function String Mapping
```
Always      → "always"
Never       → "never"
Less        → "less"
LessEqual   → "lequal"
Greater     → "greater"
GreaterEqual → "gequal"
Equal       → "equal"
NotEqual    → "noequal"
```

## Phase 6: Animation Renaming

Switch BFRES combines all material-related animations into a single dictionary,
differentiated by name suffix:

```
Wii U Dictionary              → Switch Suffix
ShaderParamAnims              → "_fsp"
TexSrtAnims                   → "_fts"
ColorAnims                    → "_fcl"
TexPatternAnims               → "_ftp"
MaterialVisibilityAnims       → "_fvs"
BoneVisibilityAnims           → (no suffix, separate dict)
```

## Phase 7: Switch BFRES Writer (switch/)

### Header Layout (little-endian)
```
0x00    4     Magic "FRES"
0x04    4     Padding (0x20202020 = "    ")
0x08    1     Version micro (0)
0x09    1     Version minor (5)
0x0A    1     Version major (0)
0x0B    1     Version patch (3)
0x0C    2     BOM (0xFFFE = little-endian in LE byte order)
0x0E    1     Alignment shift (0x0C = log2(4096))
0x0F    1     Target address size (0x40 = 64-bit pointers)
0x10    4     Filename offset (relative, 8-byte ptr)
0x14    2     Flags
0x16    2     Block offset
0x18    4     Relocation table offset
0x1C    4     File size
0x20    ...   Section offsets (models, anims, etc.)
```

### Relocation Table (_RLT)
```
0x00    4     Magic "_RLT"
0x04    4     Position (offset of this table from file start)
0x08    4     Section count
0x0C    4     Padding

Per section (32 bytes):
0x00    8     Padding
0x08    4     Section position
0x0C    4     Section size
0x10    4     Entry index
0x14    4     Entry count

Per entry (8 bytes):
0x00    4     Position within section
0x04    2     Struct count
0x06    1     Offset count
0x07    1     Padding count
```

Sections:
1. File structure (headers, dicts)
2. Index buffer info
3. Vertex buffer data
4. Memory pool (288 bytes, 4096-aligned)
5. External files (BNTX textures)

## Test Fixtures

Located in `tests/fixtures/`:
- `*.wiiu.bfres` — Decompressed Wii U input files
- `*.switch.bfres` — Reference Switch output from BfresPlatformConverter

Test pairs:
- `Animal_Boar_Big.wiiu.bfres` → `Animal_Boar_Big.switch.bfres` (model)
- `Animal_Boar_Big.Tex1.wiiu.bfres` + `Animal_Boar_Big.Tex2.wiiu.bfres`
  → `Animal_Boar_Big.Tex.switch.bfres` (textures with mipmap merging)
- `Animal_Cat.wiiu.bfres` → `Animal_Cat.switch.bfres` (model, more complex)
- `Animal_Cat.Tex1.wiiu.bfres` + `Animal_Cat.Tex2.wiiu.bfres`
  → `Animal_Cat.Tex.switch.bfres` (textures, 19 textures)
- `Armor_740_Head.15.wiiu.bfres` → `Armor_740_Head.15.switch.bfres` (UI item icon)

## Testing Strategy

### Unit Tests (per module)
- `binary.rs`: Read/write round-trip tests for all integer types
- `wiiu/header.rs`: Parse test fixture headers, verify field values
- `wiiu/dict.rs`: Parse dictionaries, verify entry names and offsets
- `wiiu/model.rs`: Parse model, verify shape/material/skeleton counts
- `wiiu/texture.rs`: Parse texture info, verify dimensions/format
- `gx2.rs`: Deswizzle known patterns, compare to linear reference
- `tegra.rs`: Swizzle known patterns, compare to reference
- `convert.rs`: Material conversion mapping correctness

### Integration Tests
- Parse each `.wiiu.bfres` fixture, verify no errors
- Convert each fixture, compare output byte-for-byte with `.switch.bfres`
- Verify output passes `is_switch_bfres()` check
- Verify output header fields (version, BOM, file size)

### Smoke Test
- Convert all 1,102 Wii U BFRES files from Second Wind v1.9.14
- Load converted files in Ryubing emulator
- Verify game doesn't crash
