//! Internal data model for BFRES files.
//!
//! This is the intermediate representation used between the Wii U parser
//! and Switch writer. It captures all data needed for conversion.

/// Top-level BFRES file representation.
#[derive(Debug, Clone)]
pub struct BfresFile {
    pub name: String,
    pub version: (u8, u8, u8, u8),
    pub alignment: u32,
    pub models: Vec<Model>,
    pub textures: Vec<TextureInfo>,
    pub skeleton_anims: Vec<Animation>,
    pub material_anims: Vec<MaterialAnimation>,
    pub bone_vis_anims: Vec<Animation>,
    pub shape_anims: Vec<Animation>,
    pub scene_anims: Vec<Animation>,
    pub external_files: Vec<ExternalFile>,
    /// Raw Wii U sub-file categories that map to Switch material anims
    pub shader_param_anims: Vec<MaterialAnimation>,
    pub color_anims: Vec<MaterialAnimation>,
    pub tex_srt_anims: Vec<MaterialAnimation>,
    pub tex_pattern_anims: Vec<MaterialAnimation>,
    pub mat_vis_anims: Vec<Animation>,
}

#[derive(Debug, Clone)]
pub struct Model {
    pub name: String,
    pub path: String,
    pub skeleton: Skeleton,
    pub shapes: Vec<Shape>,
    pub materials: Vec<Material>,
    pub vertex_buffers: Vec<VertexBuffer>,
    pub user_data: Vec<UserData>,
    pub total_vertex_count: u32,
}

#[derive(Debug, Clone)]
pub struct Skeleton {
    pub flags: u32,
    pub bones: Vec<Bone>,
    pub smooth_indices: Vec<u16>,
    pub rigid_indices: Vec<u16>,
    pub inverse_model_matrices: Vec<[f32; 12]>,
}

#[derive(Debug, Clone)]
pub struct Bone {
    pub name: String,
    pub index: u16,
    pub parent_index: i16,
    pub smooth_matrix_index: i16,
    pub rigid_matrix_index: i16,
    pub billboard_index: i16,
    pub flags: u32,
    pub scale: [f32; 3],
    pub rotation: [f32; 4],
    pub translation: [f32; 3],
    pub user_data: Vec<UserData>,
}

#[derive(Debug, Clone)]
pub struct Shape {
    pub name: String,
    pub flags: u32,
    pub index: u16,
    pub material_index: u16,
    pub bone_index: u16,
    pub vertex_buffer_index: u16,
    pub skin_bone_indices: Vec<u16>,
    pub vertex_skin_count: u8,
    pub meshes: Vec<Mesh>,
    pub key_shapes: Vec<KeyShape>,
    pub bounding_boxes: Vec<BoundingBox>,
    pub bounding_radius: Vec<f32>,
    pub bounding_nodes: Vec<BoundingNode>,
}

#[derive(Debug, Clone)]
pub struct Mesh {
    pub primitive_type: u32,
    pub index_format: u32,
    pub index_count: u32,
    pub first_vertex: u32,
    pub sub_meshes: Vec<SubMesh>,
    pub index_data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct SubMesh {
    pub offset: u32,
    pub count: u32,
}

#[derive(Debug, Clone)]
pub struct KeyShape {
    pub name: String,
    pub index: u8,
}

#[derive(Debug, Clone, Default)]
pub struct BoundingBox {
    pub center: [f32; 3],
    pub extent: [f32; 3],
}

#[derive(Debug, Clone, Default)]
pub struct BoundingNode {
    pub left_child_index: u16,
    pub next_sibling_index: u16,
    pub right_child_index: u16,
    pub unknown: u16,
    pub sub_mesh_index: u16,
    pub sub_mesh_count: u16,
}

#[derive(Debug, Clone)]
pub struct Material {
    pub name: String,
    pub flags: u32,
    pub index: u16,
    pub render_infos: Vec<RenderInfo>,
    pub render_state: Option<RenderState>,
    pub shader_assign: Option<ShaderAssign>,
    pub shader_params: Vec<ShaderParam>,
    pub texture_refs: Vec<TextureRef>,
    pub samplers: Vec<Sampler>,
    pub user_data: Vec<UserData>,
    /// Raw shader param data (big-endian bytes from Wii U)
    pub shader_param_data: Vec<u8>,
    /// Volatile flags raw data
    pub volatile_flags: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct RenderInfo {
    pub name: String,
    pub value: RenderInfoValue,
}

#[derive(Debug, Clone)]
pub enum RenderInfoValue {
    Int32(Vec<i32>),
    Float(Vec<f32>),
    String(Vec<String>),
}

#[derive(Debug, Clone)]
pub struct RenderState {
    pub flags: u32,
    pub polygon_control: u32,
    pub depth_control: u32,
    pub alpha_control: u32,
    pub alpha_ref_value: f32,
    pub blend_control: u32,
    pub blend_color: [f32; 4],
}

impl RenderState {
    pub fn flags_mode(&self) -> u32 {
        self.flags & 0x03
    }

    pub fn flags_blend_mode(&self) -> u32 {
        (self.flags >> 4) & 0x03
    }

    pub fn cull_front(&self) -> bool {
        (self.polygon_control >> 0) & 1 != 0
    }

    pub fn cull_back(&self) -> bool {
        (self.polygon_control >> 1) & 1 != 0
    }

    pub fn depth_test_enabled(&self) -> bool {
        (self.depth_control >> 1) & 1 != 0
    }

    pub fn depth_write_enabled(&self) -> bool {
        (self.depth_control >> 2) & 1 != 0
    }

    pub fn depth_func(&self) -> u32 {
        (self.depth_control >> 4) & 0x07
    }

    pub fn alpha_test_enabled(&self) -> bool {
        (self.alpha_control >> 0) & 1 != 0
    }

    pub fn alpha_func(&self) -> u32 {
        (self.alpha_control >> 8) & 0x07
    }

    pub fn color_src_blend(&self) -> u32 {
        (self.blend_control >> 0) & 0x1F
    }

    pub fn color_combine(&self) -> u32 {
        (self.blend_control >> 5) & 0x07
    }

    pub fn color_dst_blend(&self) -> u32 {
        (self.blend_control >> 8) & 0x1F
    }

    pub fn alpha_src_blend(&self) -> u32 {
        (self.blend_control >> 16) & 0x1F
    }

    pub fn alpha_combine(&self) -> u32 {
        (self.blend_control >> 21) & 0x07
    }

    pub fn alpha_dst_blend(&self) -> u32 {
        (self.blend_control >> 24) & 0x1F
    }
}

#[derive(Debug, Clone)]
pub struct ShaderAssign {
    pub shader_archive_name: String,
    pub shading_model_name: String,
    pub attrib_assigns: Vec<(String, String)>,
    pub sampler_assigns: Vec<(String, String)>,
    pub shader_options: Vec<(String, String)>,
    pub revision: u32,
}

#[derive(Debug, Clone)]
pub struct ShaderParam {
    pub name: String,
    pub param_type: u8,
    pub data_offset: u16,
    pub callback_pointer: u32,
    pub depend_index: i16,
    pub depend_count: i16,
}

#[derive(Debug, Clone)]
pub struct TextureRef {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Sampler {
    pub name: String,
    pub gx2_sampler_data: [u32; 3],
}

#[derive(Debug, Clone)]
pub struct TextureInfo {
    pub name: String,
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub mip_count: u32,
    pub format: u32,
    pub dim: u32,
    pub tile_mode: u32,
    pub swizzle: u32,
    pub array_length: u32,
    pub pitch: u32,
    pub channel_selectors: [u8; 4],
    pub aa_mode: u32,
    pub use_flags: u32,
    /// Deswizzled linear pixel data per mip level.
    pub mip_data: Vec<Vec<u8>>,
    /// Raw surface data (before deswizzle, for reference).
    pub surface_data: Vec<u8>,
    /// Raw mipmap data from Tex2 file (if any).
    pub extra_mip_data: Vec<u8>,
    pub mip_offsets: Vec<u32>,
    /// Extra swizzle value from Tex2 file.
    pub mip_swizzle: u32,
    /// Alignment
    pub alignment: u32,
    pub image_size: u32,
    pub mip_size: u32,
}

#[derive(Debug, Clone)]
pub struct VertexBuffer {
    pub index: u16,
    pub vertex_count: u32,
    pub vertex_skin_count: u8,
    pub attributes: Vec<VertexAttribute>,
    pub buffers: Vec<BufferData>,
}

#[derive(Debug, Clone)]
pub struct VertexAttribute {
    pub name: String,
    pub format: u32,
    pub offset: u16,
    pub buffer_index: u8,
}

#[derive(Debug, Clone)]
pub struct BufferData {
    pub stride: u16,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct Animation {
    pub name: String,
    pub path: String,
    pub flags: u32,
    pub frame_count: i32,
    pub baked_size: u32,
    pub raw_data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct MaterialAnimation {
    pub name: String,
    pub path: String,
    pub flags: u32,
    pub frame_count: i32,
    pub baked_size: u32,
    pub raw_data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct ExternalFile {
    pub name: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct UserData {
    pub name: String,
    pub value: UserDataValue,
}

#[derive(Debug, Clone)]
pub enum UserDataValue {
    Int32(Vec<i32>),
    Float(Vec<f32>),
    String(Vec<String>),
    Bytes(Vec<u8>),
}
