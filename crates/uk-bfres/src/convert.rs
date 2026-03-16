//! BFRES conversion orchestration and material mapping.
//!
//! Handles converting Wii U BFRES internal representation to Switch format,
//! including BotW-specific material RenderState → RenderInfo conversion
//! and animation suffix renaming.

use crate::model::*;

/// Convert a Wii U BfresFile to Switch format in-place.
///
/// This transforms the internal data model to match Switch BFRES expectations:
/// 1. Convert materials: RenderState → RenderInfo key-value pairs
/// 2. Rename material animations with Switch suffixes
/// 3. Combine separate Wii U animation dicts into Switch's unified material anim dict
/// 4. Byte-swap vertex/index buffer data (already done during parse)
pub fn convert_to_switch(bfres: &mut BfresFile) {
    // Convert materials in all models
    for model in &mut bfres.models {
        for material in &mut model.materials {
            convert_material_botw(material);
        }
    }

    // Combine Wii U animation categories into Switch's unified material anims
    // with name suffixes
    combine_material_anims(bfres);

    // Convert textures: build BNTX container and add as external file
    if !bfres.textures.is_empty() {
        convert_textures(bfres);
    }

    // Update version to Switch format
    bfres.version = (0, 5, 0, 3);
    bfres.alignment = 0x0C; // log2(4096)
}

/// Convert Wii U textures to Switch format and embed as a BNTX external file.
///
/// Builds a BNTX container from the Wii U textures (deswizzle GX2 → linear →
/// re-swizzle TegraX1) and adds it as an external file entry in the BFRES.
/// The textures are then cleared from the BfresFile since Switch BFRES stores
/// them in the external BNTX, not inline.
fn convert_textures(bfres: &mut BfresFile) {
    use crate::switch::bntx;

    let bntx_data = bntx::build_bntx(&bfres.textures);
    if bntx_data.is_empty() {
        return;
    }

    // The BNTX external file name is the BFRES name + ".bntx"
    let bntx_name = format!("{}.bntx", bfres.name);

    bfres.external_files.push(ExternalFile {
        name: bntx_name,
        data: bntx_data,
    });

    // Clear textures from the BFRES since they're now in the BNTX
    bfres.textures.clear();
}

/// Convert a BotW Wii U material to Switch format.
///
/// Converts the GX2 RenderState to gsys_* RenderInfo key-value string pairs,
/// matching BfresLibrary's MaterialConverterBOTW.ConvertToSwitchMaterial().
fn convert_material_botw(material: &mut Material) {
    let state = match &material.render_state {
        Some(s) => s,
        None => return,
    };

    // gsys_render_state_mode
    let mode = match state.flags_mode() {
        0 => "opaque",
        1 => "translucent",
        2 => "mask",
        _ => "custom",
    };
    push_string_info(&mut material.render_infos, "gsys_render_state_mode", mode);

    // gsys_render_state_display_face
    let display_face = match (state.cull_back(), state.cull_front()) {
        (true, true) => "none",
        (true, false) => "front",
        (false, true) => "back",
        (false, false) => "both",
    };
    push_string_info(&mut material.render_infos, "gsys_render_state_display_face", display_face);

    // gsys_render_state_blend_mode
    let blend_mode = match state.flags_blend_mode() {
        1 => "color",
        2 => "logic",
        _ => "none",
    };
    push_string_info(&mut material.render_infos, "gsys_render_state_blend_mode", blend_mode);

    // Depth test
    push_string_info(&mut material.render_infos, "gsys_depth_test_enable",
        bool_str(state.depth_test_enabled()));
    push_string_info(&mut material.render_infos, "gsys_depth_test_write",
        bool_str(state.depth_write_enabled()));
    push_string_info(&mut material.render_infos, "gsys_depth_test_func",
        compare_func_str(state.depth_func()));

    // Color blend
    push_string_info(&mut material.render_infos, "gsys_color_blend_rgb_src_func",
        blend_func_str(state.color_src_blend()));
    push_string_info(&mut material.render_infos, "gsys_color_blend_rgb_dst_func",
        blend_func_str(state.color_dst_blend()));
    push_string_info(&mut material.render_infos, "gsys_color_blend_rgb_op",
        blend_combine_str(state.color_combine()));

    // Alpha blend
    push_string_info(&mut material.render_infos, "gsys_color_blend_alpha_src_func",
        blend_func_str(state.alpha_src_blend()));
    push_string_info(&mut material.render_infos, "gsys_color_blend_alpha_dst_func",
        blend_func_str(state.alpha_dst_blend()));
    push_string_info(&mut material.render_infos, "gsys_color_blend_alpha_op",
        blend_combine_str(state.alpha_combine()));

    // Alpha test
    push_string_info(&mut material.render_infos, "gsys_alpha_test_enable",
        bool_str(state.alpha_test_enabled()));
    push_string_info(&mut material.render_infos, "gsys_alpha_test_func",
        compare_func_str(state.alpha_func()));

    // Blend constant color (always [0,0,0,0])
    material.render_infos.push(RenderInfo {
        name: "gsys_color_blend_const_color".to_string(),
        value: RenderInfoValue::Float(vec![0.0, 0.0, 0.0, 0.0]),
    });

    // Alpha test reference value
    material.render_infos.push(RenderInfo {
        name: "gsys_alpha_test_value".to_string(),
        value: RenderInfoValue::Float(vec![state.alpha_ref_value]),
    });

    // Update shader param for alpha ref if it exists
    // (The C# does: material.ShaderParams["gsys_alpha_test_ref_value"].DataValue = alphaRef)
    // We handle this at write time since we'd need to modify the shader param data buffer.

    // Clear the RenderState — Switch materials don't have one
    material.render_state = None;
}

fn push_string_info(infos: &mut Vec<RenderInfo>, name: &str, value: &str) {
    infos.push(RenderInfo {
        name: name.to_string(),
        value: RenderInfoValue::String(vec![value.to_string()]),
    });
}

fn bool_str(v: bool) -> &'static str {
    if v { "true" } else { "false" }
}

/// GX2CompareFunction → string mapping (from BfresLibrary MaterialConverterBOTW)
fn compare_func_str(func: u32) -> &'static str {
    match func {
        0 => "never",
        1 => "less",
        2 => "equal",
        3 => "lequal",
        4 => "greater",
        5 => "noequal",
        6 => "gequal",
        7 => "always",
        _ => "always",
    }
}

/// GX2BlendFunction → string mapping
fn blend_func_str(func: u32) -> &'static str {
    match func {
        0 => "zero",
        1 => "one",
        2 => "src_color",
        3 => "one_minus_src_color",
        4 => "src_alpha",
        5 => "one_minus_src_alpha",
        6 => "dst_alpha",
        7 => "one_minus_dst_alpha",
        8 => "dst_color",
        9 => "one_minus_dst_color",
        10 => "src_alpha_saturate",
        13 => "const_color",
        14 => "one_minus_const_color",
        15 => "src1_color",
        16 => "one_minus_src1_color",
        17 => "src1_alpha",
        18 => "one_minus_src1_alpha",
        19 => "const_alpha",
        20 => "one_minus_const_alpha",
        _ => "zero",
    }
}

/// GX2BlendCombine → string mapping
fn blend_combine_str(combine: u32) -> &'static str {
    match combine {
        0 => "add",
        1 => "sub",
        2 => "min",
        3 => "max",
        _ => "add",
    }
}

/// Combine Wii U's separate material animation dictionaries into Switch's
/// unified material animation list with name suffixes.
///
/// Switch BFRES suffix convention:
/// - ShaderParamAnims → "_fsp"
/// - TexSrtAnims → "_fts"
/// - ColorAnims → "_fcl"
/// - TexPatternAnims → "_ftp"
/// - MatVisibilityAnims → "_fvs"
fn combine_material_anims(bfres: &mut BfresFile) {
    let mut combined = Vec::new();

    for anim in bfres.shader_param_anims.drain(..) {
        combined.push(MaterialAnimation {
            name: format!("{}_fsp", anim.name),
            ..anim
        });
    }
    for anim in bfres.tex_srt_anims.drain(..) {
        combined.push(MaterialAnimation {
            name: format!("{}_fts", anim.name),
            ..anim
        });
    }
    for anim in bfres.color_anims.drain(..) {
        combined.push(MaterialAnimation {
            name: format!("{}_fcl", anim.name),
            ..anim
        });
    }
    for anim in bfres.tex_pattern_anims.drain(..) {
        combined.push(MaterialAnimation {
            name: format!("{}_ftp", anim.name),
            ..anim
        });
    }
    for anim in bfres.mat_vis_anims.drain(..) {
        combined.push(MaterialAnimation {
            name: format!("{}_fvs", anim.name.clone()),
            path: anim.path.clone(),
            flags: anim.flags,
            frame_count: anim.frame_count,
            baked_size: anim.baked_size,
            raw_data: anim.raw_data,
        });
    }

    bfres.material_anims = combined;
}

/// GX2 surface format → Switch texture format mapping.
/// Returns the Switch SurfaceFormat enum value.
pub fn map_gx2_to_switch_format(gx2_format: u32) -> u32 {
    match gx2_format {
        // TC_R8_G8_B8_A8 variants
        0x01A => 0x0B02, // TC_R8_G8_B8_A8_SNorm → R8_G8_B8_A8_SNORM
        0x41A => 0x0B01, // TCS_R8_G8_B8_A8_UNorm → R8_G8_B8_A8_UNORM
        0x81A => 0x0B06, // TCS_R8_G8_B8_A8_SRGB → R8_G8_B8_A8_SRGB
        // TCS_R5_G6_B5
        0x408 => 0x0501, // TCS_R5_G6_B5_UNorm → R5_G6_B5_UNORM (actually R5_G6_B5)
        // TC_R5_G5_B5_A1
        0x00A => 0x0601, // TC_R5_G5_B5_A1_UNorm → R5_G5_B5_A1_UNORM
        // TC_A1_B5_G5_R5
        0x00B => 0x0601, // A1_B5_G5_R5_UNORM (mapped to same)
        // TC_R8_G8
        0x007 => 0x0301, // TC_R8_G8_UNorm → R8_G8_UNORM
        // BCn formats (most common in BotW)
        0x031 => 0x1A01, // T_BC1_UNorm → BC1_UNORM
        0x431 => 0x1A01, // T_BC1_UNorm (alt encoding)
        0x831 => 0x1A06, // T_BC1_SRGB → BC1_SRGB
        0x032 => 0x1B01, // T_BC2_UNorm → BC2_UNORM
        0x432 => 0x1B01,
        0x832 => 0x1B06, // T_BC2_SRGB → BC2_SRGB
        0x033 => 0x1C01, // T_BC3_UNorm → BC3_UNORM
        0x433 => 0x1C01,
        0x833 => 0x1C06, // T_BC3_SRGB → BC3_SRGB
        0x034 => 0x1D01, // T_BC4_UNorm → BC4_UNORM
        0x234 => 0x1D02, // T_BC4_SNorm → BC4_SNORM
        0x434 => 0x1D01,
        0x035 => 0x1E01, // T_BC5_UNorm → BC5_UNORM
        0x235 => 0x1E02, // T_BC5_SNorm → BC5_SNORM
        0x435 => 0x1E01,
        // R8 formats
        0x001 => 0x0201, // TC_R8_UNorm → R8_UNORM
        // Default: pass through (caller should handle unknown)
        _ => {
            log::warn!("Unknown GX2 surface format: {:#06x}", gx2_format);
            0x0B01 // Default to R8_G8_B8_A8_UNORM
        }
    }
}

/// GX2 component selector → Switch channel type mapping.
/// GX2CompSel: 0=R, 1=G, 2=B, 3=A, 4=Zero, 5=One
/// Switch ChannelType: 0=Zero, 1=One, 2=Red, 3=Green, 4=Blue, 5=Alpha
pub fn map_channel_selector(gx2_comp_sel: u8) -> u8 {
    match gx2_comp_sel {
        0 => 2, // R → Red
        1 => 3, // G → Green
        2 => 4, // B → Blue
        3 => 5, // A → Alpha
        4 => 0, // Always0 → Zero
        5 => 1, // Always1 → One
        _ => 2, // Default to Red
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_state_conversion() {
        let mut material = Material {
            name: "Mt_Test".to_string(),
            flags: 0,
            index: 0,
            render_infos: Vec::new(),
            render_state: Some(RenderState {
                flags: 0x00, // Opaque, None blend
                polygon_control: 0x00, // no culling
                depth_control: 0x06, // depth test + write enabled
                alpha_control: 0x00, // alpha test disabled
                alpha_ref_value: 0.5,
                blend_control: 0x00050004, // src_alpha, one_minus_src_alpha
                blend_color: [0.0; 4],
            }),
            shader_assign: None,
            shader_params: Vec::new(),
            texture_refs: Vec::new(),
            samplers: Vec::new(),
            user_data: Vec::new(),
            shader_param_data: Vec::new(),
            volatile_flags: Vec::new(),
        };

        convert_material_botw(&mut material);

        assert!(material.render_state.is_none(), "RenderState should be cleared");
        assert!(!material.render_infos.is_empty(), "should have render infos");

        // Check specific render info values
        let find = |name: &str| -> Option<&RenderInfo> {
            material.render_infos.iter().find(|ri| ri.name == name)
        };

        let mode = find("gsys_render_state_mode").expect("should have mode");
        match &mode.value {
            RenderInfoValue::String(v) => assert_eq!(v[0], "opaque"),
            _ => panic!("mode should be string"),
        }

        let face = find("gsys_render_state_display_face").expect("should have display_face");
        match &face.value {
            RenderInfoValue::String(v) => assert_eq!(v[0], "both"),
            _ => panic!("face should be string"),
        }
    }

    #[test]
    fn test_format_mapping() {
        assert_eq!(map_gx2_to_switch_format(0x431), 0x1A01); // BC1_UNORM
        assert_eq!(map_gx2_to_switch_format(0x833), 0x1C06); // BC3_SRGB
        assert_eq!(map_gx2_to_switch_format(0x41A), 0x0B01); // R8_G8_B8_A8_UNORM
    }

    #[test]
    fn test_channel_selector_mapping() {
        assert_eq!(map_channel_selector(0), 2); // R → Red
        assert_eq!(map_channel_selector(3), 5); // A → Alpha
        assert_eq!(map_channel_selector(4), 0); // Always0 → Zero
        assert_eq!(map_channel_selector(5), 1); // Always1 → One
    }

    #[test]
    fn test_animation_suffix_renaming() {
        let mut bfres = BfresFile {
            name: "test".to_string(),
            version: (0, 0, 0, 0),
            alignment: 0,
            models: Vec::new(),
            textures: Vec::new(),
            skeleton_anims: Vec::new(),
            material_anims: Vec::new(),
            bone_vis_anims: Vec::new(),
            shape_anims: Vec::new(),
            scene_anims: Vec::new(),
            external_files: Vec::new(),
            shader_param_anims: vec![
                MaterialAnimation {
                    name: "Wait".to_string(),
                    path: String::new(),
                    flags: 0,
                    frame_count: 10,
                    baked_size: 0,
                    raw_data: Vec::new(),
                },
            ],
            color_anims: Vec::new(),
            tex_srt_anims: Vec::new(),
            tex_pattern_anims: vec![
                MaterialAnimation {
                    name: "Wait".to_string(),
                    path: String::new(),
                    flags: 0,
                    frame_count: 20,
                    baked_size: 0,
                    raw_data: Vec::new(),
                },
            ],
            mat_vis_anims: Vec::new(),
        };

        combine_material_anims(&mut bfres);

        assert_eq!(bfres.material_anims.len(), 2);
        assert_eq!(bfres.material_anims[0].name, "Wait_fsp");
        assert_eq!(bfres.material_anims[1].name, "Wait_ftp");
        assert!(bfres.shader_param_anims.is_empty());
        assert!(bfres.tex_pattern_anims.is_empty());
    }
}
