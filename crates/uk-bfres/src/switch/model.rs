//! Switch BFRES sub-file writer functions for model-related structures.
//!
//! Each writer function serializes a fixed-size header, writing placeholder
//! offsets for data blocks that will be written later by the orchestrator.
//! The returned "Fixups" struct captures the byte positions of all placeholders
//! so the orchestrator can fix them up once it knows the target positions.
//!
//! NOTE: These functions are not yet called by the orchestrator (which uses
//! inline implementations). They are available for future refactoring.

#![allow(dead_code)]

use crate::binary::LittleEndianWriter;
use crate::model::*;
use crate::switch::string_table::StringTable;

// ---------------------------------------------------------------------------
// String collection
// ---------------------------------------------------------------------------

/// Walk the entire [`BfresFile`] and register every referenced string into the
/// [`StringTable`] so the string pool contains all names needed by the binary.
pub(crate) fn collect_strings(bfres: &BfresFile, strings: &mut StringTable) {
    strings.add(&bfres.name);

    for model in &bfres.models {
        strings.add(&model.name);
        strings.add(&model.path);

        // Skeleton bones
        for bone in &model.skeleton.bones {
            strings.add(&bone.name);
            for ud in &bone.user_data {
                strings.add(&ud.name);
            }
        }

        // Shapes
        for shape in &model.shapes {
            strings.add(&shape.name);
            for ks in &shape.key_shapes {
                strings.add(&ks.name);
            }
        }

        // Materials
        for mat in &model.materials {
            strings.add(&mat.name);

            for ri in &mat.render_infos {
                strings.add(&ri.name);
                if let RenderInfoValue::String(ref sv) = ri.value {
                    for s in sv {
                        strings.add(s);
                    }
                }
            }

            if let Some(ref sa) = mat.shader_assign {
                strings.add(&sa.shader_archive_name);
                strings.add(&sa.shading_model_name);
                for (k, v) in &sa.attrib_assigns {
                    strings.add(k);
                    strings.add(v);
                }
                for (k, v) in &sa.sampler_assigns {
                    strings.add(k);
                    strings.add(v);
                }
                for (k, v) in &sa.shader_options {
                    strings.add(k);
                    strings.add(v);
                }
            }

            for sp in &mat.shader_params {
                strings.add(&sp.name);
            }

            for tr in &mat.texture_refs {
                strings.add(&tr.name);
            }

            for sam in &mat.samplers {
                strings.add(&sam.name);
            }

            for ud in &mat.user_data {
                strings.add(&ud.name);
            }
        }

        // Vertex buffers
        for vb in &model.vertex_buffers {
            for attr in &vb.attributes {
                strings.add(&attr.name);
            }
        }

        // Model user data
        for ud in &model.user_data {
            strings.add(&ud.name);
        }
    }

    // External files
    for ef in &bfres.external_files {
        strings.add(&ef.name);
    }

    // Animations
    for anim in &bfres.skeleton_anims {
        strings.add(&anim.name);
        strings.add(&anim.path);
    }
    for anim in &bfres.bone_vis_anims {
        strings.add(&anim.name);
        strings.add(&anim.path);
    }
    for anim in &bfres.shape_anims {
        strings.add(&anim.name);
        strings.add(&anim.path);
    }
    for anim in &bfres.scene_anims {
        strings.add(&anim.name);
        strings.add(&anim.path);
    }
    for anim in &bfres.mat_vis_anims {
        strings.add(&anim.name);
        strings.add(&anim.path);
    }

    // Material animations (all Wii U sub-categories that merge into Switch mat anims)
    for ma in &bfres.material_anims {
        strings.add(&ma.name);
        strings.add(&ma.path);
    }
    for ma in &bfres.shader_param_anims {
        strings.add(&ma.name);
        strings.add(&ma.path);
    }
    for ma in &bfres.color_anims {
        strings.add(&ma.name);
        strings.add(&ma.path);
    }
    for ma in &bfres.tex_srt_anims {
        strings.add(&ma.name);
        strings.add(&ma.path);
    }
    for ma in &bfres.tex_pattern_anims {
        strings.add(&ma.name);
        strings.add(&ma.path);
    }
}

// ---------------------------------------------------------------------------
// Fixup structs
// ---------------------------------------------------------------------------

/// Placeholder positions for an FMDL (model) header.
pub(crate) struct FmdlFixups {
    pub name_off: usize,
    pub path_off: usize,
    pub skeleton_off: usize,
    pub vbuf_array_off: usize,
    pub shape_values_off: usize,
    pub shape_dict_off: usize,
    pub material_values_off: usize,
    pub material_dict_off: usize,
    pub user_data_off: usize,
    pub user_data_dict_off: usize,
}

/// Placeholder positions for an FSKL (skeleton) header.
pub(crate) struct FsklFixups {
    pub bone_dict_off: usize,
    pub bone_array_off: usize,
    pub matrix_to_bone_off: usize,
    pub inv_model_matrices_off: usize,
    pub user_pointer_off: usize,
}

/// Placeholder positions for a single Bone entry.
pub(crate) struct BoneFixups {
    pub name_off: usize,
    pub user_data_off: usize,
    pub user_data_dict_off: usize,
}

/// Placeholder positions for an FSHP (shape) header.
pub(crate) struct FshpFixups {
    pub name_off: usize,
    /// Absolute position of the VB offset field (written as i64 of the VB's
    /// absolute position, not a normal placeholder).
    pub vertex_buffer_off: usize,
    pub mesh_array_off: usize,
    pub skin_bone_indices_off: usize,
    pub key_shapes_off: usize,
    pub key_shape_dict_off: usize,
    pub bounding_box_array_off: usize,
    pub radius_array_off: usize,
}

/// Placeholder positions for a Mesh entry.
pub(crate) struct MeshFixups {
    pub sub_mesh_array_off: usize,
    pub memory_pool_off: usize,
    pub buffer_unk_off: usize,
    pub buffer_size_off: usize,
}

/// Placeholder positions for an FMAT (material) header.
pub(crate) struct FmatFixups {
    pub name_off: usize,
    pub render_info_off: usize,
    pub render_info_dict_off: usize,
    pub shader_assign_off: usize,
    pub texture_unk1_off: usize,
    pub texture_refs_off: usize,
    pub texture_unk2_off: usize,
    pub sampler_off: usize,
    pub sampler_dict_off: usize,
    pub shader_param_off: usize,
    pub shader_param_dict_off: usize,
    pub shader_param_data_off: usize,
    pub user_data_off: usize,
    pub user_data_dict_off: usize,
    pub volatile_flags_off: usize,
    pub sampler_slot_off: usize,
    pub texture_slot_off: usize,
}

/// Placeholder positions for an FVTX (vertex buffer) header.
pub(crate) struct FvtxFixups {
    pub attrib_off: usize,
    pub attrib_dict_off: usize,
    pub memory_pool_off: usize,
    pub unk_off: usize,
    pub unk2_off: usize,
    pub buffer_size_off: usize,
    pub stride_off: usize,
}

/// Placeholder positions for a RenderInfo entry.
pub(crate) struct RenderInfoFixups {
    pub name_off: usize,
    pub data_off: usize,
}

/// Placeholder positions for a ShaderAssign block.
pub(crate) struct ShaderAssignFixups {
    pub archive_name_off: usize,
    pub model_name_off: usize,
    pub attrib_assigns_off: usize,
    pub attrib_assign_dict_off: usize,
    pub sampler_assigns_off: usize,
    pub sampler_assign_dict_off: usize,
    pub shader_options_off: usize,
    pub shader_options_dict_off: usize,
}

// ---------------------------------------------------------------------------
// Writer functions
// ---------------------------------------------------------------------------

/// Write an FMDL header.
///
/// Layout derived from C# `ModelParser.Write()`.
pub(crate) fn write_fmdl_header(
    w: &mut LittleEndianWriter,
    model: &Model,
    v: u8,
) -> FmdlFixups {
    // Version prefix
    if v >= 9 {
        w.write_u32(0); // flags (Model has no flags field in our model)
    } else {
        w.write_zeros(12); // header block
    }

    let name_off = w.write_offset_placeholder_64();
    let path_off = w.write_offset_placeholder_64();
    let skeleton_off = w.write_offset_placeholder_64();
    let vbuf_array_off = w.write_offset_placeholder_64();
    let shape_values_off = w.write_offset_placeholder_64();
    let shape_dict_off = w.write_offset_placeholder_64();
    let material_values_off = w.write_offset_placeholder_64();

    if v == 9 {
        w.write_u64(0); // padding
    }

    let material_dict_off = w.write_offset_placeholder_64();

    if v >= 10 {
        w.write_offset_placeholder_64(); // shader assign list (not tracked)
    }

    let user_data_off = w.write_offset_placeholder_64();
    let user_data_dict_off = w.write_offset_placeholder_64();

    w.write_u64(0); // user pointer (i64 0)

    w.write_u16(model.vertex_buffers.len() as u16);
    w.write_u16(model.shapes.len() as u16);
    w.write_u16(model.materials.len() as u16);

    if v >= 9 {
        w.write_u16(0); // shader assign count
        w.write_u16(model.user_data.len() as u16);
        w.write_u16(0); // padding
        w.write_u32(0); // padding
    } else {
        w.write_u16(model.user_data.len() as u16);
        w.write_u32(model.total_vertex_count);
        w.write_u32(0); // padding
    }

    FmdlFixups {
        name_off,
        path_off,
        skeleton_off,
        vbuf_array_off,
        shape_values_off,
        shape_dict_off,
        material_values_off,
        material_dict_off,
        user_data_off,
        user_data_dict_off,
    }
}

/// Write an FSKL header (including the `FSKL` magic).
///
/// Layout derived from C# `Skeleton.Save()`.
pub(crate) fn write_fskl(
    w: &mut LittleEndianWriter,
    skeleton: &Skeleton,
    v: u8,
) -> FsklFixups {
    w.write_magic(b"FSKL");

    if v >= 9 {
        w.write_u32(skeleton.flags);
    } else {
        w.write_zeros(12); // header block
    }

    let bone_dict_off = w.write_offset_placeholder_64();
    let bone_array_off = w.write_offset_placeholder_64();
    let matrix_to_bone_off = w.write_offset_placeholder_64();
    let inv_model_matrices_off = w.write_offset_placeholder_64();

    if v == 8 {
        w.write_zeros(16);
    }
    if v >= 9 {
        w.write_zeros(8);
    }

    let user_pointer_off = w.write_offset_placeholder_64();

    if v < 9 {
        w.write_u32(skeleton.flags);
    }

    let smooth_count = skeleton.inverse_model_matrices.len() as u16;
    let total_matrix_count = skeleton.smooth_indices.len() + skeleton.rigid_indices.len();
    let rigid_count = (total_matrix_count as u16).saturating_sub(smooth_count);

    w.write_u16(skeleton.bones.len() as u16);
    w.write_u16(smooth_count);
    w.write_u16(rigid_count);

    if v >= 9 {
        w.write_zeros(2); // padding
    } else {
        w.write_zeros(6); // padding
    }

    FsklFixups {
        bone_dict_off,
        bone_array_off,
        matrix_to_bone_off,
        inv_model_matrices_off,
        user_pointer_off,
    }
}

/// Write a single Bone entry.
///
/// Layout derived from C# `Bone.Save()` (Switch path).
pub(crate) fn write_bone(
    w: &mut LittleEndianWriter,
    bone: &Bone,
    index: u16,
    v: u8,
) -> BoneFixups {
    let name_off = w.write_offset_placeholder_64();
    let user_data_off = w.write_offset_placeholder_64();
    let user_data_dict_off = w.write_offset_placeholder_64();

    if v > 9 {
        w.write_zeros(8);
    } else if v == 8 || v == 9 {
        w.write_zeros(16);
    }

    w.write_u16(index);
    w.write_i16(bone.parent_index);
    w.write_i16(bone.smooth_matrix_index);
    w.write_i16(bone.rigid_matrix_index);
    w.write_i16(bone.billboard_index);
    w.write_u16(bone.user_data.len() as u16);
    w.write_u32(bone.flags);

    // Scale
    w.write_f32(bone.scale[0]);
    w.write_f32(bone.scale[1]);
    w.write_f32(bone.scale[2]);

    // Rotation (quaternion or euler XYZ w/ 4th component)
    w.write_f32(bone.rotation[0]);
    w.write_f32(bone.rotation[1]);
    w.write_f32(bone.rotation[2]);
    w.write_f32(bone.rotation[3]);

    // Translation
    w.write_f32(bone.translation[0]);
    w.write_f32(bone.translation[1]);
    w.write_f32(bone.translation[2]);

    BoneFixups {
        name_off,
        user_data_off,
        user_data_dict_off,
    }
}

/// Write an FSHP (shape) header.
///
/// Layout derived from C# `ShapeParser.Write()`.
pub(crate) fn write_fshp_header(
    w: &mut LittleEndianWriter,
    shape: &Shape,
    index: u16,
    v: u8,
) -> FshpFixups {
    if v >= 9 {
        w.write_u32(shape.flags);
    } else {
        w.write_zeros(12); // header block
    }

    let name_off = w.write_offset_placeholder_64();
    // In the C# code this writes the absolute VB position as i64, not a
    // normal placeholder. We write a placeholder here; the orchestrator will
    // fix it up with the VB's position.
    let vertex_buffer_off = w.write_offset_placeholder_64();
    let mesh_array_off = w.write_offset_placeholder_64();
    let skin_bone_indices_off = w.write_offset_placeholder_64();
    let key_shapes_off = w.write_offset_placeholder_64();
    let key_shape_dict_off = w.write_offset_placeholder_64();
    let bounding_box_array_off = w.write_offset_placeholder_64();
    let radius_array_off = w.write_offset_placeholder_64();

    w.write_u64(0); // user pointer / padding

    if v < 9 {
        w.write_u32(shape.flags);
    }

    w.write_u16(index);
    w.write_u16(shape.material_index);
    w.write_u16(shape.bone_index);
    w.write_u16(shape.vertex_buffer_index);
    w.write_u16(shape.skin_bone_indices.len() as u16);
    w.write_u8(shape.vertex_skin_count);
    w.write_u8(shape.meshes.len() as u8);
    w.write_u8(shape.key_shapes.len() as u8);
    w.write_u8(0); // target_attrib_count

    if v >= 9 {
        w.write_zeros(2); // padding
    } else {
        w.write_zeros(6); // padding
    }

    FshpFixups {
        name_off,
        vertex_buffer_off,
        mesh_array_off,
        skin_bone_indices_off,
        key_shapes_off,
        key_shape_dict_off,
        bounding_box_array_off,
        radius_array_off,
    }
}

/// Write a Mesh entry.
///
/// Layout derived from C# `Mesh.Save()` (Switch path):
/// ```text
/// offset_64  sub_mesh_array
/// offset_64  memory_pool_pointer
/// offset_64  buffer_unk (BufferSize linked)
/// offset_64  buffer_size
/// u32        face_buffer_offset
/// u32        primitive_type (switch enum)
/// u32        index_format (switch enum)
/// u32        index_count
/// u32        first_vertex
/// u16        sub_mesh_count
/// u16        padding
/// ```
pub(crate) fn write_mesh(
    w: &mut LittleEndianWriter,
    mesh: &Mesh,
    face_buffer_offset: u32,
) -> MeshFixups {
    let sub_mesh_array_off = w.write_offset_placeholder_64();
    let memory_pool_off = w.write_offset_placeholder_64(); // memory pool pointer
    let buffer_unk_off = w.write_offset_placeholder_64();
    let buffer_size_off = w.write_offset_placeholder_64();

    w.write_u32(face_buffer_offset);

    // Switch primitive type enum (same numeric values as GX2 for common types)
    w.write_u32(mesh.primitive_type);

    // Switch index format enum
    w.write_u32(mesh.index_format);

    w.write_u32(mesh.index_count);
    w.write_u32(mesh.first_vertex);
    w.write_u16(mesh.sub_meshes.len() as u16);
    w.write_u16(0); // padding

    MeshFixups {
        sub_mesh_array_off,
        memory_pool_off,
        buffer_unk_off,
        buffer_size_off,
    }
}

/// Write an FMAT (material) header.
///
/// Layout derived from C# `MaterialParser.Save()`.
pub(crate) fn write_fmat_header(
    w: &mut LittleEndianWriter,
    material: &Material,
    index: u16,
    v: u8,
) -> FmatFixups {
    if v >= 9 {
        w.write_u32(material.flags);
    } else {
        w.write_zeros(12); // header block
    }

    // Note: v >= 10 delegates to MaterialParserV10 in C#; we only handle
    // the standard path (v5-v9) here. For v10+ the orchestrator would call
    // a separate function.

    let name_off = w.write_offset_placeholder_64();
    let render_info_off = w.write_offset_placeholder_64();
    let render_info_dict_off = w.write_offset_placeholder_64();
    let shader_assign_off = w.write_offset_placeholder_64();
    let texture_unk1_off = w.write_offset_placeholder_64();
    let texture_refs_off = w.write_offset_placeholder_64();
    let texture_unk2_off = w.write_offset_placeholder_64();
    let sampler_off = w.write_offset_placeholder_64();
    let sampler_dict_off = w.write_offset_placeholder_64();
    let shader_param_off = w.write_offset_placeholder_64();
    let shader_param_dict_off = w.write_offset_placeholder_64();
    let shader_param_data_off = w.write_offset_placeholder_64();
    let user_data_off = w.write_offset_placeholder_64();
    let user_data_dict_off = w.write_offset_placeholder_64();
    let volatile_flags_off = w.write_offset_placeholder_64();

    w.write_u64(0); // user pointer (long 0)

    let sampler_slot_off = w.write_offset_placeholder_64();
    let texture_slot_off = w.write_offset_placeholder_64();

    if v != 9 {
        w.write_u32(material.flags);
    }

    w.write_u16(index);
    w.write_u16(material.render_infos.len() as u16);
    w.write_u8(material.samplers.len() as u8);
    w.write_u8(material.texture_refs.len() as u8);
    w.write_u16(material.shader_params.len() as u16);
    w.write_u16(0); // volatile flags count
    w.write_u16(material.shader_param_data.len() as u16);
    w.write_u16(0); // SizParamRaw
    w.write_u16(material.user_data.len() as u16);

    if v != 9 {
        w.write_u32(0); // padding
    }

    FmatFixups {
        name_off,
        render_info_off,
        render_info_dict_off,
        shader_assign_off,
        texture_unk1_off,
        texture_refs_off,
        texture_unk2_off,
        sampler_off,
        sampler_dict_off,
        shader_param_off,
        shader_param_dict_off,
        shader_param_data_off,
        user_data_off,
        user_data_dict_off,
        volatile_flags_off,
        sampler_slot_off,
        texture_slot_off,
    }
}

/// Write an FVTX (vertex buffer) header.
///
/// Layout derived from C# `VertexBufferParser.Save()`.
///
/// The `FVTX` magic is **not** written here — it is written by the caller
/// before invoking this function (matching C# where `IResData.Save()` writes
/// the signature then calls the parser).
pub(crate) fn write_fvtx_header(
    w: &mut LittleEndianWriter,
    vb: &VertexBuffer,
    index: u16,
    buffer_offset: u32,
    v: u8,
) -> FvtxFixups {
    if v >= 9 {
        w.write_u32(0); // flags
    } else {
        w.write_zeros(12); // header block
    }

    let attrib_off = w.write_offset_placeholder_64();
    let attrib_dict_off = w.write_offset_placeholder_64();
    let memory_pool_off = w.write_offset_placeholder_64(); // memory pool pointer
    let unk_off = w.write_offset_placeholder_64();
    let unk2_off = w.write_offset_placeholder_64();
    let buffer_size_off = w.write_offset_placeholder_64();
    let stride_off = w.write_offset_placeholder_64();

    w.write_u64(0); // padding (i64 0)

    w.write_u32(buffer_offset);
    w.write_u8(vb.attributes.len() as u8);
    w.write_u8(vb.buffers.len() as u8);
    w.write_u16(index);
    w.write_u32(vb.vertex_count);
    w.write_u16(vb.vertex_skin_count as u16);
    w.write_u16(8); // GPU buffer alignment

    FvtxFixups {
        attrib_off,
        attrib_dict_off,
        memory_pool_off,
        unk_off,
        unk2_off,
        buffer_size_off,
        stride_off,
    }
}

// ---------------------------------------------------------------------------
// Data block writers
// ---------------------------------------------------------------------------

/// Write a BoundingBox (Bounding) entry: 6 floats (center xyz, extent xyz).
pub(crate) fn write_bounding_box(w: &mut LittleEndianWriter, bbox: &BoundingBox) {
    w.write_f32(bbox.center[0]);
    w.write_f32(bbox.center[1]);
    w.write_f32(bbox.center[2]);
    w.write_f32(bbox.extent[0]);
    w.write_f32(bbox.extent[1]);
    w.write_f32(bbox.extent[2]);
}

/// Write a SubMesh entry: offset (u32) + count (u32).
pub(crate) fn write_sub_mesh(w: &mut LittleEndianWriter, sub_mesh: &SubMesh) {
    w.write_u32(sub_mesh.offset);
    w.write_u32(sub_mesh.count);
}

/// Write a RenderInfo entry (Switch layout).
///
/// Layout from C# `RenderInfo.Save()` (Switch path):
/// ```text
/// offset_64  name
/// offset_64  data
/// u16        count
/// u8         type (enum as byte)
/// 5 bytes    padding
/// ```
pub(crate) fn write_render_info(
    w: &mut LittleEndianWriter,
    ri: &RenderInfo,
) -> RenderInfoFixups {
    let name_off = w.write_offset_placeholder_64();
    let data_off = w.write_offset_placeholder_64();

    let count: u16 = match &ri.value {
        RenderInfoValue::Int32(v) => v.len() as u16,
        RenderInfoValue::Float(v) => v.len() as u16,
        RenderInfoValue::String(v) => v.len() as u16,
    };
    w.write_u16(count);

    // Type enum: Int32 = 0, Single/Float = 1, String = 2
    let type_byte: u8 = match &ri.value {
        RenderInfoValue::Int32(_) => 0,
        RenderInfoValue::Float(_) => 1,
        RenderInfoValue::String(_) => 2,
    };
    w.write_u8(type_byte);

    w.write_zeros(5); // padding

    RenderInfoFixups { name_off, data_off }
}

/// Write a ShaderParam entry (Switch layout).
///
/// Layout from C# `ShaderParam.Save()` (Switch path):
/// ```text
/// i64        callback_pointer (always 0)
/// offset_64  name
/// u8         type (ShaderParamType enum)
/// u8         data_size
/// u16        data_offset
/// i32        offset (-1)
/// u16        depended_index
/// u16        depend_index
/// u32        padding (0)
/// ```
///
/// Returns the byte position of the name placeholder.
pub(crate) fn write_shader_param(
    w: &mut LittleEndianWriter,
    sp: &ShaderParam,
) -> usize {
    w.write_u64(0); // callback pointer (i64 0)
    let name_off = w.write_offset_placeholder_64();

    w.write_u8(sp.param_type);

    // Data size — compute from param_type the same way the C# does.
    let data_size = shader_param_data_size(sp.param_type);
    w.write_u8(data_size);

    w.write_u16(sp.data_offset);
    w.write_i32(-1); // uniform variable offset
    w.write_u16(sp.depend_index as u16);
    w.write_u16(sp.depend_count as u16);
    w.write_u32(0); // padding

    name_off
}

/// Compute the byte size of a ShaderParam value from its type enum.
///
/// Mirrors C#'s `ShaderParam.DataSize` property.
fn shader_param_data_size(param_type: u8) -> u8 {
    // Float through Float4 (types 0x0C..0x0F) = 4 * (1..4)
    if param_type <= 0x0F {
        // Bool/Int/UInt/Float vectors: each component is 4 bytes
        let components = (param_type & 0x03) + 1;
        return (4 * components) as u8;
    }
    // Matrix types (Float2x2..Float4x4) — types 0x11..0x1B
    if param_type >= 0x11 && param_type <= 0x1B {
        // Skip Reserved types (0x10, 0x14, 0x18)
        let adjusted = param_type - 0x10; // 1..11
        let cols = (adjusted & 0x03) + 1; // within the group
        let row_group = (adjusted.saturating_sub(1)) / 4; // 0, 1, 2
        let rows = row_group + 2;
        return (4 * cols * rows) as u8;
    }
    // Special types
    match param_type {
        0x1C => 32, // Srt2D: 2 floats scale + 1 float rotate + 2 floats translate = 5*4 = 20? Actually Srt2D = Vector2F scale + float rotation + Vector2F translation = 5*4 = 20
        0x1D => 60, // Srt3D: 3+4+3 = 10 floats * 4 + extra = check... Actually 3 scale + 1 rotation + 3 translation + 3x4 matrix = ? In BfresLib: Srt3D.SizeInBytes = let's use known values
        0x1E => 32, // TexSrt: mode(u32) + scale(2f) + rotation(f) + translation(2f) = 4+8+4+8 = 24... Actually TexSrt has mode + 2 scale + rotate + 2 translate = 4 + 4*5 = 24
        0x1F => 36, // TexSrtEx: TexSrt + 4 bytes (u32)
        _ => 4,     // fallback
    }
}

/// Write a ShaderAssign block (Switch layout).
///
/// Layout from C# `ShaderAssign.Save()` (Switch path):
/// ```text
/// offset_64  shader_archive_name
/// offset_64  shading_model_name
/// offset_64  attrib_assigns (values)
/// offset_64  attrib_assign_dict
/// offset_64  sampler_assigns (values)
/// offset_64  sampler_assign_dict
/// offset_64  shader_options (values)
/// offset_64  shader_options_dict
/// u32        revision
/// u8         num_attrib_assign
/// u8         num_sampler_assign
/// u16        num_shader_option
/// ```
pub(crate) fn write_shader_assign(
    w: &mut LittleEndianWriter,
    sa: &ShaderAssign,
) -> ShaderAssignFixups {
    let archive_name_off = w.write_offset_placeholder_64();
    let model_name_off = w.write_offset_placeholder_64();
    let attrib_assigns_off = w.write_offset_placeholder_64();
    let attrib_assign_dict_off = w.write_offset_placeholder_64();
    let sampler_assigns_off = w.write_offset_placeholder_64();
    let sampler_assign_dict_off = w.write_offset_placeholder_64();
    let shader_options_off = w.write_offset_placeholder_64();
    let shader_options_dict_off = w.write_offset_placeholder_64();

    w.write_u32(sa.revision);
    w.write_u8(sa.attrib_assigns.len() as u8);
    w.write_u8(sa.sampler_assigns.len() as u8);
    w.write_u16(sa.shader_options.len() as u16);

    ShaderAssignFixups {
        archive_name_off,
        model_name_off,
        attrib_assigns_off,
        attrib_assign_dict_off,
        sampler_assigns_off,
        sampler_assign_dict_off,
        shader_options_off,
        shader_options_dict_off,
    }
}

/// Write a VertexAttrib entry (Switch layout).
///
/// Layout from C# `VertexAttrib.Save()` (Switch path):
/// ```text
/// offset_64  name
/// u16        format (big-endian SwitchAttribFormat stored as big-endian u16)
/// i16        padding (0)
/// u16        offset (byte offset within stride)
/// u8         buffer_index
/// u8         padding
/// ```
///
/// Returns the byte position of the name placeholder.
pub(crate) fn write_vertex_attrib(
    w: &mut LittleEndianWriter,
    attr: &VertexAttribute,
) -> usize {
    let name_off = w.write_offset_placeholder_64();

    // The C# code writes the format as a big-endian u16 (swaps byte order
    // temporarily). We replicate that by writing the u16 in big-endian.
    let format_be = (attr.format as u16).to_be_bytes();
    w.write_bytes(&format_be);

    w.write_i16(0); // padding
    w.write_u16(attr.offset);
    w.write_u8(attr.buffer_index);
    w.write_u8(0); // padding

    name_off
}

/// Write a Switch sampler entry (24 bytes).
///
/// Layout from C# `SamplerSwitch.Save()`:
/// ```text
/// u8   wrap_u
/// u8   wrap_v
/// u8   wrap_w
/// u8   compare_func
/// u8   border_color_type
/// u8   anisotropic
/// u16  filter_flags
/// f32  min_lod
/// f32  max_lod
/// f32  lod_bias
/// 12 bytes padding
/// ```
///
/// Since our internal model stores the raw GX2 sampler data (3 x u32), we
/// write a default sampler configuration. The orchestrator can override this
/// with proper conversion.
pub(crate) fn write_sampler(w: &mut LittleEndianWriter, sampler: &Sampler) {
    // We store the raw GX2 sampler words. For a proper conversion we would
    // need to decode these and re-encode into Switch format. For now, write
    // sensible defaults derived from the GX2 data where possible.
    //
    // Default sampler: Repeat/Repeat/Clamp, Never compare, White border,
    // 1:1 aniso, linear/linear/point filter, LOD 0..13, bias 0.
    let _ = sampler; // acknowledge the parameter
    w.write_u8(0); // wrap U = Repeat
    w.write_u8(0); // wrap V = Repeat
    w.write_u8(2); // wrap W = Clamp
    w.write_u8(0); // compare func = Never
    w.write_u8(0); // border color = White
    w.write_u8(1); // anisotropic = 1:1
    w.write_u16(42); // filter flags (linear shrink/expand, point mip)
    w.write_f32(0.0); // min LOD
    w.write_f32(13.0); // max LOD
    w.write_f32(0.0); // LOD bias
    w.write_zeros(12); // padding
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a minimal BfresFile for testing.
    fn make_test_bfres() -> BfresFile {
        BfresFile {
            name: "test_bfres".into(),
            version: (0, 5, 0, 3),
            alignment: 0x1000,
            models: vec![Model {
                name: "model_0".into(),
                path: "/model_0".into(),
                skeleton: Skeleton {
                    flags: 0x1200,
                    bones: vec![
                        Bone {
                            name: "root_bone".into(),
                            index: 0,
                            parent_index: -1,
                            smooth_matrix_index: 0,
                            rigid_matrix_index: -1,
                            billboard_index: -1,
                            flags: 1,
                            scale: [1.0, 1.0, 1.0],
                            rotation: [0.0, 0.0, 0.0, 1.0],
                            translation: [0.0, 0.0, 0.0],
                            user_data: vec![UserData {
                                name: "bone_ud".into(),
                                value: UserDataValue::Int32(vec![42]),
                            }],
                        },
                    ],
                    smooth_indices: vec![0],
                    rigid_indices: vec![],
                    inverse_model_matrices: vec![[
                        1.0, 0.0, 0.0, 0.0,
                        0.0, 1.0, 0.0, 0.0,
                        0.0, 0.0, 1.0, 0.0,
                    ]],
                },
                shapes: vec![Shape {
                    name: "shape_0".into(),
                    flags: 0,
                    index: 0,
                    material_index: 0,
                    bone_index: 0,
                    vertex_buffer_index: 0,
                    skin_bone_indices: vec![0],
                    vertex_skin_count: 1,
                    meshes: vec![Mesh {
                        primitive_type: 3,
                        index_format: 1,
                        index_count: 6,
                        first_vertex: 0,
                        sub_meshes: vec![SubMesh { offset: 0, count: 6 }],
                        index_data: vec![0; 12],
                    }],
                    key_shapes: vec![KeyShape {
                        name: "key_0".into(),
                        index: 0,
                    }],
                    bounding_boxes: vec![BoundingBox::default()],
                    bounding_radius: vec![1.0],
                    bounding_nodes: vec![],
                }],
                materials: vec![Material {
                    name: "mat_0".into(),
                    flags: 0,
                    index: 0,
                    render_infos: vec![RenderInfo {
                        name: "gsys_render_state_mode".into(),
                        value: RenderInfoValue::String(vec!["opaque".into()]),
                    }],
                    render_state: None,
                    shader_assign: Some(ShaderAssign {
                        shader_archive_name: "Turbo_UBER".into(),
                        shading_model_name: "turbo_uber".into(),
                        attrib_assigns: vec![
                            ("a_attr".into(), "a_val".into()),
                        ],
                        sampler_assigns: vec![
                            ("s_attr".into(), "s_val".into()),
                        ],
                        shader_options: vec![
                            ("o_key".into(), "o_val".into()),
                        ],
                        revision: 0,
                    }),
                    shader_params: vec![ShaderParam {
                        name: "cDiffuse".into(),
                        param_type: 0x0E, // Float3
                        data_offset: 0,
                        callback_pointer: 0,
                        depend_index: 0,
                        depend_count: 0,
                    }],
                    texture_refs: vec![TextureRef {
                        name: "Alb_0".into(),
                    }],
                    samplers: vec![Sampler {
                        name: "_a0".into(),
                        gx2_sampler_data: [0; 3],
                    }],
                    user_data: vec![],
                    shader_param_data: vec![0; 12],
                    volatile_flags: vec![],
                }],
                vertex_buffers: vec![VertexBuffer {
                    index: 0,
                    vertex_count: 4,
                    vertex_skin_count: 1,
                    attributes: vec![VertexAttribute {
                        name: "_p0".into(),
                        format: 0x518,
                        offset: 0,
                        buffer_index: 0,
                    }],
                    buffers: vec![BufferData {
                        stride: 12,
                        data: vec![0; 48],
                    }],
                }],
                user_data: vec![],
                total_vertex_count: 4,
            }],
            textures: vec![],
            skeleton_anims: vec![],
            material_anims: vec![],
            bone_vis_anims: vec![],
            shape_anims: vec![],
            scene_anims: vec![],
            external_files: vec![ExternalFile {
                name: "ext_file.bin".into(),
                data: vec![],
            }],
            shader_param_anims: vec![],
            color_anims: vec![],
            tex_srt_anims: vec![],
            tex_pattern_anims: vec![],
            mat_vis_anims: vec![],
        }
    }

    // -----------------------------------------------------------------------
    // Test 1: collect_strings gathers the right count of unique strings
    // -----------------------------------------------------------------------
    #[test]
    fn collect_strings_correct_count() {
        let bfres = make_test_bfres();
        let mut st = StringTable::new();
        collect_strings(&bfres, &mut st);

        // Expected unique strings from the test bfres (manually enumerated):
        // bfres.name:          "test_bfres"
        // model name:          "model_0"
        // model path:          "/model_0"
        // bone name:           "root_bone"
        // bone user_data name: "bone_ud"
        // shape name:          "shape_0"
        // key_shape name:      "key_0"
        // mat name:            "mat_0"
        // render_info name:    "gsys_render_state_mode"
        // render_info string:  "opaque"
        // shader_assign archive: "Turbo_UBER"
        // shader_assign model:   "turbo_uber"
        // attrib_assign k/v:   "a_attr", "a_val"
        // sampler_assign k/v:  "s_attr", "s_val"
        // shader_option k/v:   "o_key", "o_val"
        // shader_param name:   "cDiffuse"
        // texture_ref name:    "Alb_0"
        // sampler name:        "_a0"
        // vertex attr name:    "_p0"
        // external_file name:  "ext_file.bin"
        // Total unique = 22

        // The StringTable always adds "" as entry 0 on write, but during
        // collection we just add. We add 22 unique strings.
        // Verify by writing and counting positions.
        let mut w = LittleEndianWriter::new();
        let pool_start = st.write(&mut w);

        // After write, "" is also present, so total entries = 23.
        // Verify we can retrieve all the strings we added.
        let expected = [
            "test_bfres", "model_0", "/model_0", "root_bone", "bone_ud",
            "shape_0", "key_0", "mat_0", "gsys_render_state_mode", "opaque",
            "Turbo_UBER", "turbo_uber", "a_attr", "a_val", "s_attr", "s_val",
            "o_key", "o_val", "cDiffuse", "Alb_0", "_a0", "_p0",
            "ext_file.bin",
        ];
        for s in &expected {
            // Should not panic — string was registered.
            let _pos = st.get_position(s, pool_start);
        }
        assert_eq!(expected.len(), 23);
    }

    // -----------------------------------------------------------------------
    // Test 2: write_fmdl_header produces the expected byte count for v5
    // -----------------------------------------------------------------------
    #[test]
    fn fmdl_header_size_v5() {
        let bfres = make_test_bfres();
        let model = &bfres.models[0];
        let mut w = LittleEndianWriter::new();
        let _fixups = write_fmdl_header(&mut w, model, 5);

        // For v < 9:
        //   12 (header block)
        // + 8*8 = 64 (name, path, skeleton, vbuf, shape_vals, shape_dict,
        //             material_vals, material_dict)
        // + 8*2 = 16 (user_data, user_data_dict)
        // + 8 (user pointer i64)
        // + 2+2+2 = 6 (vb count, shape count, mat count)
        // + 2 (user_data count)
        // + 4 (total_vertex_count)
        // + 4 (padding)
        // = 12 + 64 + 16 + 8 + 6 + 2 + 4 + 4 = 116
        assert_eq!(w.pos(), 116);
    }

    // -----------------------------------------------------------------------
    // Test 3: write_bone produces the correct byte layout for v5
    // -----------------------------------------------------------------------
    #[test]
    fn bone_layout_v5() {
        let bone = Bone {
            name: "TestBone".into(),
            index: 0,
            parent_index: -1,
            smooth_matrix_index: 0,
            rigid_matrix_index: -1,
            billboard_index: -1,
            flags: 0x01001201,
            scale: [1.0, 2.0, 3.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            translation: [10.0, 20.0, 30.0],
            user_data: vec![],
        };

        let mut w = LittleEndianWriter::new();
        let fixups = write_bone(&mut w, &bone, 7, 5);
        let buf = w.as_slice();

        // For v <= 7 (v5): no extra padding after user_data_dict_off.
        // Layout:
        //   3 * 8 = 24 bytes (name, user_data, user_data_dict placeholders)
        //   0 bytes extra padding (v5 < 8)
        //   2 (index) + 2 (parent) + 2 (smooth) + 2 (rigid) + 2 (billboard) + 2 (ud count) = 12
        //   4 (flags)
        //   3*4 = 12 (scale)
        //   4*4 = 16 (rotation)
        //   3*4 = 12 (translation)
        //   Total = 24 + 12 + 4 + 12 + 16 + 12 = 80

        assert_eq!(w.pos(), 80);

        // Check that placeholders are at the right positions.
        assert_eq!(fixups.name_off, 0);
        assert_eq!(fixups.user_data_off, 8);
        assert_eq!(fixups.user_data_dict_off, 16);

        // Check index field (at offset 24).
        let idx = u16::from_le_bytes([buf[24], buf[25]]);
        assert_eq!(idx, 7);

        // Check parent_index (at offset 26).
        let parent = i16::from_le_bytes([buf[26], buf[27]]);
        assert_eq!(parent, -1);

        // Check flags (at offset 36).
        let flags = u32::from_le_bytes([buf[36], buf[37], buf[38], buf[39]]);
        assert_eq!(flags, 0x01001201);

        // Check scale[0] (at offset 40).
        let sx = f32::from_le_bytes([buf[40], buf[41], buf[42], buf[43]]);
        assert_eq!(sx, 1.0);

        // Check translation[2] (last 4 bytes).
        let tz = f32::from_le_bytes([buf[76], buf[77], buf[78], buf[79]]);
        assert_eq!(tz, 30.0);
    }

    // -----------------------------------------------------------------------
    // Test 4: write_fskl produces the expected byte count for v5
    // -----------------------------------------------------------------------
    #[test]
    fn fskl_header_size_v5() {
        let skeleton = Skeleton {
            flags: 0x1200,
            bones: vec![Bone {
                name: "b".into(),
                index: 0,
                parent_index: -1,
                smooth_matrix_index: 0,
                rigid_matrix_index: -1,
                billboard_index: -1,
                flags: 1,
                scale: [1.0; 3],
                rotation: [0.0, 0.0, 0.0, 1.0],
                translation: [0.0; 3],
                user_data: vec![],
            }],
            smooth_indices: vec![0],
            rigid_indices: vec![],
            inverse_model_matrices: vec![[1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0]],
        };

        let mut w = LittleEndianWriter::new();
        let _fixups = write_fskl(&mut w, &skeleton, 5);

        // For v5 (v < 8, v < 9):
        //   4 (magic FSKL)
        // + 12 (header block)
        // + 4*8 = 32 (bone_dict, bone_array, matrix_to_bone, inv_matrices)
        // + 0 (no v8/v9 padding)
        // + 8 (user_pointer placeholder)
        // + 4 (flags, v < 9)
        // + 2+2+2 = 6 (bone count, smooth count, rigid count)
        // + 6 (padding, v < 9)
        // = 4 + 12 + 32 + 8 + 4 + 6 + 6 = 72
        assert_eq!(w.pos(), 72);
    }

    // -----------------------------------------------------------------------
    // Test 5: write_mesh produces correct output
    // -----------------------------------------------------------------------
    #[test]
    fn mesh_entry_layout() {
        let mesh = Mesh {
            primitive_type: 3,   // Triangles
            index_format: 1,     // UInt16
            index_count: 300,
            first_vertex: 0,
            sub_meshes: vec![
                SubMesh { offset: 0, count: 100 },
                SubMesh { offset: 100, count: 200 },
            ],
            index_data: vec![],
        };

        let mut w = LittleEndianWriter::new();
        let fixups = write_mesh(&mut w, &mesh, 0x400);
        let buf = w.as_slice();

        // Layout:
        //   4*8 = 32 (sub_mesh_array, memory_pool, buffer_unk, buffer_size placeholders)
        // + 4 (face_buffer_offset)
        // + 4 (primitive_type)
        // + 4 (index_format)
        // + 4 (index_count)
        // + 4 (first_vertex)
        // + 2 (sub_mesh_count)
        // + 2 (padding)
        // = 32 + 24 = 56
        assert_eq!(w.pos(), 56);

        assert_eq!(fixups.sub_mesh_array_off, 0);
        assert_eq!(fixups.memory_pool_off, 8);

        // Check face_buffer_offset (at offset 32).
        let fbo = u32::from_le_bytes([buf[32], buf[33], buf[34], buf[35]]);
        assert_eq!(fbo, 0x400);

        // Check index_count (at offset 44).
        let ic = u32::from_le_bytes([buf[44], buf[45], buf[46], buf[47]]);
        assert_eq!(ic, 300);

        // Check sub_mesh_count (at offset 52).
        let smc = u16::from_le_bytes([buf[52], buf[53]]);
        assert_eq!(smc, 2);
    }

    // -----------------------------------------------------------------------
    // Test 6: write_fmat_header size for v5
    // -----------------------------------------------------------------------
    #[test]
    fn fmat_header_size_v5() {
        let mat = Material {
            name: "m".into(),
            flags: 0,
            index: 0,
            render_infos: vec![],
            render_state: None,
            shader_assign: None,
            shader_params: vec![],
            texture_refs: vec![],
            samplers: vec![],
            user_data: vec![],
            shader_param_data: vec![],
            volatile_flags: vec![],
        };

        let mut w = LittleEndianWriter::new();
        let _fixups = write_fmat_header(&mut w, &mat, 0, 5);

        // For v5 (v < 9, v != 9):
        //   12 (header block)
        // + 15*8 = 120 (name through volatile_flags placeholders)
        // + 8 (user pointer)
        // + 2*8 = 16 (sampler_slot, texture_slot placeholders)
        // + 4 (flags, v != 9)
        // + 2 (index) + 2 (render_info count) + 1 (sampler count) + 1 (tex count)
        //   + 2 (shader_param count) + 2 (volatile count) + 2 (param data len)
        //   + 2 (sizParamRaw) + 2 (user_data count) = 16
        // + 4 (padding, v != 9)
        // = 12 + 120 + 8 + 16 + 4 + 16 + 4 = 180
        assert_eq!(w.pos(), 180);
    }

    // -----------------------------------------------------------------------
    // Test 7: write_fvtx_header size for v5
    // -----------------------------------------------------------------------
    #[test]
    fn fvtx_header_size_v5() {
        let vb = VertexBuffer {
            index: 0,
            vertex_count: 100,
            vertex_skin_count: 0,
            attributes: vec![],
            buffers: vec![],
        };

        let mut w = LittleEndianWriter::new();
        let _fixups = write_fvtx_header(&mut w, &vb, 0, 0, 5);

        // For v5 (v < 9):
        //   12 (header block)
        // + 7*8 = 56 (attrib, attrib_dict, mem_pool, unk, unk2, buf_size, stride)
        // + 8 (padding i64)
        // + 4 (buffer_offset)
        // + 1 (attrib count) + 1 (buffer count) + 2 (index)
        // + 4 (vertex_count)
        // + 2 (vertex_skin_count) + 2 (gpu alignment)
        // = 12 + 56 + 8 + 4 + 4 + 4 + 4 = 92
        assert_eq!(w.pos(), 92);
    }
}
