//! Wii U BFRES model (FMDL) parser.
//!
//! Parses FMDL, FSKL, FSHP, FMAT, and FVTX sub-files from big-endian Wii U BFRES data.

use crate::binary::BigEndianReader;
use crate::error::{BfresError, Result};
use crate::model::*;
use crate::wiiu::dict::parse_dict;
use crate::wiiu::header::read_rel_offset;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Read a relative-offset string: reads a u32 relative offset, resolves it,
/// and reads the null-terminated string at that location.
fn read_string(reader: &mut BigEndianReader) -> Result<String> {
    let field_pos = reader.pos();
    let rel = reader.read_u32()?;
    if rel == 0 {
        return Ok(String::new());
    }
    let abs = (field_pos as u32).wrapping_add(rel) as usize;
    reader.read_string_at(abs)
}

/// Read a relative offset and resolve to absolute. Returns 0 if the raw value is 0.
fn read_offset(reader: &mut BigEndianReader) -> Result<usize> {
    let v = read_rel_offset(reader)?;
    Ok(v as usize)
}

// ---------------------------------------------------------------------------
// FMDL (Model)
// ---------------------------------------------------------------------------

/// Parse a Wii U FMDL sub-file at `offset`. `version` is the BFRES file version.
pub fn parse_model(
    reader: &mut BigEndianReader,
    offset: usize,
    version: u32,
) -> Result<Model> {
    reader.seek(offset);

    let magic = reader.read_magic()?;
    if magic != *b"FMDL" {
        return Err(BfresError::InvalidMagic(magic));
    }

    // C# Model.Load (Wii U branch):
    //   Name            = loader.LoadString();
    //   Path            = loader.LoadString();
    //   Skeleton        = loader.Load<Skeleton>();          // offset -> FSKL
    //   ofsVertexBufferList = loader.ReadOffset();          // u32 offset to FVTX list
    //   Shapes          = loader.LoadDict<Shape>();         // offset -> dict
    //   Materials       = loader.LoadDict<Material>();      // offset -> dict
    //   UserData        = loader.LoadDict<UserData>();      // offset -> dict
    //   numVertexBuffer = u16
    //   numShape        = u16
    //   numMaterial     = u16
    //   numUserData     = u16
    //   totalVertexCount= u32
    //   if version >= 0x03030000: userPointer = u32
    //   VertexBuffers   = loader.LoadList<VertexBuffer>(numVertexBuffer, ofsVertexBufferList)

    let name = read_string(reader)?;
    let path = read_string(reader)?;
    let skeleton_offset = read_offset(reader)?;
    let vertex_buffer_list_offset = read_offset(reader)?;
    let shapes_dict_offset = read_offset(reader)?;
    let materials_dict_offset = read_offset(reader)?;
    let user_data_dict_offset = read_offset(reader)?;
    let num_vertex_buffer = reader.read_u16()?;
    let _num_shape = reader.read_u16()?;
    let _num_material = reader.read_u16()?;
    let _num_user_data = reader.read_u16()?;
    let total_vertex_count = reader.read_u32()?;

    if version >= 0x03030000 {
        let _user_pointer = reader.read_u32()?;
    }

    // Parse skeleton
    let skeleton = if skeleton_offset > 0 {
        parse_skeleton(reader, skeleton_offset, version)?
    } else {
        Skeleton {
            flags: 0,
            bones: Vec::new(),
            smooth_indices: Vec::new(),
            rigid_indices: Vec::new(),
            inverse_model_matrices: Vec::new(),
        }
    };

    // Parse shapes
    let mut shapes = Vec::new();
    if shapes_dict_offset > 0 {
        let shape_entries = parse_dict(reader, shapes_dict_offset)?;
        for entry in &shape_entries {
            shapes.push(parse_shape(reader, entry.data_offset, version)?);
        }
    }

    // Parse materials
    let mut materials = Vec::new();
    if materials_dict_offset > 0 {
        let mat_entries = parse_dict(reader, materials_dict_offset)?;
        for entry in &mat_entries {
            materials.push(parse_material(reader, entry.data_offset, version)?);
        }
    }

    // Parse user data
    let user_data = if user_data_dict_offset > 0 {
        parse_user_data_dict(reader, user_data_dict_offset)?
    } else {
        Vec::new()
    };

    // Parse vertex buffers (loaded as a list at vertex_buffer_list_offset)
    let mut vertex_buffers = Vec::new();
    if vertex_buffer_list_offset > 0 && num_vertex_buffer > 0 {
        reader.seek(vertex_buffer_list_offset);
        for _ in 0..num_vertex_buffer {
            let vb = parse_vertex_buffer_inline(reader, version)?;
            vertex_buffers.push(vb);
        }
    }

    Ok(Model {
        name,
        path,
        skeleton,
        shapes,
        materials,
        vertex_buffers,
        user_data,
        total_vertex_count,
    })
}

// ---------------------------------------------------------------------------
// FSKL (Skeleton)
// ---------------------------------------------------------------------------

fn parse_skeleton(
    reader: &mut BigEndianReader,
    offset: usize,
    version: u32,
) -> Result<Skeleton> {
    reader.seek(offset);

    let magic = reader.read_magic()?;
    if magic != *b"FSKL" {
        return Err(BfresError::InvalidMagic(magic));
    }

    // C# Skeleton.Load (Wii U branch):
    //   _flags          = u32
    //   numBone         = u16
    //   numSmoothMatrix = u16
    //   numRigidMatrix  = u16
    //   padding         = 2 bytes (loader.Seek(2))
    //   Bones           = loader.LoadDict<Bone>()          // offset -> dict (each entry loads a Bone inline)
    //   ofsBoneList     = loader.ReadOffset()               // we skip this (only dict needed)
    //   MatrixToBoneList= loader.LoadCustom(...)            // offset -> u16 array
    //   if version >= 0x03040000:
    //     InverseModelMatrices = loader.LoadCustom(...)     // offset -> Matrix3x4 array
    //   userPointer     = u32

    let flags = reader.read_u32()?;
    let num_bone = reader.read_u16()?;
    let num_smooth_matrix = reader.read_u16()?;
    let num_rigid_matrix = reader.read_u16()?;
    reader.skip(2); // padding

    let bone_dict_offset = read_offset(reader)?;
    let _bone_list_offset = read_offset(reader)?;
    let matrix_to_bone_list_offset = read_offset(reader)?;

    let inverse_model_matrices_offset = if version >= 0x03040000 {
        read_offset(reader)?
    } else {
        0
    };

    let _user_pointer = reader.read_u32()?;

    // Parse bones from dictionary
    let mut bones = Vec::new();
    if bone_dict_offset > 0 && num_bone > 0 {
        let bone_entries = parse_dict(reader, bone_dict_offset)?;
        for (idx, entry) in bone_entries.iter().enumerate() {
            let bone = parse_bone(reader, entry.data_offset, version, idx as u16)?;
            bones.push(bone);
        }
    }

    // Read matrix-to-bone list (smooth + rigid indices together)
    let total_matrices = (num_smooth_matrix as usize) + (num_rigid_matrix as usize);
    let mut matrix_to_bone = Vec::new();
    if matrix_to_bone_list_offset > 0 && total_matrices > 0 {
        reader.seek(matrix_to_bone_list_offset);
        for _ in 0..total_matrices {
            matrix_to_bone.push(reader.read_u16()?);
        }
    }

    // Split matrix-to-bone into smooth and rigid indices
    let smooth_count = num_smooth_matrix as usize;
    let smooth_indices: Vec<u16> = matrix_to_bone.iter().take(smooth_count).copied().collect();
    let rigid_indices: Vec<u16> = matrix_to_bone.iter().skip(smooth_count).copied().collect();

    // Read inverse model matrices
    let mut inverse_model_matrices = Vec::new();
    if inverse_model_matrices_offset > 0 && num_smooth_matrix > 0 {
        reader.seek(inverse_model_matrices_offset);
        for _ in 0..num_smooth_matrix {
            let mut mat = [0.0f32; 12];
            for j in 0..12 {
                mat[j] = reader.read_f32()?;
            }
            inverse_model_matrices.push(mat);
        }
    }

    Ok(Skeleton {
        flags,
        bones,
        smooth_indices,
        rigid_indices,
        inverse_model_matrices,
    })
}

// ---------------------------------------------------------------------------
// Bone
// ---------------------------------------------------------------------------

fn parse_bone(
    reader: &mut BigEndianReader,
    offset: usize,
    version: u32,
    _fallback_index: u16,
) -> Result<Bone> {
    reader.seek(offset);

    // C# Bone.Load (Wii U branch):
    //   Name          = loader.LoadString()
    //   idx           = u16
    //   ParentIndex   = i16
    //   SmoothMatrixIndex = i16
    //   RigidMatrixIndex  = i16
    //   BillboardIndex    = i16
    //   numUserData       = u16
    //   _flags            = u32
    //   Scale             = Vector3F (3 x f32)
    //   Rotation          = Vector4F (4 x f32)
    //   Position          = Vector3F (3 x f32)
    //   UserData          = loader.LoadDict<UserData>()
    //   if version < 0x03040000: InverseMatrix = Matrix3x4

    let name = read_string(reader)?;
    let index = reader.read_u16()?;
    let parent_index = reader.read_i16()?;
    let smooth_matrix_index = reader.read_i16()?;
    let rigid_matrix_index = reader.read_i16()?;
    let billboard_index = reader.read_i16()?;
    let num_user_data = reader.read_u16()?;
    let flags = reader.read_u32()?;

    let scale = [reader.read_f32()?, reader.read_f32()?, reader.read_f32()?];
    let rotation = [
        reader.read_f32()?,
        reader.read_f32()?,
        reader.read_f32()?,
        reader.read_f32()?,
    ];
    let translation = [reader.read_f32()?, reader.read_f32()?, reader.read_f32()?];

    let user_data_dict_offset = read_offset(reader)?;

    if version < 0x03040000 {
        // Old versions store inverse matrix inline (12 floats = 48 bytes)
        reader.skip(48);
    }

    let user_data = if user_data_dict_offset > 0 && num_user_data > 0 {
        parse_user_data_dict(reader, user_data_dict_offset)?
    } else {
        Vec::new()
    };

    Ok(Bone {
        name,
        index,
        parent_index,
        smooth_matrix_index,
        rigid_matrix_index,
        billboard_index,
        flags,
        scale,
        rotation,
        translation,
        user_data,
    })
}

// ---------------------------------------------------------------------------
// FSHP (Shape)
// ---------------------------------------------------------------------------

fn parse_shape(
    reader: &mut BigEndianReader,
    offset: usize,
    version: u32,
) -> Result<Shape> {
    reader.seek(offset);

    let magic = reader.read_magic()?;
    if magic != *b"FSHP" {
        return Err(BfresError::InvalidMagic(magic));
    }

    // C# Shape.Load (Wii U branch):
    //   Name               = loader.LoadString()
    //   Flags              = loader.ReadEnum<ShapeFlags>(true)    // u32
    //   idx                = u16
    //   MaterialIndex      = u16
    //   BoneIndex          = u16
    //   VertexBufferIndex  = u16
    //   numSkinBoneIndex   = u16
    //   VertexSkinCount    = u8
    //   numMesh            = u8
    //   numKeyShape        = u8
    //   TargetAttribCount  = u8
    //   numSubMeshBoundingNodes = u16
    //
    //   if version >= 0x04050000:
    //     RadiusArray = loader.LoadCustom(() => loader.ReadSingles(numMesh))  // offset -> f32[]
    //   else:
    //     RadiusArray = loader.ReadSingles(1)     // inline single f32
    //
    //   VertexBuffer = loader.Load<VertexBuffer>()  // offset -> FVTX (we just read the offset, skip parsing here)
    //   Meshes = loader.LoadList<Mesh>(numMesh)   // offset -> Mesh[]
    //   SkinBoneIndices = loader.LoadCustom(...)   // offset -> u16[]
    //   KeyShapes = loader.LoadDict<KeyShape>()   // offset -> dict
    //
    //   if numSubMeshBoundingNodes == 0:
    //     (compute count, then load boundings)
    //   else:
    //     SubMeshBoundingNodes = loader.LoadList<BoundingNode>(numSubMeshBoundingNodes)
    //     SubMeshBoundings = loader.LoadCustom(...)
    //     SubMeshBoundingIndices = loader.LoadCustom(...)
    //
    //   userPointer = u32

    let name = read_string(reader)?;
    let flags = reader.read_u32()?;
    let index = reader.read_u16()?;
    let material_index = reader.read_u16()?;
    let bone_index = reader.read_u16()?;
    let vertex_buffer_index = reader.read_u16()?;
    let num_skin_bone_index = reader.read_u16()?;
    let vertex_skin_count = reader.read_u8()?;
    let num_mesh = reader.read_u8()?;
    let num_key_shape = reader.read_u8()?;
    let _target_attrib_count = reader.read_u8()?;
    let num_sub_mesh_bounding_nodes = reader.read_u16()?;

    // Radius
    let radius_offset;
    let bounding_radius;
    if version >= 0x04050000 {
        radius_offset = read_offset(reader)?;
        bounding_radius = Vec::new(); // will be filled below
    } else {
        radius_offset = 0;
        let r = reader.read_f32()?;
        bounding_radius = vec![r];
    }

    let _vertex_buffer_offset = read_offset(reader)?; // FVTX offset (already parsed by model)
    let meshes_offset = read_offset(reader)?;
    let skin_bone_indices_offset = read_offset(reader)?;
    let key_shapes_dict_offset = read_offset(reader)?;

    // Boundings
    let (bounding_nodes_offset, boundings_offset, _bounding_indices_offset);
    if num_sub_mesh_bounding_nodes == 0 {
        bounding_nodes_offset = 0;
        boundings_offset = read_offset(reader)?;
        _bounding_indices_offset = 0;
    } else {
        bounding_nodes_offset = read_offset(reader)?;
        boundings_offset = read_offset(reader)?;
        _bounding_indices_offset = read_offset(reader)?;
    }

    let _user_pointer = reader.read_u32()?;

    // Now read deferred data

    // Radius array (for version >= 0x04050000)
    let bounding_radius = if radius_offset > 0 && version >= 0x04050000 && num_mesh > 0 {
        reader.seek(radius_offset);
        let mut radii = Vec::with_capacity(num_mesh as usize);
        for _ in 0..num_mesh {
            radii.push(reader.read_f32()?);
        }
        radii
    } else {
        bounding_radius
    };

    // Parse meshes
    let mut meshes = Vec::new();
    if meshes_offset > 0 && num_mesh > 0 {
        reader.seek(meshes_offset);
        for _ in 0..num_mesh {
            let mesh = parse_mesh_inline(reader)?;
            meshes.push(mesh);
        }
    }

    // Skin bone indices
    let mut skin_bone_indices = Vec::new();
    if skin_bone_indices_offset > 0 && num_skin_bone_index > 0 {
        reader.seek(skin_bone_indices_offset);
        for _ in 0..num_skin_bone_index {
            skin_bone_indices.push(reader.read_u16()?);
        }
    }

    // Key shapes
    let mut key_shapes = Vec::new();
    if key_shapes_dict_offset > 0 && num_key_shape > 0 {
        let key_entries = parse_dict(reader, key_shapes_dict_offset)?;
        for entry in &key_entries {
            key_shapes.push(KeyShape {
                name: entry.name.clone(),
                index: 0, // KeyShape has TargetAttribIndices (20 bytes) + offsets (4 bytes), we just store name/index
            });
        }
    }

    // Bounding boxes
    let mut bounding_boxes = Vec::new();
    let total_bounding_count;
    if num_sub_mesh_bounding_nodes == 0 {
        // Compute count: numMesh + sum(submesh counts) for version >= 0x04050000
        // Or 1 + submesh count + 1 for older versions
        if version >= 0x04050000 {
            total_bounding_count = (num_mesh as usize)
                + meshes.iter().map(|m| m.sub_meshes.len()).sum::<usize>();
        } else if !meshes.is_empty() {
            total_bounding_count = 1 + meshes[0].sub_meshes.len() + 1;
        } else {
            total_bounding_count = 0;
        }
    } else {
        total_bounding_count = num_sub_mesh_bounding_nodes as usize;
    }

    if boundings_offset > 0 && total_bounding_count > 0 {
        reader.seek(boundings_offset);
        for _ in 0..total_bounding_count {
            let center = [reader.read_f32()?, reader.read_f32()?, reader.read_f32()?];
            let extent = [reader.read_f32()?, reader.read_f32()?, reader.read_f32()?];
            bounding_boxes.push(BoundingBox { center, extent });
        }
    }

    // Bounding nodes
    let mut bounding_nodes = Vec::new();
    if bounding_nodes_offset > 0 && num_sub_mesh_bounding_nodes > 0 {
        reader.seek(bounding_nodes_offset);
        for _ in 0..num_sub_mesh_bounding_nodes {
            // C# BoundingNode.Load:
            //   LeftChildIndex  = u16
            //   RightChildIndex = u16
            //   Unknown         = u16
            //   NextSibling     = u16
            //   SubMeshIndex    = u16
            //   SubMeshCount    = u16
            let left_child_index = reader.read_u16()?;
            let right_child_index = reader.read_u16()?;
            let unknown = reader.read_u16()?;
            let next_sibling_index = reader.read_u16()?;
            let sub_mesh_index = reader.read_u16()?;
            let sub_mesh_count = reader.read_u16()?;
            bounding_nodes.push(BoundingNode {
                left_child_index,
                next_sibling_index,
                right_child_index,
                unknown,
                sub_mesh_index,
                sub_mesh_count,
            });
        }
    }

    Ok(Shape {
        name,
        flags,
        index,
        material_index,
        bone_index,
        vertex_buffer_index,
        skin_bone_indices,
        vertex_skin_count,
        meshes,
        key_shapes,
        bounding_boxes,
        bounding_radius,
        bounding_nodes,
    })
}

// ---------------------------------------------------------------------------
// Mesh (read inline from a list)
// ---------------------------------------------------------------------------

fn parse_mesh_inline(reader: &mut BigEndianReader) -> Result<Mesh> {
    // C# Mesh.Load (Wii U):
    //   PrimitiveType = loader.ReadEnum<GX2PrimitiveType>(true)    // u32
    //   IndexFormat   = loader.ReadEnum<GX2IndexFormat>(true)      // u32
    //   indexCount    = u32
    //   numSubMesh    = u16
    //   padding       = 2 bytes
    //   SubMeshes     = loader.LoadList<SubMesh>(numSubMesh)       // offset -> SubMesh[]
    //   IndexBuffer   = loader.Load<Buffer>()                      // offset -> Buffer
    //   FirstVertex   = u32

    let primitive_type = reader.read_u32()?;
    let index_format = reader.read_u32()?;
    let index_count = reader.read_u32()?;
    let num_sub_mesh = reader.read_u16()?;
    reader.skip(2); // padding

    let sub_meshes_offset = read_offset(reader)?;
    let index_buffer_offset = read_offset(reader)?;
    let first_vertex = reader.read_u32()?;

    // Read sub meshes
    let mut sub_meshes = Vec::new();
    if sub_meshes_offset > 0 && num_sub_mesh > 0 {
        let saved_pos = reader.pos();
        reader.seek(sub_meshes_offset);
        for _ in 0..num_sub_mesh {
            // C# SubMesh.Load:
            //   Offset = u32
            //   Count  = u32
            let sm_offset = reader.read_u32()?;
            let sm_count = reader.read_u32()?;
            sub_meshes.push(SubMesh {
                offset: sm_offset,
                count: sm_count,
            });
        }
        reader.seek(saved_pos);
    }

    // Read index buffer (Buffer)
    let mut index_data = Vec::new();
    if index_buffer_offset > 0 {
        let saved_pos = reader.pos();
        reader.seek(index_buffer_offset);

        // C# Buffer.Load:
        //   dataPointer    = u32
        //   size           = u32
        //   handle         = u32
        //   Stride         = u16
        //   numBuffering   = u16
        //   contextPointer = u32
        //   Data = loader.LoadCustom(...) -> byte[numBuffering][size]
        let _data_pointer = reader.read_u32()?;
        let size = reader.read_u32()?;
        let _handle = reader.read_u32()?;
        let _stride = reader.read_u16()?;
        let _num_buffering = reader.read_u16()?.max(1);
        let _context_pointer = reader.read_u32()?;

        // The buffer data is stored at an offset that follows (via LoadCustom which reads another offset)
        let data_offset = read_offset(reader)?;
        if data_offset > 0 && size > 0 {
            reader.seek(data_offset);
            // Read first buffering only (all bufferings have the same data)
            let raw = reader.read_bytes(size as usize)?;
            index_data = raw.to_vec();

            // Byte-swap index data based on format
            byteswap_index_data(&mut index_data, index_format);
        }

        reader.seek(saved_pos);
    }

    Ok(Mesh {
        primitive_type,
        index_format,
        index_count,
        first_vertex,
        sub_meshes,
        index_data,
    })
}

// ---------------------------------------------------------------------------
// FMAT (Material)
// ---------------------------------------------------------------------------

fn parse_material(
    reader: &mut BigEndianReader,
    offset: usize,
    version: u32,
) -> Result<Material> {
    eprintln!("[DEBUG parse_material] seeking to offset={:#x}", offset);
    reader.seek(offset);
    eprintln!("[DEBUG parse_material] reader.pos after seek={:#x}", reader.pos());

    let magic = reader.read_magic()?;
    eprintln!("[DEBUG parse_material] magic={:?} reader.pos={:#x}", magic, reader.pos());
    if magic != *b"FMAT" {
        return Err(BfresError::InvalidMagic(magic));
    }

    // C# Material.Load (Wii U):
    //   Name                  = loader.LoadString()
    //   Flags                 = loader.ReadEnum<MaterialFlags>(true)   // u32
    //   idx                   = u16
    //   numRenderInfo         = u16
    //   numSampler            = u8
    //   numTextureRef         = u8
    //   numShaderParam        = u16
    //   numShaderParamVolatile= u16
    //   sizParamSource        = u16
    //   sizParamRaw           = u16
    //   numUserData           = u16
    //   RenderInfos           = loader.LoadDict<RenderInfo>()
    //   RenderState           = loader.Load<RenderState>()
    //   ShaderAssign          = loader.Load<ShaderAssign>()
    //   TextureRefs           = loader.LoadList<TextureRef>(numTextureRef)
    //   ofsSamplerList        = loader.ReadOffset()  (skip)
    //   Samplers              = loader.LoadDict<Sampler>()
    //   ofsShaderParamList    = loader.ReadOffset()  (skip)
    //   ShaderParams          = loader.LoadDict<ShaderParam>()
    //   ShaderParamData       = loader.LoadCustom(() => loader.ReadBytes(sizParamSource))
    //   UserData              = loader.LoadDict<UserData>()
    //   VolatileFlags         = loader.LoadCustom(() => loader.ReadBytes(ceil(numShaderParam/8)))
    //   userPointer           = u32

    let name = read_string(reader)?;
    let flags = reader.read_u32()?;
    let index = reader.read_u16()?;
    let num_render_info = reader.read_u16()?;
    let num_sampler = reader.read_u8()?;
    let num_texture_ref = reader.read_u8()?;
    let num_shader_param = reader.read_u16()?;
    let _num_shader_param_volatile = reader.read_u16()?;
    let siz_param_source = reader.read_u16()?;
    let _siz_param_raw = reader.read_u16()?;
    let num_user_data = reader.read_u16()?;

    let render_info_dict_offset = read_offset(reader)?;
    let render_state_offset = read_offset(reader)?;
    let shader_assign_offset = read_offset(reader)?;
    let texture_ref_list_offset = read_offset(reader)?;
    let _sampler_list_offset = read_offset(reader)?;
    let sampler_dict_offset = read_offset(reader)?;
    let _shader_param_list_offset = read_offset(reader)?;
    let shader_param_dict_offset = read_offset(reader)?;
    let shader_param_data_offset = read_offset(reader)?;
    let user_data_dict_offset = read_offset(reader)?;

    let volatile_flags_offset = if version >= 0x03030000 {
        read_offset(reader)?
    } else {
        0
    };

    let _user_pointer = reader.read_u32()?;

    // Parse render infos
    let render_infos = if render_info_dict_offset > 0 && num_render_info > 0 {
        parse_render_info_dict(reader, render_info_dict_offset)?
    } else {
        Vec::new()
    };

    // Parse render state
    let render_state = if render_state_offset > 0 {
        Some(parse_render_state(reader, render_state_offset)?)
    } else {
        None
    };

    // Parse shader assign
    let shader_assign = if shader_assign_offset > 0 {
        Some(parse_shader_assign(reader, shader_assign_offset)?)
    } else {
        None
    };

    // Parse texture refs
    let mut texture_refs = Vec::new();
    if texture_ref_list_offset > 0 && num_texture_ref > 0 {
        reader.seek(texture_ref_list_offset);
        for _ in 0..num_texture_ref {
            // C# TextureRef.Load (Wii U):
            //   Name = loader.LoadString()
            //   Texture = loader.Load<WiiU.Texture>()  // offset (we just skip it)
            let tex_name = read_string(reader)?;
            let _tex_offset = read_offset(reader)?; // skip texture data offset
            texture_refs.push(TextureRef { name: tex_name });
        }
    }

    // Parse samplers
    let mut samplers = Vec::new();
    if sampler_dict_offset > 0 && num_sampler > 0 {
        let sampler_entries = parse_dict(reader, sampler_dict_offset)?;
        for entry in &sampler_entries {
            let sampler = parse_sampler(reader, entry.data_offset)?;
            samplers.push(sampler);
        }
    }

    // Parse shader params
    let mut shader_params = Vec::new();
    if shader_param_dict_offset > 0 && num_shader_param > 0 {
        let param_entries = parse_dict(reader, shader_param_dict_offset)?;
        for entry in &param_entries {
            let param = parse_shader_param(reader, entry.data_offset, version)?;
            shader_params.push(param);
        }
    }

    // Read shader param data blob
    let shader_param_data = if shader_param_data_offset > 0 && siz_param_source > 0 {
        reader.seek(shader_param_data_offset);
        reader.read_bytes(siz_param_source as usize)?.to_vec()
    } else {
        Vec::new()
    };

    // Parse user data
    let user_data = if user_data_dict_offset > 0 && num_user_data > 0 {
        parse_user_data_dict(reader, user_data_dict_offset)?
    } else {
        Vec::new()
    };

    // Read volatile flags
    let volatile_flags = if volatile_flags_offset > 0 && num_shader_param > 0 {
        let byte_count = (num_shader_param as usize + 7) / 8;
        reader.seek(volatile_flags_offset);
        reader.read_bytes(byte_count)?.to_vec()
    } else {
        Vec::new()
    };

    Ok(Material {
        name,
        flags,
        index,
        render_infos,
        render_state,
        shader_assign,
        shader_params,
        texture_refs,
        samplers,
        user_data,
        shader_param_data,
        volatile_flags,
    })
}

// ---------------------------------------------------------------------------
// RenderInfo
// ---------------------------------------------------------------------------

fn parse_render_info_dict(
    reader: &mut BigEndianReader,
    dict_offset: usize,
) -> Result<Vec<RenderInfo>> {
    let entries = parse_dict(reader, dict_offset)?;
    let mut render_infos = Vec::with_capacity(entries.len());

    for entry in &entries {
        reader.seek(entry.data_offset);

        // C# RenderInfo.Load (Wii U):
        //   count = u16
        //   Type  = ReadEnum<RenderInfoType>(true) // u8 (but after reading u16 it's byte-aligned)
        //   padding = 1 byte
        //   Name  = loader.LoadString()
        //   data based on type

        let count = reader.read_u16()?;
        let ri_type = reader.read_u8()?;
        reader.skip(1); // padding
        let name = read_string(reader)?;

        let value = match ri_type {
            0 => {
                // Int32
                let mut vals = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    vals.push(reader.read_i32()?);
                }
                RenderInfoValue::Int32(vals)
            }
            1 => {
                // Single (f32)
                let mut vals = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    vals.push(reader.read_f32()?);
                }
                RenderInfoValue::Float(vals)
            }
            2 => {
                // String
                let mut vals = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    let s = read_string(reader)?;
                    vals.push(s);
                }
                RenderInfoValue::String(vals)
            }
            _ => {
                // Unknown type, treat as empty int32
                RenderInfoValue::Int32(Vec::new())
            }
        };

        render_infos.push(RenderInfo { name, value });
    }

    Ok(render_infos)
}

// ---------------------------------------------------------------------------
// RenderState
// ---------------------------------------------------------------------------

fn parse_render_state(reader: &mut BigEndianReader, offset: usize) -> Result<RenderState> {
    reader.seek(offset);

    // C# RenderState.Load:
    //   _flags         = u32
    //   PolygonControl = u32  (stored as .Value)
    //   DepthControl   = u32
    //   AlphaControl   = u32
    //   AlphaRefValue  = f32
    //   ColorControl   = u32
    //   BlendTarget    = u32
    //   BlendControl   = u32
    //   BlendColor     = Vector4F (4 x f32)

    let flags = reader.read_u32()?;
    let polygon_control = reader.read_u32()?;
    let depth_control = reader.read_u32()?;
    let alpha_control = reader.read_u32()?;
    let alpha_ref_value = reader.read_f32()?;
    let _color_control = reader.read_u32()?;
    let _blend_target = reader.read_u32()?;
    let blend_control = reader.read_u32()?;
    let blend_color = [
        reader.read_f32()?,
        reader.read_f32()?,
        reader.read_f32()?,
        reader.read_f32()?,
    ];

    Ok(RenderState {
        flags,
        polygon_control,
        depth_control,
        alpha_control,
        alpha_ref_value,
        blend_control,
        blend_color,
    })
}

// ---------------------------------------------------------------------------
// ShaderAssign
// ---------------------------------------------------------------------------

fn parse_shader_assign(reader: &mut BigEndianReader, offset: usize) -> Result<ShaderAssign> {
    reader.seek(offset);

    // C# ShaderAssign.Load (Wii U):
    //   ShaderArchiveName = loader.LoadString()
    //   ShadingModelName  = loader.LoadString()
    //   Revision          = u32
    //   numAttribAssign   = u8
    //   numSamplerAssign  = u8
    //   numShaderOption   = u16
    //   AttribAssigns     = loader.LoadDict<ResString>()   // dict of string-keyed strings
    //   SamplerAssigns    = loader.LoadDict<ResString>()
    //   ShaderOptions     = loader.LoadDict<ResString>()

    let shader_archive_name = read_string(reader)?;
    let shading_model_name = read_string(reader)?;
    let revision = reader.read_u32()?;
    let _num_attrib_assign = reader.read_u8()?;
    let _num_sampler_assign = reader.read_u8()?;
    let _num_shader_option = reader.read_u16()?;

    let attrib_assigns_dict_offset = read_offset(reader)?;
    let sampler_assigns_dict_offset = read_offset(reader)?;
    let shader_options_dict_offset = read_offset(reader)?;

    // Parse attrib assigns: dict where values are inline zero-terminated strings
    let attrib_assigns = parse_string_dict(reader, attrib_assigns_dict_offset)?;
    let sampler_assigns = parse_string_dict(reader, sampler_assigns_dict_offset)?;
    let shader_options = parse_string_dict(reader, shader_options_dict_offset)?;

    Ok(ShaderAssign {
        shader_archive_name,
        shading_model_name,
        attrib_assigns,
        sampler_assigns,
        shader_options,
        revision,
    })
}

/// Parse a dict of (key, value) string pairs. Each dict entry points to an inline
/// ResString (zero-terminated string read at the data_offset).
fn parse_string_dict(
    reader: &mut BigEndianReader,
    dict_offset: usize,
) -> Result<Vec<(String, String)>> {
    if dict_offset == 0 {
        return Ok(Vec::new());
    }
    let entries = parse_dict(reader, dict_offset)?;
    let mut result = Vec::with_capacity(entries.len());
    for entry in &entries {
        // The data at entry.data_offset is a ResString which for Wii U is a
        // zero-terminated string read directly (no offset indirection).
        let value = reader.read_string_at(entry.data_offset).unwrap_or_default();
        result.push((entry.name.clone(), value));
    }
    Ok(result)
}

// ---------------------------------------------------------------------------
// Sampler
// ---------------------------------------------------------------------------

fn parse_sampler(reader: &mut BigEndianReader, offset: usize) -> Result<Sampler> {
    reader.seek(offset);

    // C# Sampler.Load (Wii U):
    //   TexSampler = new TexSampler(loader.ReadUInt32s(3))    // 3 x u32
    //   handle = u32
    //   Name   = loader.LoadString()
    //   idx    = u8
    //   padding = 3 bytes

    let w0 = reader.read_u32()?;
    let w1 = reader.read_u32()?;
    let w2 = reader.read_u32()?;
    let _handle = reader.read_u32()?;
    let name = read_string(reader)?;
    let _idx = reader.read_u8()?;
    reader.skip(3); // padding

    Ok(Sampler {
        name,
        gx2_sampler_data: [w0, w1, w2],
    })
}

// ---------------------------------------------------------------------------
// ShaderParam
// ---------------------------------------------------------------------------

fn parse_shader_param(
    reader: &mut BigEndianReader,
    offset: usize,
    version: u32,
) -> Result<ShaderParam> {
    reader.seek(offset);

    // C# ShaderParam.Load (Wii U):
    //   Type       = ReadEnum<ShaderParamType>(true)   // u8 stored as enum
    //   sizData    = u8
    //   DataOffset = u16
    //   offset     = i32 (uniform variable offset)
    //   if version >= 0x03040000:
    //     callbackPointer = u32
    //     DependedIndex   = u16
    //     DependIndex     = u16
    //   else if version >= 0x03030000 && version < 0x03040000:
    //     callbackPointer = u32
    //     DependedIndex   = u16
    //     DependIndex     = u16
    //     FMATOffset      = u32
    //   Name = loader.LoadString()

    // Note: ReadEnum<ShaderParamType>(true) reads as the underlying type.
    // ShaderParamType is a byte enum, but ReadEnum with strict=true reads a u32 for enums.
    // Actually, looking more carefully at the C# code: ReadEnum with strict=true reads based
    // on the size of the enum's underlying type. ShaderParamType is byte.
    // But the Save code writes it as: saver.Write(Type, true) which for Wii U writes as a u8,
    // then immediately writes (byte)DataSize.
    // So it's: u8 Type, u8 sizData, u16 DataOffset, ...

    let param_type = reader.read_u8()?;
    let _siz_data = reader.read_u8()?;
    let data_offset = reader.read_u16()?;
    let _uniform_offset = reader.read_i32()?;

    let (callback_pointer, depend_index, depend_count);
    if version >= 0x03040000 {
        callback_pointer = reader.read_u32()?;
        let depended_index = reader.read_u16()? as i16;
        let dep_index = reader.read_u16()? as i16;
        depend_index = depended_index;
        depend_count = dep_index;
    } else if version >= 0x03030000 {
        callback_pointer = reader.read_u32()?;
        let depended_index = reader.read_u16()? as i16;
        let dep_index = reader.read_u16()? as i16;
        let _fmat_offset = reader.read_u32()?;
        depend_index = depended_index;
        depend_count = dep_index;
    } else {
        callback_pointer = 0;
        depend_index = 0;
        depend_count = 0;
    }

    let name = read_string(reader)?;

    Ok(ShaderParam {
        name,
        param_type,
        data_offset,
        callback_pointer,
        depend_index,
        depend_count,
    })
}

// ---------------------------------------------------------------------------
// FVTX (Vertex Buffer) - inline from list
// ---------------------------------------------------------------------------

fn parse_vertex_buffer_inline(
    reader: &mut BigEndianReader,
    _version: u32,
) -> Result<VertexBuffer> {
    let magic = reader.read_magic()?;
    if magic != *b"FVTX" {
        return Err(BfresError::InvalidMagic(magic));
    }

    // C# VertexBuffer.Load (Wii U):
    //   numVertexAttrib = u8
    //   numBuffer       = u8
    //   idx             = u16
    //   VertexCount     = u32
    //   VertexSkinCount = u8
    //   padding         = 3 bytes
    //   ofsVertexAttribList = loader.ReadOffset()  (skip, use dict)
    //   Attributes      = loader.LoadDict<VertexAttrib>()
    //   Buffers         = loader.LoadList<Buffer>(numBuffer)
    //   userPointer     = u32

    let num_vertex_attrib = reader.read_u8()?;
    let num_buffer = reader.read_u8()?;
    let index = reader.read_u16()?;
    let vertex_count = reader.read_u32()?;
    let vertex_skin_count = reader.read_u8()?;
    reader.skip(3); // padding

    let _attrib_list_offset = read_offset(reader)?;
    let attrib_dict_offset = read_offset(reader)?;
    let buffer_list_offset = read_offset(reader)?;
    let _user_pointer = reader.read_u32()?;

    // Save position after reading the fixed-size header.
    // This is where the next FVTX in the list starts.
    let next_fvtx_pos = reader.pos();

    // Parse vertex attributes from dictionary
    let mut attributes = Vec::new();
    if attrib_dict_offset > 0 && num_vertex_attrib > 0 {
        let attrib_entries = parse_dict(reader, attrib_dict_offset)?;
        for entry in &attrib_entries {
            let attr = parse_vertex_attrib(reader, entry.data_offset)?;
            attributes.push(attr);
        }
    }

    // Parse buffers from list
    let mut buffers = Vec::new();
    if buffer_list_offset > 0 && num_buffer > 0 {
        reader.seek(buffer_list_offset);
        for _ in 0..num_buffer {
            let buf = parse_buffer_inline(reader)?;
            buffers.push(buf);
        }
    }

    // Byte-swap vertex buffer data based on attribute formats
    byteswap_vertex_buffers(&mut buffers, &attributes);

    // Restore position so the next FVTX in the list can be read
    reader.seek(next_fvtx_pos);

    Ok(VertexBuffer {
        index,
        vertex_count,
        vertex_skin_count,
        attributes,
        buffers,
    })
}

// ---------------------------------------------------------------------------
// VertexAttrib
// ---------------------------------------------------------------------------

fn parse_vertex_attrib(reader: &mut BigEndianReader, offset: usize) -> Result<VertexAttribute> {
    reader.seek(offset);

    // C# VertexAttrib.Load (Wii U):
    //   Name       = loader.LoadString()
    //   BufferIndex = u8
    //   padding    = 1 byte
    //   Offset     = u16
    //   Format     = ReadEnum<GX2AttribFormat>(true)  // u32

    let name = read_string(reader)?;
    let buffer_index = reader.read_u8()?;
    reader.skip(1); // padding
    let offset_val = reader.read_u16()?;
    let format = reader.read_u32()?;

    Ok(VertexAttribute {
        name,
        format,
        offset: offset_val,
        buffer_index,
    })
}

// ---------------------------------------------------------------------------
// Buffer (GPU buffer, inline from list)
// ---------------------------------------------------------------------------

fn parse_buffer_inline(reader: &mut BigEndianReader) -> Result<BufferData> {
    // C# Buffer.Load:
    //   dataPointer    = u32
    //   size           = u32
    //   handle         = u32
    //   Stride         = u16
    //   numBuffering   = u16
    //   contextPointer = u32
    //   Data = loader.LoadCustom(...) -> byte[numBuffering][size]

    let _data_pointer = reader.read_u32()?;
    let size = reader.read_u32()?;
    let _handle = reader.read_u32()?;
    let stride = reader.read_u16()?;
    let _num_buffering = reader.read_u16()?.max(1);
    let _context_pointer = reader.read_u32()?;

    let data_offset = read_offset(reader)?;

    let data = if data_offset > 0 && size > 0 {
        let saved_pos = reader.pos();
        reader.seek(data_offset);
        // Read first buffering only
        let raw = reader.read_bytes(size as usize)?.to_vec();
        reader.seek(saved_pos);
        raw
    } else {
        Vec::new()
    };

    Ok(BufferData { stride, data })
}

// ---------------------------------------------------------------------------
// UserData
// ---------------------------------------------------------------------------

fn parse_user_data_dict(
    reader: &mut BigEndianReader,
    dict_offset: usize,
) -> Result<Vec<UserData>> {
    let entries = parse_dict(reader, dict_offset)?;
    let mut result = Vec::with_capacity(entries.len());

    for entry in &entries {
        reader.seek(entry.data_offset);
        let ud = parse_user_data_inline(reader)?;
        result.push(ud);
    }

    Ok(result)
}

fn parse_user_data_inline(reader: &mut BigEndianReader) -> Result<UserData> {
    // C# UserData.Load (Wii U):
    //   Name   = loader.LoadString()
    //   count  = u16
    //   Type   = ReadEnum<UserDataType>(true) // u8
    //   padding = 1 byte
    //   then data inline based on type

    let name = read_string(reader)?;
    let count = reader.read_u16()?;
    let ud_type = reader.read_u8()?;
    reader.skip(1); // padding

    let value = match ud_type {
        0 => {
            // Int32
            let mut vals = Vec::with_capacity(count as usize);
            for _ in 0..count {
                vals.push(reader.read_i32()?);
            }
            UserDataValue::Int32(vals)
        }
        1 => {
            // Single (f32)
            let mut vals = Vec::with_capacity(count as usize);
            for _ in 0..count {
                vals.push(reader.read_f32()?);
            }
            UserDataValue::Float(vals)
        }
        2 | 3 => {
            // String / WString - stored as string offsets
            let mut vals = Vec::with_capacity(count as usize);
            for _ in 0..count {
                let s = read_string(reader)?;
                vals.push(s);
            }
            UserDataValue::String(vals)
        }
        4 => {
            // Byte
            let raw = reader.read_bytes(count as usize)?.to_vec();
            UserDataValue::Bytes(raw)
        }
        _ => UserDataValue::Bytes(Vec::new()),
    };

    Ok(UserData { name, value })
}

// ---------------------------------------------------------------------------
// Byte-swapping helpers
// ---------------------------------------------------------------------------

/// Determine the byte-swap unit size for a GX2 attribute format.
/// Returns the number of bytes per component that need byte-swapping.
fn attrib_format_swap_size(format: u32) -> usize {
    // The low byte of the GX2AttribFormat determines the component encoding:
    //   0x00 => 8-bit per component  (no swap)
    //   0x01 => 4_4 packed           (no swap)
    //   0x02 => 16-bit per component (swap 2)
    //   0x03 => 16-bit               (swap 2)
    //   0x04 => 8-bit x2             (no swap)
    //   0x05 => 32-bit per component (swap 4)
    //   0x06 => 32-bit               (swap 4)
    //   0x07 => 16-bit               (swap 2)
    //   0x08 => 16-bit               (swap 2)
    //   0x09 => 10_11_11 special     (swap 4, treated as one u32)
    //   0x0A => 8-bit per component  (no swap)
    //   0x0B => 10_10_10_2           (swap 4)
    //   0x0C => 32-bit               (swap 4)
    //   0x0D => 32-bit               (swap 4)
    //   0x0E => 16-bit               (swap 2)
    //   0x0F => 16-bit               (swap 2)
    //   0x10 => 32-bit               (swap 4)
    //   0x11 => 32-bit               (swap 4)
    //   0x12 => 32-bit               (swap 4)
    //   0x13 => 32-bit               (swap 4)
    match format & 0xFF {
        0x00 | 0x01 | 0x04 | 0x0A => 1,
        0x02 | 0x03 | 0x07 | 0x08 | 0x0E | 0x0F => 2,
        0x05 | 0x06 | 0x09 | 0x0B | 0x0C | 0x0D | 0x10 | 0x11 | 0x12 | 0x13 => 4,
        _ => 1, // Unknown, assume no swap
    }
}

/// Byte-swap vertex buffer data in-place. For each buffer, walk through the data
/// swapping each component based on the attributes that reference that buffer.
fn byteswap_vertex_buffers(buffers: &mut [BufferData], attributes: &[VertexAttribute]) {
    for (buf_idx, buf) in buffers.iter_mut().enumerate() {
        if buf.data.is_empty() || buf.stride == 0 {
            continue;
        }

        // Collect all attributes that reference this buffer
        let attrs_for_buf: Vec<&VertexAttribute> = attributes
            .iter()
            .filter(|a| a.buffer_index as usize == buf_idx)
            .collect();

        if attrs_for_buf.is_empty() {
            continue;
        }

        let stride = buf.stride as usize;
        let vertex_count = buf.data.len() / stride;

        for attr in &attrs_for_buf {
            let swap_size = attrib_format_swap_size(attr.format);
            if swap_size <= 1 {
                continue;
            }

            // Determine how many bytes this attribute occupies per vertex
            let attr_byte_size = attrib_format_byte_size(attr.format);

            for v in 0..vertex_count {
                let base = v * stride + attr.offset as usize;
                let end = base + attr_byte_size;
                if end > buf.data.len() {
                    break;
                }

                // Swap each component
                let mut pos = base;
                while pos + swap_size <= end {
                    match swap_size {
                        2 => buf.data.swap(pos, pos + 1),
                        4 => {
                            buf.data.swap(pos, pos + 3);
                            buf.data.swap(pos + 1, pos + 2);
                        }
                        _ => {}
                    }
                    pos += swap_size;
                }
            }
        }
    }
}

/// Return the total byte size of a GX2 attribute format.
fn attrib_format_byte_size(format: u32) -> usize {
    // Common GX2AttribFormat values and their sizes:
    match format {
        // 8-bit formats (1 component)
        0x00000000 | 0x00000100 | 0x00000200 | 0x00000300 => 1,
        // 4_4 (1 byte)
        0x00000001 => 1,
        // 8_8 formats (2 bytes)
        0x00000004 | 0x00000104 | 0x00000204 | 0x00000304 => 2,
        // 16-bit formats (1 component, 2 bytes)
        0x00000002 | 0x00000102 | 0x00000202 | 0x00000302 | 0x00000802 | 0x00000A02 => 2,
        // 16_16 formats (4 bytes)
        0x00000007 | 0x00000107 | 0x00000207 | 0x00000307 | 0x00000807 | 0x00000A07 => 4,
        // 8_8_8_8 formats (4 bytes)
        0x0000000A | 0x0000010A | 0x0000020A | 0x0000030A => 4,
        // 10_10_10_2 formats (4 bytes)
        0x0000000B | 0x0000010B | 0x0000020B | 0x0000030B | 0x0000080B | 0x00000A0B => 4,
        // 10_11_11 (4 bytes)
        0x00000009 | 0x00000809 => 4,
        // 16_16_16_16 formats (8 bytes)
        0x0000000E | 0x0000010E | 0x0000020E | 0x0000030E | 0x0000050E | 0x0000080E | 0x00000A0E => 8,
        // 32 formats (4 bytes)
        0x00000005 | 0x00000105 | 0x00000205 | 0x00000305 | 0x00000505 => 4,
        // 32 Single (4 bytes)
        0x00000006 | 0x00000106 | 0x00000206 | 0x00000306 | 0x00000506 => 4,
        // 32_32 formats (8 bytes)
        0x00000010 | 0x00000110 | 0x00000210 | 0x00000310 | 0x00000510 => 8,
        // 32_32_32 formats (12 bytes)
        0x00000011 | 0x00000111 | 0x00000211 | 0x00000311 | 0x00000511 => 12,
        // 32_32_32_32 formats (16 bytes)
        0x00000012 | 0x00000112 | 0x00000212 | 0x00000312 | 0x00000512 => 16,
        _ => {
            // Fallback: determine from low byte (format encoding) and component count
            let swap = attrib_format_swap_size(format);
            // Estimate component count from the format encoding
            // This is a rough heuristic; exact values above cover all real cases
            let low = format & 0xFF;
            let components = match low {
                0x00 | 0x02 | 0x03 | 0x05 | 0x06 => 1,
                0x01 | 0x04 | 0x07 | 0x08 => 2,
                0x09 | 0x11 => 3,
                0x0A | 0x0B | 0x0C | 0x0D | 0x12 => 4,
                0x0E | 0x0F | 0x10 | 0x13 => 4,
                _ => 1,
            };
            swap * components
        }
    }
}

/// Byte-swap index data in-place based on the GX2 index format.
fn byteswap_index_data(data: &mut [u8], index_format: u32) {
    // GX2IndexFormat:
    //   UInt16 = 0  (big-endian, swap 2)
    //   UInt32 = 1  (big-endian, swap 4)
    //   UInt16LE = 4 (little-endian, no swap needed)
    //   UInt32LE = 9 (little-endian, no swap needed)
    let swap_size = match index_format {
        0 => 2, // UInt16 (big-endian)
        4 => 0, // UInt16 (little-endian) - already LE
        1 => 4, // UInt32 (big-endian)
        9 => 0, // UInt32 (little-endian) - already LE
        _ => 2, // Default to u16 swap
    };

    if swap_size == 0 {
        return;
    }

    let mut pos = 0;
    while pos + swap_size <= data.len() {
        match swap_size {
            2 => data.swap(pos, pos + 1),
            4 => {
                data.swap(pos, pos + 3);
                data.swap(pos + 1, pos + 2);
            }
            _ => {}
        }
        pos += swap_size;
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wiiu::header;

    #[test]
    fn parse_animal_boar_big_model() {
        let data = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/Animal_Boar_Big.wiiu.bfres"
        ))
        .expect("fixture file should exist");

        let mut reader = BigEndianReader::new(&data);
        let hdr = header::parse_header(&mut reader).expect("header should parse");

        assert!(hdr.model_dict_offset > 0, "should have model dict");

        let model_entries =
            parse_dict(&mut reader, hdr.model_dict_offset as usize).expect("model dict");

        assert!(!model_entries.is_empty(), "should have at least one model entry");

        let model = parse_model(&mut reader, model_entries[0].data_offset, hdr.version)
            .expect("model should parse");

        assert_eq!(model.name, "Boar_Big");
        assert_eq!(model.skeleton.bones.len(), 31);
        assert_eq!(model.shapes.len(), 2);
        assert_eq!(model.materials.len(), 2);
        assert_eq!(model.vertex_buffers.len(), 2);

        // Verify materials have shader assigns
        for mat in &model.materials {
            assert!(mat.shader_assign.is_some(), "material should have shader assign");
            assert!(!mat.shader_params.is_empty(), "material should have shader params");
        }

        // Verify vertex buffers have data
        for vb in &model.vertex_buffers {
            let total: usize = vb.buffers.iter().map(|b| b.data.len()).sum();
            assert!(total > 0, "vertex buffer {} should have data", vb.index);
        }
    }
}
