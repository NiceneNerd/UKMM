//! Wii U BFRES parser.
//!
//! Parses big-endian BFRES v0.3 files into the internal data model.

mod header;
mod dict;
pub mod model;
pub mod texture;

use crate::binary::BigEndianReader;
use crate::error::Result;
use crate::model::*;

/// Parse a Wii U BFRES file into the internal data model.
pub fn parse(data: &[u8]) -> Result<BfresFile> {
    let mut reader = BigEndianReader::new(data);
    let hdr = header::parse_header(&mut reader)?;
    log::debug!("Parsed BFRES header: name={}, version={:#010x}", hdr.name, hdr.version);

    let mut bfres = BfresFile {
        name: hdr.name,
        version: hdr.version_bytes,
        alignment: hdr.alignment,
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

    // Parse textures
    if hdr.texture_dict_offset > 0 {
        let entries = dict::parse_dict(&mut reader, hdr.texture_dict_offset as usize)?;
        log::debug!("Found {} textures", entries.len());
        for entry in &entries {
            match texture::parse_texture(&mut reader, entry.data_offset) {
                Ok(tex) => {
                    log::debug!("  Texture: {} ({}x{}, fmt={:#x})",
                        tex.name, tex.width, tex.height, tex.format);
                    bfres.textures.push(tex);
                }
                Err(e) => {
                    log::warn!("  Failed to parse texture '{}': {}", entry.name, e);
                }
            }
        }
    }

    // Parse models
    if hdr.model_dict_offset > 0 {
        let entries = dict::parse_dict(&mut reader, hdr.model_dict_offset as usize)?;
        log::debug!("Found {} models", entries.len());
        for entry in &entries {
            log::debug!("  Model: {} at offset {:#x}", entry.name, entry.data_offset);
            match model::parse_model(&mut reader, entry.data_offset, hdr.version) {
                Ok(mdl) => {
                    log::debug!(
                        "  Parsed model: {} (shapes={}, mats={}, vbufs={})",
                        mdl.name,
                        mdl.shapes.len(),
                        mdl.materials.len(),
                        mdl.vertex_buffers.len()
                    );
                    bfres.models.push(mdl);
                }
                Err(e) => {
                    log::warn!("  Failed to parse model '{}': {}", entry.name, e);
                }
            }
        }
    }

    // Parse external files
    if hdr.external_file_dict_offset > 0 {
        let entries = dict::parse_dict(&mut reader, hdr.external_file_dict_offset as usize)?;
        log::debug!("Found {} external files", entries.len());
        for entry in &entries {
            // External files are stored as raw data blocks.
            // The dict entry's data_offset points to an offset/size pair.
            reader.seek(entry.data_offset);
            if let (Ok(data_offset), Ok(data_size)) = (
                header::read_rel_offset(&mut reader),
                reader.read_u32(),
            ) {
                if data_offset > 0 && data_size > 0 {
                    reader.seek(data_offset as usize);
                    if let Ok(bytes) = reader.read_bytes(data_size as usize) {
                        bfres.external_files.push(ExternalFile {
                            name: entry.name.clone(),
                            data: bytes.to_vec(),
                        });
                        log::debug!("  External file: {} ({} bytes)", entry.name, data_size);
                    }
                }
            }
        }
    }

    Ok(bfres)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_model_bfres() {
        let data = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/Animal_Boar_Big.wiiu.bfres"
        ))
        .expect("fixture file should exist");

        let bfres = parse(&data).expect("should parse");
        assert_eq!(bfres.name, "Animal_Boar_Big");
        assert_eq!(bfres.models.len(), 1, "should have 1 model");
        assert_eq!(bfres.textures.len(), 0, "model file should have 0 textures");
        let mdl = &bfres.models[0];
        assert!(!mdl.shapes.is_empty(), "model should have shapes");
        assert!(!mdl.materials.is_empty(), "model should have materials");
        assert!(!mdl.vertex_buffers.is_empty(), "model should have vertex buffers");
        assert!(!mdl.skeleton.bones.is_empty(), "model should have bones");
    }

    #[test]
    fn parse_texture_bfres() {
        let data = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/Animal_Boar_Big.Tex1.wiiu.bfres"
        ))
        .expect("fixture file should exist");

        let bfres = parse(&data).expect("should parse");
        assert_eq!(bfres.textures.len(), 6, "Tex1 should have 6 textures");
        for tex in &bfres.textures {
            assert!(tex.width > 0 && tex.height > 0, "texture dims should be nonzero");
            assert!(!tex.surface_data.is_empty(), "texture data should not be empty");
        }
        eprintln!("Parsed Tex1: {} textures", bfres.textures.len());
    }
}
