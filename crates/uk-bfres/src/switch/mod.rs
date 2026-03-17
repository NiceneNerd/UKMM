//! Switch BFRES writer.
//!
//! Serializes the internal data model to little-endian BFRES v0.5 format.

pub mod string_table;
pub mod dict;
pub mod reloc;
pub mod model;
pub mod bntx;

use crate::binary::LittleEndianWriter;
use crate::error::Result;
use crate::model::*;
use dict::DictBuilder;
use reloc::RelocationTable;
use string_table::StringTable;

// ---------------------------------------------------------------------------
// String collection helpers
// ---------------------------------------------------------------------------

/// Collect every string referenced in the BfresFile into the string table.
fn collect_strings(bfres: &BfresFile, st: &mut StringTable) {
    st.add(&bfres.name);

    for model in &bfres.models {
        collect_model_strings(model, st);
    }

    for anim in &bfres.skeleton_anims {
        st.add(&anim.name);
        st.add(&anim.path);
    }
    for anim in &bfres.material_anims {
        st.add(&anim.name);
        st.add(&anim.path);
    }
    for anim in &bfres.bone_vis_anims {
        st.add(&anim.name);
        st.add(&anim.path);
    }
    for anim in &bfres.shape_anims {
        st.add(&anim.name);
        st.add(&anim.path);
    }
    for anim in &bfres.scene_anims {
        st.add(&anim.name);
        st.add(&anim.path);
    }
    for ef in &bfres.external_files {
        st.add(&ef.name);
    }
}

fn collect_model_strings(model: &Model, st: &mut StringTable) {
    st.add(&model.name);
    st.add(&model.path);

    // Skeleton
    for bone in &model.skeleton.bones {
        st.add(&bone.name);
        collect_user_data_strings(&bone.user_data, st);
    }

    // Shapes
    for shape in &model.shapes {
        st.add(&shape.name);
        for ks in &shape.key_shapes {
            st.add(&ks.name);
        }
    }

    // Materials
    for mat in &model.materials {
        collect_material_strings(mat, st);
    }

    // Vertex buffers
    for vb in &model.vertex_buffers {
        for attr in &vb.attributes {
            st.add(&attr.name);
        }
    }

    // User data
    collect_user_data_strings(&model.user_data, st);
}

fn collect_material_strings(mat: &Material, st: &mut StringTable) {
    st.add(&mat.name);

    for ri in &mat.render_infos {
        st.add(&ri.name);
        if let RenderInfoValue::String(vs) = &ri.value {
            for v in vs {
                st.add(v);
            }
        }
    }

    if let Some(sa) = &mat.shader_assign {
        st.add(&sa.shader_archive_name);
        st.add(&sa.shading_model_name);
        for (k, v) in &sa.attrib_assigns {
            st.add(k);
            st.add(v);
        }
        for (k, v) in &sa.sampler_assigns {
            st.add(k);
            st.add(v);
        }
        for (k, v) in &sa.shader_options {
            st.add(k);
            st.add(v);
        }
    }

    for sp in &mat.shader_params {
        st.add(&sp.name);
    }

    for tr in &mat.texture_refs {
        st.add(&tr.name);
    }

    for s in &mat.samplers {
        st.add(&s.name);
    }

    collect_user_data_strings(&mat.user_data, st);
}

fn collect_user_data_strings(user_data: &[UserData], st: &mut StringTable) {
    for ud in user_data {
        st.add(&ud.name);
        if let UserDataValue::String(vs) = &ud.value {
            for v in vs {
                st.add(v);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tracked offset placeholder types
// ---------------------------------------------------------------------------

/// A position in the output buffer where an i64 relative offset placeholder
/// was written, together with the string that should eventually be resolved
/// to its string-pool position.
struct StringFixup {
    placeholder_pos: usize,
    value: String,
}

// ---------------------------------------------------------------------------
// Sub-file writing helpers (inline, not in model.rs)
// ---------------------------------------------------------------------------

/// Positions of offset placeholders within an FMDL header.
struct FmdlOffsets {
    name_off: usize,
    path_off: usize,
    fskl_off: usize,
    fvtx_array_off: usize,
    fshp_off: usize,
    fshp_dict_off: usize,
    fmat_off: usize,
    fmat_dict_off: usize,
    #[allow(dead_code)]
    user_data_off: usize,
    #[allow(dead_code)]
    user_data_dict_off: usize,
}

/// Write the FMDL header block; returns the offsets structure for later fixup.
fn write_fmdl_header(w: &mut LittleEndianWriter, model: &Model) -> FmdlOffsets {
    // Magic
    w.write_magic(b"FMDL");

    // Two block offset u32s + padding (12 bytes). The first typically records
    // the header size. We write placeholders -- not critical for loading since
    // the Switch runtime uses the relocation table for navigation.
    w.write_u32(0);
    w.write_u32(0);
    w.write_u32(0);

    // i64 offset fields
    let name_off = w.write_offset_placeholder_64();
    let path_off = w.write_offset_placeholder_64();
    let fskl_off = w.write_offset_placeholder_64();
    let fvtx_array_off = w.write_offset_placeholder_64();
    let fshp_off = w.write_offset_placeholder_64();
    let fshp_dict_off = w.write_offset_placeholder_64();
    let fmat_off = w.write_offset_placeholder_64();
    let fmat_dict_off = w.write_offset_placeholder_64();
    let user_data_off = w.write_offset_placeholder_64();
    w.write_u64(0); // padding i64
    let user_data_dict_off = w.write_offset_placeholder_64();

    // Counts
    w.write_u16(model.vertex_buffers.len() as u16); // FVTX count
    w.write_u16(model.shapes.len() as u16);
    w.write_u16(model.materials.len() as u16);
    w.write_u16(model.vertex_buffers.len() as u16); // param count

    w.write_u32(model.total_vertex_count);
    w.write_u32(0); // padding

    FmdlOffsets {
        name_off,
        path_off,
        fskl_off,
        fvtx_array_off,
        fshp_off,
        fshp_dict_off,
        fmat_off,
        fmat_dict_off,
        user_data_off,
        user_data_dict_off,
    }
}

/// Positions of offset placeholders within an FVTX block that need deferred fixup.
struct FvtxDictOffsets {
    attr_dict_off: usize,
    /// Position of the u32 BufferOffset field (offset from buffer data section start
    /// to this FVTX's first vertex buffer data).
    buf_offset_pos: usize,
    /// Position of the i64 memory pool pointer placeholder.
    mem_pool_off: usize,
    /// Start position of this FVTX block.
    start: usize,
}

/// Write an FVTX (vertex buffer) block. Returns dict offsets for deferred fixup.
///
/// v5 layout (96 bytes):
///   +0x00  magic "FVTX" (4) + block offsets (12)
///   +0x10  attr_array_offset (i64)
///   +0x18  attr_dict_offset (i64)
///   +0x20  memory_pool_ptr (i64)
///   +0x28  runtime_buf_ptrs (i64) — runtime-filled, serialize as pointer to zero array
///   +0x30  buf_info_array (i64) — runtime-filled zero array (sizes)
///   +0x38  buf_stride_array (i64) — runtime-filled zero array (strides)
///   +0x40  buf_data_off_array (i64) — runtime-filled zero array (data offsets)
///   +0x48  padding (i64) = 0
///   +0x50  buffer_offset (u32)
///   +0x54  attr_count (u8), buf_count (u8), index (u16)
///   +0x58  vertex_count (u32)
///   +0x5C  vertex_skin_count (u16), alignment (u16)
fn write_fvtx(
    w: &mut LittleEndianWriter,
    vb: &VertexBuffer,
    string_fixups: &mut Vec<StringFixup>,
    reloc: &mut RelocationTable,
) -> FvtxDictOffsets {
    let start = w.pos();

    // Magic "FVTX"
    w.write_magic(b"FVTX");
    w.write_u32(0); // block offset
    w.write_u32(0); // block offset 2
    w.write_u32(0); // padding

    // +0x10: Attribute array offset (i64)
    let attr_array_off = w.write_offset_placeholder_64();
    reloc.add(0, attr_array_off as u32, 1, 1, 0);

    // +0x18: Attribute dict offset (i64)
    let attr_dict_off = w.write_offset_placeholder_64();

    // +0x20: Memory pool pointer (i64) — points to memory pool section
    let mem_pool_off = w.write_offset_placeholder_64();

    // +0x28: Runtime buffer pointers (i64) — points to zero array, runtime-filled
    let runtime_buf_ptrs_off = w.write_offset_placeholder_64();
    reloc.add(0, runtime_buf_ptrs_off as u32, 1, 1, 0);

    // +0x30: Buffer info array (i64) — runtime-filled zero array
    let buf_info_off = w.write_offset_placeholder_64();
    reloc.add(0, buf_info_off as u32, 1, 1, 0);

    // +0x38: Buffer stride array (i64) — runtime-filled zero array
    let buf_stride_off = w.write_offset_placeholder_64();
    reloc.add(0, buf_stride_off as u32, 1, 1, 0);

    // +0x40: Buffer data offset array (i64) — runtime-filled zero array
    let buf_data_off_arr = w.write_offset_placeholder_64();
    reloc.add(0, buf_data_off_arr as u32, 1, 1, 0);

    // +0x48: Padding (i64)
    w.write_u64(0);

    // +0x50: BufferOffset (u32) — offset from buffer data section start
    let buf_offset_pos = w.pos();
    w.write_u32(0); // patched later by orchestrator

    // +0x54: Counts
    w.write_u8(vb.attributes.len() as u8);
    w.write_u8(vb.buffers.len() as u8);
    w.write_u16(vb.index);
    w.write_u32(vb.vertex_count);
    w.write_u16(vb.vertex_skin_count as u16);
    w.write_u16(8); // GPU buffer alignment

    // Write attribute entries
    let attr_start = w.pos();
    w.fixup_offset_64(attr_array_off, attr_start);
    for attr in &vb.attributes {
        let name_ph = w.write_offset_placeholder_64();
        string_fixups.push(StringFixup {
            placeholder_pos: name_ph,
            value: attr.name.clone(),
        });
        reloc.add(0, name_ph as u32, 1, 1, 0);
        w.write_u32(attr.format);
        w.write_u16(attr.offset);
        w.write_u8(attr.buffer_index);
        w.write_u8(0); // padding
    }

    // Write runtime buffer pointer array (zeros, one i64 per buffer — runtime-filled)
    w.align(8);
    let runtime_ptrs_start = w.pos();
    w.fixup_offset_64(runtime_buf_ptrs_off, runtime_ptrs_start);
    w.write_zeros(vb.buffers.len() * 8);

    // Write buffer info array (zeros, one entry of 8 bytes per buffer — runtime-filled)
    w.align(8);
    let buf_info_start = w.pos();
    w.fixup_offset_64(buf_info_off, buf_info_start);
    w.write_zeros(vb.buffers.len() * 8);

    // Write buffer stride array (zeros, one entry of 4 bytes per buffer — runtime-filled)
    w.align(8);
    let buf_stride_start = w.pos();
    w.fixup_offset_64(buf_stride_off, buf_stride_start);
    w.write_zeros(vb.buffers.len() * 4);
    w.align(8);

    // Write buffer data offset array (zeros, one i64 per buffer — runtime-filled)
    let buf_data_off_start = w.pos();
    w.fixup_offset_64(buf_data_off_arr, buf_data_off_start);
    w.write_zeros(vb.buffers.len() * 8);

    FvtxDictOffsets {
        attr_dict_off,
        buf_offset_pos,
        mem_pool_off,
        start,
    }
}

/// Positions of offset placeholders within an FSKL block that need deferred fixup.
struct FsklDictOffsets {
    bone_dict_off: usize,
    start: usize,
}

/// Write skeleton (FSKL) block. Returns (start_pos, dict offsets).
fn write_fskl(
    w: &mut LittleEndianWriter,
    skeleton: &Skeleton,
    string_fixups: &mut Vec<StringFixup>,
    reloc: &mut RelocationTable,
) -> FsklDictOffsets {
    let start = w.pos();

    w.write_magic(b"FSKL");
    w.write_u32(0); // block offset
    w.write_u32(0); // block offset 2
    w.write_u32(0); // padding

    // Bone dict offset (i64)
    let bone_dict_off = w.write_offset_placeholder_64();
    reloc.add(0, bone_dict_off as u32, 1, 1, 0);

    // Bone array offset (i64)
    let bone_array_off = w.write_offset_placeholder_64();
    reloc.add(0, bone_array_off as u32, 1, 1, 0);

    // Smooth index array offset (i64)
    let smooth_idx_off = w.write_offset_placeholder_64();
    if !skeleton.smooth_indices.is_empty() {
        reloc.add(0, smooth_idx_off as u32, 1, 1, 0);
    }

    // Rigid index array offset (i64)
    let rigid_idx_off = w.write_offset_placeholder_64();
    if !skeleton.rigid_indices.is_empty() {
        reloc.add(0, rigid_idx_off as u32, 1, 1, 0);
    }

    // Inverse model matrix array offset (i64)
    let inv_matrix_off = w.write_offset_placeholder_64();
    if !skeleton.inverse_model_matrices.is_empty() {
        reloc.add(0, inv_matrix_off as u32, 1, 1, 0);
    }

    // Padding i64
    w.write_u64(0);

    // Flags + counts
    w.write_u32(skeleton.flags);
    w.write_u16(skeleton.bones.len() as u16);
    w.write_u16(skeleton.smooth_indices.len() as u16);
    w.write_u16(skeleton.rigid_indices.len() as u16);
    w.write_u16(0); // padding

    // Write bone array
    let bone_arr_start = w.pos();
    w.fixup_offset_64(bone_array_off, bone_arr_start);
    for bone in &skeleton.bones {
        write_bone(w, bone, string_fixups, reloc);
    }

    // Write smooth index array
    if !skeleton.smooth_indices.is_empty() {
        let smooth_start = w.pos();
        w.fixup_offset_64(smooth_idx_off, smooth_start);
        for &idx in &skeleton.smooth_indices {
            w.write_u16(idx);
        }
        w.align(8);
    }

    // Write rigid index array
    if !skeleton.rigid_indices.is_empty() {
        let rigid_start = w.pos();
        w.fixup_offset_64(rigid_idx_off, rigid_start);
        for &idx in &skeleton.rigid_indices {
            w.write_u16(idx);
        }
        w.align(8);
    }

    // Write inverse model matrices
    if !skeleton.inverse_model_matrices.is_empty() {
        let mtx_start = w.pos();
        w.fixup_offset_64(inv_matrix_off, mtx_start);
        for mtx in &skeleton.inverse_model_matrices {
            for &v in mtx {
                w.write_f32(v);
            }
        }
    }

    FsklDictOffsets {
        bone_dict_off,
        start,
    }
}

/// Write a single bone entry.
fn write_bone(
    w: &mut LittleEndianWriter,
    bone: &Bone,
    string_fixups: &mut Vec<StringFixup>,
    reloc: &mut RelocationTable,
) {
    // name offset (i64)
    let name_ph = w.write_offset_placeholder_64();
    string_fixups.push(StringFixup {
        placeholder_pos: name_ph,
        value: bone.name.clone(),
    });
    reloc.add(0, name_ph as u32, 1, 1, 0);

    // User data offset (i64) - null
    w.write_u64(0);
    // User data dict offset (i64) - null
    w.write_u64(0);

    // Bone fields
    w.write_u16(bone.index);
    w.write_i16(bone.parent_index);
    w.write_i16(bone.smooth_matrix_index);
    w.write_i16(bone.rigid_matrix_index);
    w.write_i16(bone.billboard_index);
    w.write_u16(bone.user_data.len() as u16);
    w.write_u32(bone.flags);

    // Scale
    for &v in &bone.scale {
        w.write_f32(v);
    }

    // Rotation (quaternion)
    for &v in &bone.rotation {
        w.write_f32(v);
    }

    // Translation
    for &v in &bone.translation {
        w.write_f32(v);
    }
}

/// Offsets returned from write_fshp for deferred fixup.
struct FshpFixups {
    start: usize,
    /// Placeholder for the vertex buffer pointer (points to the FVTX block).
    vb_ptr_off: usize,
    /// Per-mesh index buffer data pointer placeholders.
    mesh_idx_buf_offs: Vec<usize>,
    /// Per-mesh memory pool offset u32 positions.
    mesh_mem_pool_offset_positions: Vec<usize>,
}

/// Write FSHP (shape) block.
///
/// v5 layout:
///   +0x10  name (i64)
///   +0x18  vertex_buffer_ptr (i64) — points to this shape's FVTX block
///   +0x20  mesh_array (i64)
///   +0x28  skin_bone_idx_array (i64)
///   +0x30  key_shape_array (i64)
///   +0x38  key_shape_dict (i64)
///   +0x40  bbox_array (i64)
///   +0x48  brad_array (i64)
///   +0x50  user_pointer (i64) = 0
///   +0x58  flags (u32), index (u16), mat_idx (u16)
///   +0x60  bone_idx (u16), vb_idx (u16), skin_count (u16), vtx_skin (u8), mesh_count (u8)
///   +0x68  key_count (u8), target_attrib (u8), padding (u16)
fn write_fshp(
    w: &mut LittleEndianWriter,
    shape: &Shape,
    string_fixups: &mut Vec<StringFixup>,
    reloc: &mut RelocationTable,
) -> FshpFixups {
    let start = w.pos();

    w.write_magic(b"FSHP");
    w.write_u32(0); // block offset
    w.write_u32(0); // block offset 2
    w.write_u32(0); // padding

    // +0x10: Name offset (i64)
    let name_ph = w.write_offset_placeholder_64();
    string_fixups.push(StringFixup {
        placeholder_pos: name_ph,
        value: shape.name.clone(),
    });
    reloc.add(0, name_ph as u32, 1, 1, 0);

    // +0x18: Vertex buffer pointer (i64) — points to FVTX block, fixed up by orchestrator
    let vb_ptr_off = w.write_offset_placeholder_64();
    reloc.add(0, vb_ptr_off as u32, 1, 1, 0);

    // +0x20: Mesh array offset (i64)
    let mesh_off = w.write_offset_placeholder_64();
    reloc.add(0, mesh_off as u32, 1, 1, 0);

    // +0x28: Skin bone index array offset (i64)
    let skin_off = w.write_offset_placeholder_64();
    if !shape.skin_bone_indices.is_empty() {
        reloc.add(0, skin_off as u32, 1, 1, 0);
    }

    // +0x30: Key shape array offset (i64)
    let key_shape_off = w.write_offset_placeholder_64();
    if !shape.key_shapes.is_empty() {
        reloc.add(0, key_shape_off as u32, 1, 1, 0);
    }

    // +0x38: Key shape dict offset (i64) - null
    w.write_u64(0);

    // +0x40: Bounding box array offset (i64)
    let bbox_off = w.write_offset_placeholder_64();
    if !shape.bounding_boxes.is_empty() {
        reloc.add(0, bbox_off as u32, 1, 1, 0);
    }

    // +0x48: Bounding radius array offset (i64)
    let brad_off = w.write_offset_placeholder_64();
    if !shape.bounding_radius.is_empty() {
        reloc.add(0, brad_off as u32, 1, 1, 0);
    }

    // +0x50: User pointer (i64) - null (runtime)
    w.write_u64(0);

    // +0x58: Flags, index, counts
    w.write_u32(shape.flags);
    w.write_u16(shape.index);
    w.write_u16(shape.material_index);
    w.write_u16(shape.bone_index);
    w.write_u16(shape.vertex_buffer_index);
    w.write_u16(shape.skin_bone_indices.len() as u16);
    w.write_u8(shape.vertex_skin_count);
    w.write_u8(shape.meshes.len() as u8);
    w.write_u8(shape.key_shapes.len() as u8);
    w.write_u8(0); // target_attrib_count
    w.write_u16(0); // padding

    w.align(8);

    // Write mesh array
    let mesh_start = w.pos();
    w.fixup_offset_64(mesh_off, mesh_start);
    let mut per_mesh_idx_buf_offs: Vec<usize> = Vec::new();
    let mut per_mesh_mem_pool_positions: Vec<usize> = Vec::new();
    for mesh in &shape.meshes {
        let mf = write_mesh(w, mesh);
        per_mesh_idx_buf_offs.push(mf.idx_buf_data_off);
        per_mesh_mem_pool_positions.push(mf.mem_pool_offset_pos);
    }

    // Write skin bone indices
    if !shape.skin_bone_indices.is_empty() {
        let skin_start = w.pos();
        w.fixup_offset_64(skin_off, skin_start);
        for &idx in &shape.skin_bone_indices {
            w.write_u16(idx);
        }
        w.align(8);
    }

    // Write key shapes
    if !shape.key_shapes.is_empty() {
        let ks_start = w.pos();
        w.fixup_offset_64(key_shape_off, ks_start);
        for ks in &shape.key_shapes {
            let ks_name_ph = w.write_offset_placeholder_64();
            string_fixups.push(StringFixup {
                placeholder_pos: ks_name_ph,
                value: ks.name.clone(),
            });
            w.write_u8(ks.index);
            w.write_zeros(7); // padding
        }
    }

    // Write bounding boxes
    if !shape.bounding_boxes.is_empty() {
        let bb_start = w.pos();
        w.fixup_offset_64(bbox_off, bb_start);
        for bb in &shape.bounding_boxes {
            for &v in &bb.center {
                w.write_f32(v);
            }
            for &v in &bb.extent {
                w.write_f32(v);
            }
        }
    }

    // Write bounding radii
    if !shape.bounding_radius.is_empty() {
        let br_start = w.pos();
        w.fixup_offset_64(brad_off, br_start);
        for &r in &shape.bounding_radius {
            w.write_f32(r);
        }
        w.align(8);
    }

    FshpFixups {
        start,
        vb_ptr_off,
        mesh_idx_buf_offs: per_mesh_idx_buf_offs,
        mesh_mem_pool_offset_positions: per_mesh_mem_pool_positions,
    }
}

/// Position of a mesh's buffer offset placeholder for deferred fixup.
struct MeshFixups {
    /// Position of the i64 index buffer data pointer placeholder.
    idx_buf_data_off: usize,
    /// Position of the u32 memory pool offset placeholder.
    mem_pool_offset_pos: usize,
}

/// Write a mesh entry.
///
/// v5 layout (0x38 = 56 bytes per entry):
///   +0x00  sub_mesh_array (i64)
///   +0x08  memory_pool_ptr (i64) — runtime, zeroed
///   +0x10  index_buffer_data (i64) — points to index data in buffer section
///   +0x18  index_buffer_info (i64) — runtime, zeroed
///   +0x20  memory_pool_offset (u32) — offset into buffer data section
///   +0x24  primitive_type (u32)
///   +0x28  index_format (u32)
///   +0x2C  index_count (u32)
///   +0x30  first_vertex (u32)
///   +0x34  sub_mesh_count (u16)
///   +0x36  padding (u16)
fn write_mesh(w: &mut LittleEndianWriter, mesh: &Mesh) -> MeshFixups {
    // +0x00: Sub-mesh array offset (i64)
    let sub_mesh_off = w.write_offset_placeholder_64();

    // +0x08: Memory pool pointer (i64) - runtime, zeroed
    w.write_u64(0);

    // +0x10: Index buffer data offset (i64) — fixed up later by orchestrator
    let idx_buf_data_off = w.write_offset_placeholder_64();

    // +0x18: Index buffer info (i64) - runtime, zeroed
    w.write_u64(0);

    // +0x20: Memory pool offset (u32) — offset into buffer data section
    let mem_pool_offset_pos = w.pos();
    w.write_u32(0); // patched later

    // +0x24: Fields
    w.write_u32(mesh.primitive_type);
    w.write_u32(mesh.index_format);
    w.write_u32(mesh.index_count);
    w.write_u32(mesh.first_vertex);

    // +0x34: Sub-mesh count
    w.write_u16(mesh.sub_meshes.len() as u16);
    w.write_u16(0); // padding

    // Write sub-mesh array inline
    if !mesh.sub_meshes.is_empty() {
        w.align(8);
        let sm_start = w.pos();
        w.fixup_offset_64(sub_mesh_off, sm_start);
        for sm in &mesh.sub_meshes {
            w.write_u32(sm.offset);
            w.write_u32(sm.count);
        }
    }

    MeshFixups {
        idx_buf_data_off,
        mem_pool_offset_pos,
    }
}

/// Positions of offset placeholders within an FMAT block that need deferred fixup.
struct FmatDictOffsets {
    ri_dict_off: usize,
    samp_dict_off: usize,
    shader_assign_dicts: Option<ShaderAssignDictOffsets>,
    start: usize,
}

/// Write FMAT (material) block. Returns (start_pos, dict offsets).
fn write_fmat(
    w: &mut LittleEndianWriter,
    mat: &Material,
    string_fixups: &mut Vec<StringFixup>,
    reloc: &mut RelocationTable,
) -> FmatDictOffsets {
    let start = w.pos();

    w.write_magic(b"FMAT");
    w.write_u32(0); // block offset
    w.write_u32(0); // block offset 2
    w.write_u32(0); // padding

    // Name offset (i64)
    let name_ph = w.write_offset_placeholder_64();
    string_fixups.push(StringFixup {
        placeholder_pos: name_ph,
        value: mat.name.clone(),
    });
    reloc.add(0, name_ph as u32, 1, 1, 0);

    // Render info array offset (i64)
    let ri_off = w.write_offset_placeholder_64();
    if !mat.render_infos.is_empty() {
        reloc.add(0, ri_off as u32, 1, 1, 0);
    }

    // Render info dict offset (i64)
    let ri_dict_off = w.write_offset_placeholder_64();

    // Shader assign offset (i64)
    let sa_off = w.write_offset_placeholder_64();
    if mat.shader_assign.is_some() {
        reloc.add(0, sa_off as u32, 1, 1, 0);
    }

    // Texture ref array offset (i64)
    let tex_off = w.write_offset_placeholder_64();
    if !mat.texture_refs.is_empty() {
        reloc.add(0, tex_off as u32, 1, 1, 0);
    }

    // Texture name array offset (i64)
    let tex_name_off = w.write_offset_placeholder_64();
    if !mat.texture_refs.is_empty() {
        reloc.add(0, tex_name_off as u32, 1, 1, 0);
    }

    // Sampler array offset (i64)
    let samp_off = w.write_offset_placeholder_64();
    if !mat.samplers.is_empty() {
        reloc.add(0, samp_off as u32, 1, 1, 0);
    }

    // Sampler dict offset (i64)
    let samp_dict_off = w.write_offset_placeholder_64();

    // Shader param array offset (i64)
    let sp_off = w.write_offset_placeholder_64();
    if !mat.shader_params.is_empty() {
        reloc.add(0, sp_off as u32, 1, 1, 0);
    }

    // Shader param data offset (i64)
    let sp_data_off = w.write_offset_placeholder_64();
    if !mat.shader_param_data.is_empty() {
        reloc.add(0, sp_data_off as u32, 1, 1, 0);
    }

    // User data array offset (i64) - null
    w.write_u64(0);
    // User data dict offset (i64) - null
    w.write_u64(0);

    // Volatile flags offset (i64)
    let vf_off = w.write_offset_placeholder_64();
    if !mat.volatile_flags.is_empty() {
        reloc.add(0, vf_off as u32, 1, 1, 0);
    }

    // Runtime user pointer (i64)
    w.write_u64(0);
    // Padding i64
    w.write_u64(0);

    // Counts
    w.write_u32(mat.flags);
    w.write_u16(mat.index);
    w.write_u16(mat.render_infos.len() as u16);
    w.write_u8(mat.texture_refs.len() as u8);
    w.write_u8(mat.samplers.len() as u8);

    let sp_data_size = mat.shader_param_data.len() as u16;
    w.write_u16(sp_data_size);
    w.write_u16(0); // raw param size
    w.write_u16(mat.shader_params.len() as u16);
    w.write_u16(mat.volatile_flags.len() as u16);
    w.write_u16(mat.user_data.len() as u16);

    w.align(8);

    // -- Write render info array --
    if !mat.render_infos.is_empty() {
        let ri_start = w.pos();
        w.fixup_offset_64(ri_off, ri_start);
        for ri in &mat.render_infos {
            write_render_info(w, ri, string_fixups, reloc);
        }
    }

    // -- Write shader params array --
    if !mat.shader_params.is_empty() {
        w.align(8);
        let sp_start = w.pos();
        w.fixup_offset_64(sp_off, sp_start);
        for sp in &mat.shader_params {
            write_shader_param(w, sp, string_fixups, reloc);
        }
    }

    // -- Write shader param data --
    if !mat.shader_param_data.is_empty() {
        w.align(8);
        let spd_start = w.pos();
        w.fixup_offset_64(sp_data_off, spd_start);
        w.write_bytes(&mat.shader_param_data);
        w.align(8);
    }

    // -- Write texture ref entries --
    if !mat.texture_refs.is_empty() {
        let tex_start = w.pos();
        w.fixup_offset_64(tex_off, tex_start);
        // Each texture ref: runtime texture ptr (i64) + runtime view (i64)
        for _tr in &mat.texture_refs {
            w.write_u64(0); // runtime texture pointer
            w.write_u64(0); // runtime texture view
        }

        // Texture name offset array
        let tex_name_start = w.pos();
        w.fixup_offset_64(tex_name_off, tex_name_start);
        for tr in &mat.texture_refs {
            let tn_ph = w.write_offset_placeholder_64();
            string_fixups.push(StringFixup {
                placeholder_pos: tn_ph,
                value: tr.name.clone(),
            });
            reloc.add(0, tn_ph as u32, 1, 1, 0);
        }
    }

    // -- Write sampler array --
    if !mat.samplers.is_empty() {
        let samp_start = w.pos();
        w.fixup_offset_64(samp_off, samp_start);
        for s in &mat.samplers {
            // GX2/NVN sampler data (3 u32s) + handle padding to 32 bytes
            for &v in &s.gx2_sampler_data {
                w.write_u32(v);
            }
            w.write_zeros(20); // pad 3*4=12 written, fill to 32
        }
    }

    // -- Write shader assign --
    let shader_assign_dicts = if let Some(sa) = &mat.shader_assign {
        let sa_start = w.pos();
        w.fixup_offset_64(sa_off, sa_start);
        Some(write_shader_assign(w, sa, string_fixups, reloc))
    } else {
        None
    };

    // -- Write volatile flags --
    if !mat.volatile_flags.is_empty() {
        w.align(8);
        let vf_start = w.pos();
        w.fixup_offset_64(vf_off, vf_start);
        w.write_bytes(&mat.volatile_flags);
        w.align(8);
    }

    FmatDictOffsets {
        ri_dict_off,
        samp_dict_off,
        shader_assign_dicts,
        start,
    }
}

/// Write a render info entry.
fn write_render_info(
    w: &mut LittleEndianWriter,
    ri: &RenderInfo,
    string_fixups: &mut Vec<StringFixup>,
    reloc: &mut RelocationTable,
) {
    // Name offset (i64)
    let name_ph = w.write_offset_placeholder_64();
    string_fixups.push(StringFixup {
        placeholder_pos: name_ph,
        value: ri.name.clone(),
    });
    reloc.add(0, name_ph as u32, 1, 1, 0);

    // Data offset (i64)
    let data_off = w.write_offset_placeholder_64();
    reloc.add(0, data_off as u32, 1, 1, 0);

    match &ri.value {
        RenderInfoValue::Int32(vals) => {
            w.write_u16(vals.len() as u16);
            w.write_u8(0); // type = int32
            w.write_zeros(5);

            let d_start = w.pos();
            w.fixup_offset_64(data_off, d_start);
            for &v in vals {
                w.write_i32(v);
            }
            w.align(8);
        }
        RenderInfoValue::Float(vals) => {
            w.write_u16(vals.len() as u16);
            w.write_u8(1); // type = float
            w.write_zeros(5);

            let d_start = w.pos();
            w.fixup_offset_64(data_off, d_start);
            for &v in vals {
                w.write_f32(v);
            }
            w.align(8);
        }
        RenderInfoValue::String(vals) => {
            w.write_u16(vals.len() as u16);
            w.write_u8(2); // type = string
            w.write_zeros(5);

            let d_start = w.pos();
            w.fixup_offset_64(data_off, d_start);
            for v in vals {
                let sv_ph = w.write_offset_placeholder_64();
                string_fixups.push(StringFixup {
                    placeholder_pos: sv_ph,
                    value: v.clone(),
                });
                reloc.add(0, sv_ph as u32, 1, 1, 0);
            }
        }
    }
}

/// Write a shader param entry.
fn write_shader_param(
    w: &mut LittleEndianWriter,
    sp: &ShaderParam,
    string_fixups: &mut Vec<StringFixup>,
    reloc: &mut RelocationTable,
) {
    // Callback pointer (i64)
    w.write_u64(sp.callback_pointer as u64);

    // Name offset (i64)
    let name_ph = w.write_offset_placeholder_64();
    string_fixups.push(StringFixup {
        placeholder_pos: name_ph,
        value: sp.name.clone(),
    });
    reloc.add(0, name_ph as u32, 1, 1, 0);

    // Type
    w.write_u8(sp.param_type);
    // Size placeholder
    w.write_u8(0);
    // Data offset (u16)
    w.write_u16(sp.data_offset);
    // Depend index + count
    w.write_i16(sp.depend_index);
    w.write_i16(sp.depend_count);
}

/// Positions of offset placeholders within a ShaderAssign that need deferred fixup.
struct ShaderAssignDictOffsets {
    aa_dict_off: usize,
    sam_dict_off: usize,
    opt_dict_off: usize,
}

/// Write a shader assign block. Returns dict offsets for deferred fixup.
fn write_shader_assign(
    w: &mut LittleEndianWriter,
    sa: &ShaderAssign,
    string_fixups: &mut Vec<StringFixup>,
    reloc: &mut RelocationTable,
) -> ShaderAssignDictOffsets {
    // Shader archive name (i64)
    let arch_ph = w.write_offset_placeholder_64();
    string_fixups.push(StringFixup {
        placeholder_pos: arch_ph,
        value: sa.shader_archive_name.clone(),
    });
    reloc.add(0, arch_ph as u32, 1, 1, 0);

    // Shading model name (i64)
    let model_ph = w.write_offset_placeholder_64();
    string_fixups.push(StringFixup {
        placeholder_pos: model_ph,
        value: sa.shading_model_name.clone(),
    });
    reloc.add(0, model_ph as u32, 1, 1, 0);

    // Attrib assign array + dict (i64 each)
    let aa_off = w.write_offset_placeholder_64();
    if !sa.attrib_assigns.is_empty() {
        reloc.add(0, aa_off as u32, 1, 1, 0);
    }
    let aa_dict_off = w.write_offset_placeholder_64();

    // Sampler assign array + dict (i64 each)
    let sam_off = w.write_offset_placeholder_64();
    if !sa.sampler_assigns.is_empty() {
        reloc.add(0, sam_off as u32, 1, 1, 0);
    }
    let sam_dict_off = w.write_offset_placeholder_64();

    // Shader option array + dict (i64 each)
    let opt_off = w.write_offset_placeholder_64();
    if !sa.shader_options.is_empty() {
        reloc.add(0, opt_off as u32, 1, 1, 0);
    }
    let opt_dict_off = w.write_offset_placeholder_64();

    // Revision
    w.write_u32(sa.revision);

    // Counts
    w.write_u8(sa.attrib_assigns.len() as u8);
    w.write_u8(sa.sampler_assigns.len() as u8);
    w.write_u16(sa.shader_options.len() as u16);

    w.align(8);

    // Write attrib assign key-value pairs
    if !sa.attrib_assigns.is_empty() {
        let aa_start = w.pos();
        w.fixup_offset_64(aa_off, aa_start);
        for (k, v) in &sa.attrib_assigns {
            let kph = w.write_offset_placeholder_64();
            string_fixups.push(StringFixup {
                placeholder_pos: kph,
                value: k.clone(),
            });
            reloc.add(0, kph as u32, 1, 1, 0);
            let vph = w.write_offset_placeholder_64();
            string_fixups.push(StringFixup {
                placeholder_pos: vph,
                value: v.clone(),
            });
            reloc.add(0, vph as u32, 1, 1, 0);
        }
    }

    // Write sampler assign pairs
    if !sa.sampler_assigns.is_empty() {
        let sam_start = w.pos();
        w.fixup_offset_64(sam_off, sam_start);
        for (k, v) in &sa.sampler_assigns {
            let kph = w.write_offset_placeholder_64();
            string_fixups.push(StringFixup {
                placeholder_pos: kph,
                value: k.clone(),
            });
            reloc.add(0, kph as u32, 1, 1, 0);
            let vph = w.write_offset_placeholder_64();
            string_fixups.push(StringFixup {
                placeholder_pos: vph,
                value: v.clone(),
            });
            reloc.add(0, vph as u32, 1, 1, 0);
        }
    }

    // Write shader option pairs
    if !sa.shader_options.is_empty() {
        let opt_start = w.pos();
        w.fixup_offset_64(opt_off, opt_start);
        for (k, v) in &sa.shader_options {
            let kph = w.write_offset_placeholder_64();
            string_fixups.push(StringFixup {
                placeholder_pos: kph,
                value: k.clone(),
            });
            reloc.add(0, kph as u32, 1, 1, 0);
            let vph = w.write_offset_placeholder_64();
            string_fixups.push(StringFixup {
                placeholder_pos: vph,
                value: v.clone(),
            });
            reloc.add(0, vph as u32, 1, 1, 0);
        }
    }

    ShaderAssignDictOffsets {
        aa_dict_off,
        sam_dict_off,
        opt_dict_off,
    }
}

// ---------------------------------------------------------------------------
// Deferred dict fixup tracking for the orchestrator
// ---------------------------------------------------------------------------

/// All deferred dict offset placeholders for a single model's sub-files.
struct ModelDictFixups {
    /// FMDL shape dict placeholder
    fshp_dict_off: usize,
    /// FMDL material dict placeholder
    fmat_dict_off: usize,
    /// FSKL bone dict placeholder
    fskl_bone_dict_off: usize,
    /// Per-FVTX attribute dict placeholders
    fvtx_attr_dict_offs: Vec<usize>,
    /// Per-FVTX buffer offset u32 positions (patched with buffer data offsets)
    fvtx_buf_offset_positions: Vec<usize>,
    /// Per-FVTX memory pool i64 placeholder positions
    fvtx_mem_pool_offs: Vec<usize>,
    /// Per-FVTX start positions (for FSHP vertex_buffer_ptr fixup)
    fvtx_starts: Vec<usize>,
    /// Per-FSHP vertex buffer pointer placeholder positions
    fshp_vb_ptr_offs: Vec<usize>,
    /// Per-FSHP shape vertex_buffer_index (to look up the right FVTX)
    fshp_vb_indices: Vec<u16>,
    /// Per-mesh index buffer data pointer placeholder positions, grouped by shape
    mesh_idx_buf_offs: Vec<Vec<usize>>,
    /// Per-mesh memory pool offset u32 positions, grouped by shape
    mesh_mem_pool_offset_positions: Vec<Vec<usize>>,
    /// Per-FMAT render info dict + sampler dict placeholders
    fmat_dict_offs: Vec<(usize, usize)>, // (ri_dict_off, samp_dict_off)
    /// Per-FMAT shader assign dict placeholders (if shader assign exists)
    shader_assign_dict_offs: Vec<Option<ShaderAssignDictOffsets>>,
}

// ---------------------------------------------------------------------------
// Main orchestrator
// ---------------------------------------------------------------------------

/// Write a BfresFile to Switch BFRES format.
pub fn write(bfres: &BfresFile) -> Result<Vec<u8>> {
    let mut w = LittleEndianWriter::with_capacity(64 * 1024);
    let mut string_fixups: Vec<StringFixup> = Vec::new();
    let mut reloc = RelocationTable::new();

    // -----------------------------------------------------------------------
    // Phase 0: Collect all strings
    // -----------------------------------------------------------------------
    let mut st = StringTable::new();
    collect_strings(bfres, &mut st);

    // -----------------------------------------------------------------------
    // Phase 1: Write FRES header (with placeholders)
    // -----------------------------------------------------------------------
    let v = bfres.version.1; // VersionMajor2

    let version_u32: u32 = (bfres.version.0 as u32) << 24
        | (bfres.version.1 as u32) << 16
        | (bfres.version.2 as u32) << 8
        | (bfres.version.3 as u32);

    // Magic "FRES" + padding spaces
    w.write_magic(b"FRES");
    w.write_bytes(b"    "); // 0x20202020

    // Version (u32)
    w.write_u32(version_u32);

    // BOM (little-endian marker): bytes FF FE in file = 0xFEFF as LE u16
    w.write_u16(0xFEFF);

    // Alignment
    w.write_u8(bfres.alignment as u8);

    // Target address size
    w.write_u8(0);

    // Name offset (u32, absolute from file start) - placeholder at 0x10
    let name_offset_pos = w.pos();
    w.write_u32(0);

    // Flag (u16) + Block offset (u16) at 0x14
    w.write_u16(0);
    let block_offset_pos = w.pos();
    w.write_u16(0); // patched later

    // Relocation table offset (u32) at 0x18
    let reloc_offset_pos = w.pos();
    w.write_u32(0);

    // File size (u32) at 0x1C
    let file_size_pos = w.pos();
    w.write_u32(0);

    // -- i64 offset section --

    // File name string offset (i64) at 0x20
    let fres_name_off = w.write_offset_placeholder_64();
    string_fixups.push(StringFixup {
        placeholder_pos: fres_name_off,
        value: bfres.name.clone(),
    });
    reloc.add(0, fres_name_off as u32, 1, 1, 0);

    // Model values + dict offsets
    let model_values_off = w.write_offset_placeholder_64();
    let model_dict_off = w.write_offset_placeholder_64();
    if !bfres.models.is_empty() {
        reloc.add(0, model_values_off as u32, 1, 1, 0);
        reloc.add(0, model_dict_off as u32, 1, 1, 0);
    }

    // For v >= 9: 32 zero bytes (BotW is v5, skip)
    if v >= 9 {
        w.write_zeros(32);
    }

    // Skeleton anim values + dict
    let _skel_anim_values_off = w.write_offset_placeholder_64();
    let _skel_anim_dict_off = w.write_offset_placeholder_64();

    // Material anim values + dict
    let _mat_anim_values_off = w.write_offset_placeholder_64();
    let _mat_anim_dict_off = w.write_offset_placeholder_64();

    // Bone visibility anim values + dict
    let _bone_vis_values_off = w.write_offset_placeholder_64();
    let _bone_vis_dict_off = w.write_offset_placeholder_64();

    // Shape anim values + dict
    let _shape_anim_values_off = w.write_offset_placeholder_64();
    let _shape_anim_dict_off = w.write_offset_placeholder_64();

    // Scene anim values + dict
    let _scene_anim_values_off = w.write_offset_placeholder_64();
    let _scene_anim_dict_off = w.write_offset_placeholder_64();

    // Memory pool pointer (i64)
    let memory_pool_off = w.write_offset_placeholder_64();
    reloc.add(0, memory_pool_off as u32, 1, 1, 0);

    // Buffer info pointer (i64)
    let buffer_info_off = w.write_offset_placeholder_64();
    reloc.add(0, buffer_info_off as u32, 1, 1, 0);

    // External file values + dict
    let ext_file_values_off = w.write_offset_placeholder_64();
    let ext_file_dict_off = w.write_offset_placeholder_64();
    if !bfres.external_files.is_empty() {
        reloc.add(0, ext_file_values_off as u32, 1, 1, 0);
        reloc.add(0, ext_file_dict_off as u32, 1, 1, 0);
    }

    // Padding i64
    w.write_u64(0);

    // String pool offset (i64)
    let string_pool_off_ph = w.write_offset_placeholder_64();
    reloc.add(0, string_pool_off_ph as u32, 1, 1, 0);

    // String pool size (u32) - placeholder
    let string_pool_size_pos = w.pos();
    w.write_u32(0);

    // Model count
    w.write_u16(bfres.models.len() as u16);

    // For v >= 9: extra counts
    if v >= 9 {
        w.write_u16(0);
        w.write_u16(0);
    }

    // Animation and external file counts
    w.write_u16(bfres.skeleton_anims.len() as u16);
    w.write_u16(bfres.material_anims.len() as u16);
    w.write_u16(bfres.bone_vis_anims.len() as u16);
    w.write_u16(bfres.shape_anims.len() as u16);
    w.write_u16(bfres.scene_anims.len() as u16);
    w.write_u16(bfres.external_files.len() as u16);

    // Padding
    if v >= 9 {
        w.write_u8(0);
        w.write_u8(if v >= 10 { 1 } else { 0 });
    } else {
        w.write_zeros(6);
    }

    // -----------------------------------------------------------------------
    // Phase 2: Write model sub-files (FMDL blocks)
    // -----------------------------------------------------------------------
    let fres_header_end = w.pos();
    w.set_u16_at(block_offset_pos, fres_header_end as u16);

    let mut model_positions: Vec<usize> = Vec::new();
    let mut model_dict_fixups: Vec<ModelDictFixups> = Vec::new();

    for (mdl_idx, model) in bfres.models.iter().enumerate() {
        w.align(8);
        let mdl_start = w.pos();
        model_positions.push(mdl_start);

        let fmdl = write_fmdl_header(&mut w, model);

        // Register FMDL name and path for string fixup
        string_fixups.push(StringFixup {
            placeholder_pos: fmdl.name_off,
            value: model.name.clone(),
        });
        reloc.add(0, fmdl.name_off as u32, 1, 1, 0);
        string_fixups.push(StringFixup {
            placeholder_pos: fmdl.path_off,
            value: model.path.clone(),
        });
        reloc.add(0, fmdl.path_off as u32, 1, 1, 0);

        // Write FVTX blocks
        let mut fvtx_positions: Vec<usize> = Vec::new();
        let mut fvtx_attr_dict_offs: Vec<usize> = Vec::new();
        let mut fvtx_buf_offset_positions: Vec<usize> = Vec::new();
        let mut fvtx_mem_pool_offs: Vec<usize> = Vec::new();
        let mut fvtx_starts: Vec<usize> = Vec::new();
        for vb in &model.vertex_buffers {
            w.align(8);
            let vpos = w.pos();
            let fvtx_dicts = write_fvtx(&mut w, vb, &mut string_fixups, &mut reloc);
            fvtx_positions.push(vpos);
            fvtx_attr_dict_offs.push(fvtx_dicts.attr_dict_off);
            fvtx_buf_offset_positions.push(fvtx_dicts.buf_offset_pos);
            fvtx_mem_pool_offs.push(fvtx_dicts.mem_pool_off);
            fvtx_starts.push(fvtx_dicts.start);
        }
        if let Some(&first_vpos) = fvtx_positions.first() {
            w.fixup_offset_64(fmdl.fvtx_array_off, first_vpos);
        }

        // Write FSKL
        w.align(8);
        let fskl = write_fskl(&mut w, &model.skeleton, &mut string_fixups, &mut reloc);
        w.fixup_offset_64(fmdl.fskl_off, fskl.start);

        // Write FSHP blocks
        let mut fshp_positions: Vec<usize> = Vec::new();
        let mut fshp_vb_ptr_offs: Vec<usize> = Vec::new();
        let mut fshp_vb_indices: Vec<u16> = Vec::new();
        let mut per_shape_mesh_idx_buf_offs: Vec<Vec<usize>> = Vec::new();
        let mut per_shape_mesh_mem_pool_positions: Vec<Vec<usize>> = Vec::new();
        for shape in &model.shapes {
            w.align(8);
            let fshp = write_fshp(&mut w, shape, &mut string_fixups, &mut reloc);
            fshp_positions.push(fshp.start);
            fshp_vb_ptr_offs.push(fshp.vb_ptr_off);
            fshp_vb_indices.push(shape.vertex_buffer_index);
            per_shape_mesh_idx_buf_offs.push(fshp.mesh_idx_buf_offs);
            per_shape_mesh_mem_pool_positions.push(fshp.mesh_mem_pool_offset_positions);
        }
        if let Some(&first_spos) = fshp_positions.first() {
            w.fixup_offset_64(fmdl.fshp_off, first_spos);
        }

        // Write FMAT blocks
        let mut fmat_positions: Vec<usize> = Vec::new();
        let mut fmat_ri_samp_dict_offs: Vec<(usize, usize)> = Vec::new();
        let mut sa_dict_offs: Vec<Option<ShaderAssignDictOffsets>> = Vec::new();
        for mat in &model.materials {
            w.align(8);
            let fmat = write_fmat(&mut w, mat, &mut string_fixups, &mut reloc);
            fmat_positions.push(fmat.start);
            fmat_ri_samp_dict_offs.push((fmat.ri_dict_off, fmat.samp_dict_off));
            sa_dict_offs.push(fmat.shader_assign_dicts);
        }
        if let Some(&first_mpos) = fmat_positions.first() {
            w.fixup_offset_64(fmdl.fmat_off, first_mpos);
        }

        model_dict_fixups.push(ModelDictFixups {
            fshp_dict_off: fmdl.fshp_dict_off,
            fmat_dict_off: fmdl.fmat_dict_off,
            fskl_bone_dict_off: fskl.bone_dict_off,
            fvtx_attr_dict_offs,
            fvtx_buf_offset_positions,
            fvtx_mem_pool_offs,
            fvtx_starts,
            fshp_vb_ptr_offs,
            fshp_vb_indices,
            mesh_idx_buf_offs: per_shape_mesh_idx_buf_offs,
            mesh_mem_pool_offset_positions: per_shape_mesh_mem_pool_positions,
            fmat_dict_offs: fmat_ri_samp_dict_offs,
            shader_assign_dict_offs: sa_dict_offs,
        });
    }

    // Fix up model values offset
    if let Some(&first_model) = model_positions.first() {
        w.fixup_offset_64(model_values_off, first_model);
    }

    // -----------------------------------------------------------------------
    // Phase 3: Write buffer info
    // -----------------------------------------------------------------------
    w.align(8);
    let buffer_info_start = w.pos();
    w.fixup_offset_64(buffer_info_off, buffer_info_start);

    // Buffer info structure: i64 offset to buffer data + u32 size + padding
    let buf_data_off_ph = w.write_offset_placeholder_64();
    reloc.add(0, buf_data_off_ph as u32, 1, 1, 0);

    let buf_size_field_pos = w.pos();
    w.write_u32(0); // total buffer size (patched later)
    w.write_u32(0); // padding
    w.write_u64(0); // padding

    let buffer_info_end = w.pos();

    // -----------------------------------------------------------------------
    // Phase 4: Write external file entries
    // -----------------------------------------------------------------------
    let mut ef_data_off_phs: Vec<usize> = Vec::new();
    if !bfres.external_files.is_empty() {
        w.align(8);
        let ef_entries_start = w.pos();
        w.fixup_offset_64(ext_file_values_off, ef_entries_start);

        for ef in &bfres.external_files {
            let doff = w.write_offset_placeholder_64();
            reloc.add(0, doff as u32, 1, 1, 0);
            ef_data_off_phs.push(doff);
            w.write_u32(ef.data.len() as u32);
            w.write_u32(0); // padding
        }

        // Write external file data blocks
        for (i, ef) in bfres.external_files.iter().enumerate() {
            w.align(8);
            let data_start = w.pos();
            w.fixup_offset_64(ef_data_off_phs[i], data_start);
            w.write_bytes(&ef.data);
        }
    }

    // -----------------------------------------------------------------------
    // Phase 5: Write string table (must come before dicts)
    // -----------------------------------------------------------------------
    w.align(8);
    let pool_start = st.write(&mut w);
    w.fixup_offset_64(string_pool_off_ph, pool_start);

    let string_pool_size = (w.pos() - pool_start) as u32;
    w.set_u32_at(string_pool_size_pos, string_pool_size);

    // Fix up FRES header name offset (u32 absolute from file start)
    let name_abs = st.get_position(&bfres.name, pool_start);
    w.set_u32_at(name_offset_pos, name_abs as u32);

    // Fix up all string offset placeholders
    for fixup in &string_fixups {
        let target = st.get_position(&fixup.value, pool_start);
        w.fixup_offset_64(fixup.placeholder_pos, target);
    }

    // -----------------------------------------------------------------------
    // Phase 6: Write dicts (after string table so strings are resolved)
    // -----------------------------------------------------------------------

    // Model dict
    if !bfres.models.is_empty() {
        w.align(8);
        let mut model_dict = DictBuilder::new();
        for mdl in &bfres.models {
            model_dict.add(&mdl.name);
        }
        let dict_pos = model_dict.write(&mut w, &st, pool_start);
        w.fixup_offset_64(model_dict_off, dict_pos);
    }

    // Sub-file dicts (bone, shape, material, attribute, render info, sampler, shader assign)
    for (mdl_idx, model) in bfres.models.iter().enumerate() {
        let fixups = &model_dict_fixups[mdl_idx];

        // Bone dict (in FSKL)
        if !model.skeleton.bones.is_empty() {
            w.align(8);
            let mut bone_dict = DictBuilder::new();
            for bone in &model.skeleton.bones {
                bone_dict.add(&bone.name);
            }
            let dict_pos = bone_dict.write(&mut w, &st, pool_start);
            w.fixup_offset_64(fixups.fskl_bone_dict_off, dict_pos);
            reloc.add(0, fixups.fskl_bone_dict_off as u32, 1, 1, 0);
        }

        // Shape dict (in FMDL)
        if !model.shapes.is_empty() {
            w.align(8);
            let mut shape_dict = DictBuilder::new();
            for shape in &model.shapes {
                shape_dict.add(&shape.name);
            }
            let dict_pos = shape_dict.write(&mut w, &st, pool_start);
            w.fixup_offset_64(fixups.fshp_dict_off, dict_pos);
            reloc.add(0, fixups.fshp_dict_off as u32, 1, 1, 0);
        }

        // Material dict (in FMDL)
        if !model.materials.is_empty() {
            w.align(8);
            let mut mat_dict = DictBuilder::new();
            for mat in &model.materials {
                mat_dict.add(&mat.name);
            }
            let dict_pos = mat_dict.write(&mut w, &st, pool_start);
            w.fixup_offset_64(fixups.fmat_dict_off, dict_pos);
            reloc.add(0, fixups.fmat_dict_off as u32, 1, 1, 0);
        }

        // Per-FVTX attribute dicts
        for (vb_idx, vb) in model.vertex_buffers.iter().enumerate() {
            if !vb.attributes.is_empty() {
                w.align(8);
                let mut attr_dict = DictBuilder::new();
                for attr in &vb.attributes {
                    attr_dict.add(&attr.name);
                }
                let dict_pos = attr_dict.write(&mut w, &st, pool_start);
                let off = fixups.fvtx_attr_dict_offs[vb_idx];
                w.fixup_offset_64(off, dict_pos);
                reloc.add(0, off as u32, 1, 1, 0);
            }
        }

        // Per-FMAT render info dicts, sampler dicts, and shader assign dicts
        for (mat_idx, mat) in model.materials.iter().enumerate() {
            let (ri_dict_off, samp_dict_off) = fixups.fmat_dict_offs[mat_idx];

            // Render info dict
            if !mat.render_infos.is_empty() {
                w.align(8);
                let mut ri_dict = DictBuilder::new();
                for ri in &mat.render_infos {
                    ri_dict.add(&ri.name);
                }
                let dict_pos = ri_dict.write(&mut w, &st, pool_start);
                w.fixup_offset_64(ri_dict_off, dict_pos);
                reloc.add(0, ri_dict_off as u32, 1, 1, 0);
            }

            // Sampler dict
            if !mat.samplers.is_empty() {
                w.align(8);
                let mut samp_dict = DictBuilder::new();
                for s in &mat.samplers {
                    samp_dict.add(&s.name);
                }
                let dict_pos = samp_dict.write(&mut w, &st, pool_start);
                w.fixup_offset_64(samp_dict_off, dict_pos);
                reloc.add(0, samp_dict_off as u32, 1, 1, 0);
            }

            // Shader assign sub-dicts (attrib assign, sampler assign, shader options)
            if let Some(sa_dicts) = &fixups.shader_assign_dict_offs[mat_idx] {
                if let Some(sa) = &mat.shader_assign {
                    if !sa.attrib_assigns.is_empty() {
                        w.align(8);
                        let mut aa_dict = DictBuilder::new();
                        for (k, _) in &sa.attrib_assigns {
                            aa_dict.add(k);
                        }
                        let dict_pos = aa_dict.write(&mut w, &st, pool_start);
                        w.fixup_offset_64(sa_dicts.aa_dict_off, dict_pos);
                        reloc.add(0, sa_dicts.aa_dict_off as u32, 1, 1, 0);
                    }

                    if !sa.sampler_assigns.is_empty() {
                        w.align(8);
                        let mut sam_dict = DictBuilder::new();
                        for (k, _) in &sa.sampler_assigns {
                            sam_dict.add(k);
                        }
                        let dict_pos = sam_dict.write(&mut w, &st, pool_start);
                        w.fixup_offset_64(sa_dicts.sam_dict_off, dict_pos);
                        reloc.add(0, sa_dicts.sam_dict_off as u32, 1, 1, 0);
                    }

                    if !sa.shader_options.is_empty() {
                        w.align(8);
                        let mut opt_dict = DictBuilder::new();
                        for (k, _) in &sa.shader_options {
                            opt_dict.add(k);
                        }
                        let dict_pos = opt_dict.write(&mut w, &st, pool_start);
                        w.fixup_offset_64(sa_dicts.opt_dict_off, dict_pos);
                        reloc.add(0, sa_dicts.opt_dict_off as u32, 1, 1, 0);
                    }
                }
            }
        }
    }

    // External file dict
    if !bfres.external_files.is_empty() {
        w.align(8);
        let mut ef_dict = DictBuilder::new();
        for ef in &bfres.external_files {
            ef_dict.add(&ef.name);
        }
        let dict_pos = ef_dict.write(&mut w, &st, pool_start);
        w.fixup_offset_64(ext_file_dict_off, dict_pos);
    }

    // -----------------------------------------------------------------------
    // Phase 7: Memory pool (272 bytes of zeros, aligned to 4096)
    // -----------------------------------------------------------------------
    w.align(4096);
    let memory_pool_start = w.pos();
    w.fixup_offset_64(memory_pool_off, memory_pool_start);
    w.write_zeros(272);

    // -----------------------------------------------------------------------
    // Phase 7.5: Fix up FVTX memory pool pointers and FSHP vertex buffer pointers
    // -----------------------------------------------------------------------
    for (mdl_idx, model) in bfres.models.iter().enumerate() {
        let fixups = &model_dict_fixups[mdl_idx];

        // Fix up FVTX memory pool pointers → point to memory pool section
        for &mp_off in &fixups.fvtx_mem_pool_offs {
            w.fixup_offset_64(mp_off, memory_pool_start);
            reloc.add(0, mp_off as u32, 1, 1, 0);
        }

        // Fix up FSHP vertex buffer pointers → point to the correct FVTX block
        for (shp_idx, &vb_ptr_off) in fixups.fshp_vb_ptr_offs.iter().enumerate() {
            let vb_idx = fixups.fshp_vb_indices[shp_idx] as usize;
            if vb_idx < fixups.fvtx_starts.len() {
                let fvtx_pos = fixups.fvtx_starts[vb_idx];
                w.fixup_offset_64(vb_ptr_off, fvtx_pos);
            }
        }
    }

    // -----------------------------------------------------------------------
    // Phase 8: Buffer data (index buffers then vertex buffers, 8-byte aligned)
    //          Track positions for fixup.
    // -----------------------------------------------------------------------
    w.align(8);
    let buffer_data_start = w.pos();
    w.fixup_offset_64(buf_data_off_ph, buffer_data_start);

    // Write index buffer data: for each model, for each shape, for each mesh
    // Track positions for mesh index buffer pointer fixups.
    let mut idx_buf_positions: Vec<Vec<Vec<usize>>> = Vec::new(); // [model][shape][mesh]
    for (mdl_idx, model) in bfres.models.iter().enumerate() {
        let mut per_shape: Vec<Vec<usize>> = Vec::new();
        for shape in &model.shapes {
            let mut per_mesh: Vec<usize> = Vec::new();
            for mesh in &shape.meshes {
                if !mesh.index_data.is_empty() {
                    w.align(8);
                    let pos = w.pos();
                    per_mesh.push(pos);
                    w.write_bytes(&mesh.index_data);
                } else {
                    per_mesh.push(0);
                }
            }
            per_shape.push(per_mesh);
        }
        idx_buf_positions.push(per_shape);
    }

    // Write vertex buffer data: for each model, for each vertex buffer, for each buffer
    // Track the start position of each FVTX's first buffer for BufferOffset fixup.
    let mut vb_first_buf_positions: Vec<Vec<usize>> = Vec::new(); // [model][vb]
    for model in &bfres.models {
        let mut per_vb: Vec<usize> = Vec::new();
        for vb in &model.vertex_buffers {
            let mut first_pos = 0usize;
            for (buf_idx, buf) in vb.buffers.iter().enumerate() {
                if !buf.data.is_empty() {
                    w.align(8);
                    let pos = w.pos();
                    if buf_idx == 0 {
                        first_pos = pos;
                    }
                    w.write_bytes(&buf.data);
                }
            }
            per_vb.push(first_pos);
        }
        vb_first_buf_positions.push(per_vb);
    }

    let buffer_data_end = w.pos();
    let total_buffer_size = (buffer_data_end - buffer_data_start) as u32;
    w.set_u32_at(buf_size_field_pos, total_buffer_size);

    // -----------------------------------------------------------------------
    // Phase 8.5: Fix up buffer data pointers now that we know positions
    // -----------------------------------------------------------------------
    for (mdl_idx, model) in bfres.models.iter().enumerate() {
        let fixups = &model_dict_fixups[mdl_idx];

        // Fix up FVTX BufferOffset (u32) — offset from buffer_data_start
        for (vb_idx, &buf_off_pos) in fixups.fvtx_buf_offset_positions.iter().enumerate() {
            if vb_idx < vb_first_buf_positions[mdl_idx].len() {
                let first_buf = vb_first_buf_positions[mdl_idx][vb_idx];
                if first_buf > 0 {
                    let offset = (first_buf - buffer_data_start) as u32;
                    w.set_u32_at(buf_off_pos, offset);
                }
            }
        }

        // Fix up mesh index buffer data pointers and memory pool offsets
        for (shp_idx, shape) in model.shapes.iter().enumerate() {
            if shp_idx >= fixups.mesh_idx_buf_offs.len() {
                break;
            }
            for (mesh_idx, _mesh) in shape.meshes.iter().enumerate() {
                if mesh_idx >= fixups.mesh_idx_buf_offs[shp_idx].len() {
                    break;
                }
                let idx_pos = idx_buf_positions[mdl_idx][shp_idx][mesh_idx];
                if idx_pos > 0 {
                    // Fix up i64 index buffer data pointer
                    let placeholder = fixups.mesh_idx_buf_offs[shp_idx][mesh_idx];
                    w.fixup_offset_64(placeholder, idx_pos);
                    reloc.add(0, placeholder as u32, 1, 1, 0);

                    // Fix up u32 memory pool offset (offset from buffer_data_start)
                    let mpo_pos = fixups.mesh_mem_pool_offset_positions[shp_idx][mesh_idx];
                    let offset = (idx_pos - buffer_data_start) as u32;
                    w.set_u32_at(mpo_pos, offset);
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Phase 9: Write relocation table
    // -----------------------------------------------------------------------
    let section_boundaries: [u32; 5] = [
        memory_pool_start as u32,         // section 0 end (main data)
        buffer_info_end as u32,           // section 1 end (buffer info)
        buffer_data_end as u32,           // section 2 end (buffer data)
        (memory_pool_start + 272) as u32, // section 3 end (memory pool)
        buffer_data_end as u32,           // section 4 end (external files)
    ];

    w.align(8);
    let reloc_table_start = w.pos();
    reloc.write(&mut w, &section_boundaries);

    // -----------------------------------------------------------------------
    // Phase 10: Fix up FRES header fields
    // -----------------------------------------------------------------------
    let file_size = w.pos() as u32;
    w.set_u32_at(reloc_offset_pos, reloc_table_start as u32);
    w.set_u32_at(file_size_pos, file_size);

    Ok(w.into_inner())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::convert::convert_to_switch;
    use crate::wiiu;

    #[test]
    fn write_switch_bfres_from_wiiu_fixture() {
        let data = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/Animal_Boar_Big.wiiu.bfres"
        ))
        .expect("fixture file should exist");

        let mut bfres = wiiu::parse(&data).expect("should parse Wii U BFRES");
        convert_to_switch(&mut bfres);

        let output = write(&bfres).expect("should write Switch BFRES");

        // Verify magic is "FRES"
        assert_eq!(&output[0..4], b"FRES", "magic should be FRES");

        // Verify padding spaces
        assert_eq!(&output[4..8], b"    ", "should have space padding");

        // Verify BOM is 0xFEFF (little-endian: bytes FF FE in file)
        let bom = u16::from_le_bytes([output[12], output[13]]);
        assert_eq!(bom, 0xFEFF, "BOM should be 0xFEFF for little-endian");

        // Verify version matches (0, 5, 0, 3)
        let version = u32::from_le_bytes([output[8], output[9], output[10], output[11]]);
        assert_eq!(
            version, 0x00050003,
            "version should be 0x00050003 for BotW Switch"
        );

        // File size should be > 0 and match the stored field
        let stored_size =
            u32::from_le_bytes([output[0x1C], output[0x1D], output[0x1E], output[0x1F]]);
        assert!(stored_size > 0, "file size should be > 0");
        assert_eq!(
            stored_size as usize,
            output.len(),
            "stored file size should match actual output length"
        );

        // Verify _STR magic appears in the output
        let has_str = output.windows(4).any(|win| win == b"_STR");
        assert!(has_str, "output should contain _STR string table magic");

        // Verify _RLT magic appears in the output
        let has_rlt = output.windows(4).any(|win| win == b"_RLT");
        assert!(has_rlt, "output should contain _RLT relocation table magic");

        // Verify FMDL magic appears (we have models)
        let has_fmdl = output.windows(4).any(|win| win == b"FMDL");
        assert!(has_fmdl, "output should contain FMDL model magic");

        // Verify the file name string appears in the string table
        let has_name = output
            .windows(b"Animal_Boar_Big".len())
            .any(|win| win == b"Animal_Boar_Big");
        assert!(has_name, "output should contain the file name string");

        // Verify the name offset at 0x10 points to the name string
        let name_off =
            u32::from_le_bytes([output[0x10], output[0x11], output[0x12], output[0x13]]);
        assert!(
            (name_off as usize) < output.len(),
            "name offset should be within file bounds"
        );
        let name_end = name_off as usize + 15;
        if name_end <= output.len() {
            assert_eq!(
                &output[name_off as usize..name_end],
                b"Animal_Boar_Big",
                "name offset should point to the file name"
            );
        }

        // Verify block_offset points to FMDL
        let block_off = u16::from_le_bytes([output[0x16], output[0x17]]);
        assert!(block_off > 0, "block offset should be > 0");
        if (block_off as usize + 4) <= output.len() {
            assert_eq!(
                &output[block_off as usize..block_off as usize + 4],
                b"FMDL",
                "block offset should point to FMDL magic"
            );
        }

        eprintln!(
            "Switch BFRES written: {} bytes, name_off={:#x}, block_off={:#x}",
            output.len(),
            name_off,
            block_off
        );
    }

    #[test]
    fn public_api_convert_wiiu_to_switch() {
        let data = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/Animal_Boar_Big.wiiu.bfres"
        ))
        .expect("fixture file should exist");

        let output = crate::convert_wiiu_to_switch(&data).expect("public API should work");
        assert!(crate::is_switch_bfres(&output), "output should be Switch BFRES");
        assert!(!crate::is_wiiu_bfres(&output), "output should NOT be Wii U BFRES");

        // Write to Downloads for manual inspection
        if let Some(home) = std::env::var_os("HOME") {
            let out_path = std::path::PathBuf::from(home)
                .join("Downloads")
                .join("Animal_Boar_Big.switch.bfres");
            if let Err(e) = std::fs::write(&out_path, &output) {
                eprintln!("Could not write test output: {}", e);
            } else {
                eprintln!("Wrote Switch BFRES to: {}", out_path.display());
            }
        }
    }

    #[test]
    fn write_empty_bfres() {
        let bfres = BfresFile {
            name: "empty".to_string(),
            version: (0, 5, 0, 3),
            alignment: 0x0C,
            models: Vec::new(),
            textures: Vec::new(),
            skeleton_anims: Vec::new(),
            material_anims: Vec::new(),
            bone_vis_anims: Vec::new(),
            shape_anims: Vec::new(),
            scene_anims: Vec::new(),
            external_files: Vec::new(),
            shader_param_anims: Vec::new(),
            color_anims: Vec::new(),
            tex_srt_anims: Vec::new(),
            tex_pattern_anims: Vec::new(),
            mat_vis_anims: Vec::new(),
        };

        let output = write(&bfres).expect("should write empty BFRES");

        assert_eq!(&output[0..4], b"FRES");
        let bom = u16::from_le_bytes([output[12], output[13]]);
        assert_eq!(bom, 0xFEFF);

        let stored_size =
            u32::from_le_bytes([output[0x1C], output[0x1D], output[0x1E], output[0x1F]]);
        assert_eq!(stored_size as usize, output.len());
    }
}
