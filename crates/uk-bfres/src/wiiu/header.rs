//! Wii U BFRES header parser.

use crate::binary::BigEndianReader;
use crate::error::{BfresError, Result};

/// Parsed Wii U BFRES header.
#[derive(Debug)]
pub struct BfresHeader {
    pub version: u32,
    pub version_bytes: (u8, u8, u8, u8),
    pub file_size: u32,
    pub alignment: u32,
    pub name: String,
    pub string_table_size: u32,
    pub string_table_offset: u32,

    // Sub-file dict offsets (absolute). Each section is a single relative offset
    // read by LoadDict<T>() in the C# library.
    pub model_dict_offset: u32,
    pub texture_dict_offset: u32,
    pub skeleton_anim_dict_offset: u32,
    pub shader_param_anim_dict_offset: u32,
    pub color_anim_dict_offset: u32,
    pub tex_srt_anim_dict_offset: u32,
    pub tex_pattern_anim_dict_offset: u32,
    pub bone_vis_anim_dict_offset: u32,
    pub mat_vis_anim_dict_offset: u32,
    pub shape_anim_dict_offset: u32,
    pub scene_anim_dict_offset: u32,
    pub external_file_dict_offset: u32,

    // Sub-file counts
    pub model_count: u16,
    pub texture_count: u16,
    pub skeleton_anim_count: u16,
    pub shader_param_anim_count: u16,
    pub color_anim_count: u16,
    pub tex_srt_anim_count: u16,
    pub tex_pattern_anim_count: u16,
    pub bone_vis_anim_count: u16,
    pub mat_vis_anim_count: u16,
    pub shape_anim_count: u16,
    pub scene_anim_count: u16,
    pub external_file_count: u16,
}

/// Read a relative offset: reads a u32, then converts to absolute position.
/// Returns 0 if the raw offset is 0.
pub(crate) fn read_rel_offset(reader: &mut BigEndianReader) -> Result<u32> {
    let field_pos = reader.pos();
    let rel = reader.read_u32()?;
    if rel == 0 {
        Ok(0)
    } else {
        Ok((field_pos as u32).wrapping_add(rel))
    }
}

/// Parse the Wii U BFRES header starting at offset 0.
///
/// Exact field order from BfresLibrary WiiU/ResFileParser.cs Load():
///   CheckSignature("FRES")          // 4 bytes
///   ReadUInt32()                     // version
///   ReadByteOrder()                  // 2 bytes BOM
///   ReadUInt16()                     // header size
///   ReadUInt32()                     // file size
///   ReadUInt32()                     // alignment
///   LoadString()                     // 1 relative offset (4 bytes)
///   ReadUInt32()                     // string pool size
///   ReadOffset()                     // string pool offset (4 bytes)
///   LoadDict<Model>()               // 1 offset
///   LoadDict<Texture>()             // 1 offset
///   LoadDict<SkeletalAnim>()        // 1 offset
///   LoadDict<MaterialAnim>()        // shader param anims (1 offset)
///   LoadDict<MaterialAnim>()        // color anims (1 offset)
///   LoadDict<MaterialAnim>()        // tex srt anims (1 offset)
///   LoadDict<MaterialAnim>()        // tex pattern anims (1 offset)
///   LoadDict<VisibilityAnim>()      // bone vis anims (1 offset)
///   LoadDict<VisibilityAnim>()      // mat vis anims (read but ignored) (1 offset)
///   LoadDict<ShapeAnim>()           // shape anims (1 offset)
///   (version >= 0x02040000):
///     LoadDict<SceneAnim>()         // 1 offset
///     LoadDict<ExternalFile>()      // 1 offset
///     12x ReadUInt16()              // counts
///     ReadUInt32()                  // user pointer
pub fn parse_header(reader: &mut BigEndianReader) -> Result<BfresHeader> {
    let magic = reader.read_magic()?;
    if magic != *b"FRES" {
        return Err(BfresError::InvalidMagic(magic));
    }

    let version = reader.read_u32()?;
    let version_bytes = (
        ((version >> 24) & 0xFF) as u8,
        ((version >> 16) & 0xFF) as u8,
        ((version >> 8) & 0xFF) as u8,
        (version & 0xFF) as u8,
    );

    let bom = reader.read_u16()?;
    if bom != 0xFEFF {
        return Err(BfresError::NotWiiU);
    }

    let _header_size = reader.read_u16()?;
    let file_size = reader.read_u32()?;
    let alignment = reader.read_u32()?;

    // Name: relative offset pointing to the string text (after 4-byte length prefix)
    let name_offset = read_rel_offset(reader)?;
    let name = if name_offset > 0 {
        reader.read_string_at(name_offset as usize).unwrap_or_default()
    } else {
        String::new()
    };

    let string_table_size = reader.read_u32()?;
    let string_table_offset = read_rel_offset(reader)?;

    // 10 sub-file dict offsets (each is a single relative offset)
    let model_dict_offset = read_rel_offset(reader)?;
    let texture_dict_offset = read_rel_offset(reader)?;
    let skeleton_anim_dict_offset = read_rel_offset(reader)?;
    let shader_param_anim_dict_offset = read_rel_offset(reader)?;
    let color_anim_dict_offset = read_rel_offset(reader)?;
    let tex_srt_anim_dict_offset = read_rel_offset(reader)?;
    let tex_pattern_anim_dict_offset = read_rel_offset(reader)?;
    let bone_vis_anim_dict_offset = read_rel_offset(reader)?;
    let mat_vis_anim_dict_offset = read_rel_offset(reader)?;
    let shape_anim_dict_offset = read_rel_offset(reader)?;

    // For version >= 0x02040000 (which includes all BotW files at v0.04050003)
    let (scene_anim_dict_offset, external_file_dict_offset);
    let (model_count, texture_count, skeleton_anim_count, shader_param_anim_count,
         color_anim_count, tex_srt_anim_count, tex_pattern_anim_count,
         bone_vis_anim_count, mat_vis_anim_count, shape_anim_count,
         scene_anim_count, external_file_count);

    if version >= 0x02040000 {
        scene_anim_dict_offset = read_rel_offset(reader)?;
        external_file_dict_offset = read_rel_offset(reader)?;

        model_count = reader.read_u16()?;
        texture_count = reader.read_u16()?;
        skeleton_anim_count = reader.read_u16()?;
        shader_param_anim_count = reader.read_u16()?;
        color_anim_count = reader.read_u16()?;
        tex_srt_anim_count = reader.read_u16()?;
        tex_pattern_anim_count = reader.read_u16()?;
        bone_vis_anim_count = reader.read_u16()?;
        mat_vis_anim_count = reader.read_u16()?;
        shape_anim_count = reader.read_u16()?;
        scene_anim_count = reader.read_u16()?;
        external_file_count = reader.read_u16()?;

        let _user_pointer = reader.read_u32()?;
    } else {
        let _user_pointer = reader.read_u32()?;
        let _user_pointer2 = reader.read_u32()?;
        scene_anim_dict_offset = read_rel_offset(reader)?;
        external_file_dict_offset = read_rel_offset(reader)?;
        // Old versions don't have explicit counts
        model_count = 0;
        texture_count = 0;
        skeleton_anim_count = 0;
        shader_param_anim_count = 0;
        color_anim_count = 0;
        tex_srt_anim_count = 0;
        tex_pattern_anim_count = 0;
        bone_vis_anim_count = 0;
        mat_vis_anim_count = 0;
        shape_anim_count = 0;
        scene_anim_count = 0;
        external_file_count = 0;
    }

    Ok(BfresHeader {
        version,
        version_bytes,
        file_size,
        alignment,
        name,
        string_table_size,
        string_table_offset,
        model_dict_offset,
        texture_dict_offset,
        skeleton_anim_dict_offset,
        shader_param_anim_dict_offset,
        color_anim_dict_offset,
        tex_srt_anim_dict_offset,
        tex_pattern_anim_dict_offset,
        bone_vis_anim_dict_offset,
        mat_vis_anim_dict_offset,
        shape_anim_dict_offset,
        scene_anim_dict_offset,
        external_file_dict_offset,
        model_count,
        texture_count,
        skeleton_anim_count,
        shader_param_anim_count,
        color_anim_count,
        tex_srt_anim_count,
        tex_pattern_anim_count,
        bone_vis_anim_count,
        mat_vis_anim_count,
        shape_anim_count,
        scene_anim_count,
        external_file_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_animal_boar_big_header() {
        let data = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/Animal_Boar_Big.wiiu.bfres"
        ))
        .expect("fixture file should exist");

        let mut reader = BigEndianReader::new(&data);
        let header = parse_header(&mut reader).expect("header should parse");

        assert_eq!(header.file_size as usize, data.len(), "file size should match");
        assert!(header.alignment > 0, "alignment should be nonzero");
        assert!(!header.name.is_empty(), "name should not be empty");
        assert_eq!(header.name, "Animal_Boar_Big");
        assert!(header.model_count > 0, "should have at least 1 model");

        eprintln!("Header: name={}, version={:#010x}, alignment={:#x}, file_size={:#x}",
            header.name, header.version, header.alignment, header.file_size);
        eprintln!("Counts: models={}, textures={}, skel_anims={}, ext_files={}",
            header.model_count, header.texture_count,
            header.skeleton_anim_count, header.external_file_count);
        eprintln!("Dict offsets: model={:#x}, texture={:#x}, ext_file={:#x}",
            header.model_dict_offset, header.texture_dict_offset, header.external_file_dict_offset);
    }

    #[test]
    fn parse_armor_head_header() {
        let data = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/Armor_740_Head.15.wiiu.bfres"
        ))
        .expect("fixture file should exist");

        let mut reader = BigEndianReader::new(&data);
        let header = parse_header(&mut reader).expect("header should parse");

        assert_eq!(header.file_size as usize, data.len(), "file size should match");
        assert!(!header.name.is_empty(), "name should not be empty");
        eprintln!("Header: name={}, version={:#010x}, counts: models={}, textures={}",
            header.name, header.version, header.model_count, header.texture_count);
    }

    #[test]
    fn parse_texture_file_header() {
        let data = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/Animal_Boar_Big.Tex1.wiiu.bfres"
        ))
        .expect("fixture file should exist");

        let mut reader = BigEndianReader::new(&data);
        let header = parse_header(&mut reader).expect("header should parse");

        assert_eq!(header.file_size as usize, data.len(), "file size should match");
        assert!(header.texture_count > 0, "Tex1 file should have textures");
        eprintln!("Tex1 Header: name={}, textures={}", header.name, header.texture_count);
    }
}
