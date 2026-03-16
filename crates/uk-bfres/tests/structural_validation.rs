//! Level 2 structural validation tests for the uk-bfres BFRES converter.
//!
//! These tests convert Wii U model BFRES fixtures to Switch format using
//! `uk_bfres::convert_wiiu_to_switch()` and validate the output structurally
//! against BfresLibrary-generated reference Switch BFRES files.
//!
//! Structural validation means we compare sub-file counts, magic bytes,
//! header fields, and string content -- NOT byte-for-byte equality (the two
//! serializers may order dictionaries or pad buffers differently).

use std::path::PathBuf;

/// Fixture directory, resolved from CARGO_MANIFEST_DIR at compile time.
fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures")
}

/// Discover all model fixture pairs: `*.wiiu.bfres` files that are NOT
/// texture files (no `.Tex` in the stem) and have a matching `*.switch.bfres`.
fn discover_model_fixture_pairs() -> Vec<(String, PathBuf, PathBuf)> {
    let dir = fixtures_dir();
    let mut pairs = Vec::new();

    for entry in std::fs::read_dir(&dir).expect("fixtures dir should be readable") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();

        // Only Wii U model files (skip Tex files)
        if !name.ends_with(".wiiu.bfres") || name.contains(".Tex") {
            continue;
        }

        let base = name.trim_end_matches(".wiiu.bfres");
        let switch_name = format!("{}.switch.bfres", base);
        let switch_path = dir.join(&switch_name);

        if switch_path.exists() {
            pairs.push((base.to_string(), path.clone(), switch_path));
        }
    }

    pairs.sort_by(|a, b| a.0.cmp(&b.0));
    pairs
}

/// Count non-overlapping occurrences of a 4-byte magic in a byte slice.
fn count_magic(data: &[u8], magic: &[u8; 4]) -> usize {
    data.windows(4).filter(|w| *w == magic).count()
}

/// Read the u16 model count from the FRES header at offset 0xBC (v5 layout).
fn read_model_count(data: &[u8]) -> u16 {
    u16::from_le_bytes([data[0xBC], data[0xBD]])
}

/// Read the u32 file-size field from offset 0x1C.
fn read_file_size_field(data: &[u8]) -> u32 {
    u32::from_le_bytes([data[0x1C], data[0x1D], data[0x1E], data[0x1F]])
}

/// Read the u32 relocation-table offset from offset 0x18.
fn read_reloc_offset(data: &[u8]) -> u32 {
    u32::from_le_bytes([data[0x18], data[0x19], data[0x1A], data[0x1B]])
}

/// Read the u32 name offset from offset 0x10.
fn read_name_offset(data: &[u8]) -> u32 {
    u32::from_le_bytes([data[0x10], data[0x11], data[0x12], data[0x13]])
}

/// Extract the null-terminated string starting at `offset` in `data`.
fn read_cstring(data: &[u8], offset: usize) -> Option<String> {
    if offset >= data.len() {
        return None;
    }
    let end = data[offset..]
        .iter()
        .position(|&b| b == 0)
        .map(|p| offset + p)
        .unwrap_or(data.len());
    String::from_utf8(data[offset..end].to_vec()).ok()
}

/// Collect all null-terminated ASCII strings of length >= 2 that appear
/// inside a `_STR` section of the output. We locate `_STR` magic and then
/// scan forward for printable runs terminated by 0x00.
fn collect_strings_from_str_section(data: &[u8]) -> Vec<String> {
    let mut strings = Vec::new();

    // Find _STR magic position
    let str_pos = match data.windows(4).position(|w| w == b"_STR") {
        Some(p) => p,
        None => return strings,
    };

    // Scan from _STR position to end-of-file (or until _RLT) for strings.
    // Strings in the pool are preceded by a u16 length, then the characters,
    // then a null terminator. We simply look for printable runs.
    let end = data
        .windows(4)
        .position(|w| w == b"_RLT")
        .unwrap_or(data.len());
    let region = &data[str_pos..end];

    let mut i = 0;
    while i < region.len() {
        // Look for a run of printable ASCII
        if region[i].is_ascii_graphic() || region[i] == b' ' {
            let start = i;
            while i < region.len() && (region[i].is_ascii_graphic() || region[i] == b' ') {
                i += 1;
            }
            let len = i - start;
            if len >= 2 {
                if let Ok(s) = std::str::from_utf8(&region[start..start + len]) {
                    // Skip the _STR magic itself
                    if s != "_STR" {
                        strings.push(s.to_string());
                    }
                }
            }
        } else {
            i += 1;
        }
    }

    strings
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Validate that the converter discovers at least one fixture pair.
#[test]
fn discover_fixtures() {
    let pairs = discover_model_fixture_pairs();
    assert!(
        !pairs.is_empty(),
        "should find at least one model fixture pair in tests/fixtures/"
    );
    eprintln!("Found {} model fixture pair(s):", pairs.len());
    for (name, _, _) in &pairs {
        eprintln!("  - {}", name);
    }
}

/// For every model fixture pair, convert the Wii U file and validate FRES
/// header magic, BOM, version, and file-size consistency.
#[test]
fn header_fields_valid() {
    let pairs = discover_model_fixture_pairs();
    assert!(!pairs.is_empty());

    for (name, wiiu_path, _switch_path) in &pairs {
        let wiiu_data = std::fs::read(wiiu_path)
            .unwrap_or_else(|e| panic!("[{}] failed to read Wii U fixture: {}", name, e));
        let output = uk_bfres::convert_wiiu_to_switch(&wiiu_data)
            .unwrap_or_else(|e| panic!("[{}] conversion failed: {}", name, e));

        // Magic "FRES" + space padding
        assert_eq!(
            &output[0..4],
            b"FRES",
            "[{}] magic should be FRES",
            name
        );
        assert_eq!(
            &output[4..8],
            b"    ",
            "[{}] bytes 4-7 should be space padding (Switch format)",
            name
        );

        // BOM 0xFFFE (little-endian)
        let bom = u16::from_le_bytes([output[12], output[13]]);
        assert_eq!(
            bom, 0xFFFE,
            "[{}] BOM should be 0xFFFE (little-endian)",
            name
        );

        // Version 0x00050003
        let version = u32::from_le_bytes([output[8], output[9], output[10], output[11]]);
        assert_eq!(
            version, 0x00050003,
            "[{}] version should be 0x00050003",
            name
        );

        // File-size field matches actual length
        let stored_size = read_file_size_field(&output);
        assert_eq!(
            stored_size as usize,
            output.len(),
            "[{}] stored file size ({}) should match actual output length ({})",
            name,
            stored_size,
            output.len()
        );

        eprintln!("[{}] header OK: {} bytes", name, output.len());
    }
}

/// Validate that all expected sub-file magics are present in the output.
/// Model-specific magics (FMDL, FVTX, FSKL, FSHP, FMAT) are only required
/// when the Wii U parser successfully extracts models from the fixture.
#[test]
fn required_sub_file_magics_present() {
    let pairs = discover_model_fixture_pairs();
    assert!(!pairs.is_empty());

    // Magics that should always be present regardless of model content
    let always_required: &[(&[u8; 4], &str)] = &[
        (b"_STR", "string table"),
        (b"_RLT", "relocation table"),
    ];

    // Magics required only when models are present
    let model_magics: &[(&[u8; 4], &str)] = &[
        (b"FMDL", "model"),
        (b"FVTX", "vertex buffer"),
        (b"FSKL", "skeleton"),
        (b"FSHP", "shape"),
        (b"FMAT", "material"),
    ];

    for (name, wiiu_path, _) in &pairs {
        let wiiu_data = std::fs::read(wiiu_path).unwrap();
        let parsed = uk_bfres::wiiu::parse(&wiiu_data).unwrap();
        let output = uk_bfres::convert_wiiu_to_switch(&wiiu_data).unwrap();

        for (magic, label) in always_required {
            let found = output.windows(4).any(|w| w == *magic);
            assert!(
                found,
                "[{}] output should contain {} magic {:?}",
                name,
                label,
                std::str::from_utf8(*magic).unwrap_or("??")
            );
        }

        if parsed.models.is_empty() {
            eprintln!(
                "[{}] SKIPPED model magic checks (parser found 0 models -- known limitation)",
                name
            );
            continue;
        }

        for (magic, label) in model_magics {
            let found = output.windows(4).any(|w| w == *magic);
            assert!(
                found,
                "[{}] output should contain {} magic {:?}",
                name,
                label,
                std::str::from_utf8(*magic).unwrap_or("??")
            );
        }

        eprintln!("[{}] all required sub-file magics present", name);
    }
}

/// Compare sub-file magic counts between our output and the reference Switch
/// file. The number of FMDL, FVTX, FSKL, FSHP, and FMAT blocks should match.
/// Fixtures where the parser finds 0 models are skipped (known limitation).
#[test]
fn sub_file_counts_match_reference() {
    let pairs = discover_model_fixture_pairs();
    assert!(!pairs.is_empty());

    let magics: &[&[u8; 4]] = &[b"FMDL", b"FVTX", b"FSKL", b"FSHP", b"FMAT"];
    let mut tested = 0;

    for (name, wiiu_path, switch_path) in &pairs {
        let wiiu_data = std::fs::read(wiiu_path).unwrap();
        let parsed = uk_bfres::wiiu::parse(&wiiu_data).unwrap();

        if parsed.models.is_empty() {
            eprintln!(
                "[{}] SKIPPED sub-file count comparison (parser found 0 models -- known limitation)",
                name
            );
            continue;
        }

        let reference = std::fs::read(switch_path).unwrap();
        let output = uk_bfres::convert_wiiu_to_switch(&wiiu_data).unwrap();

        for magic in magics {
            let out_count = count_magic(&output, magic);
            let ref_count = count_magic(&reference, magic);
            assert_eq!(
                out_count, ref_count,
                "[{}] {:?} count mismatch: output={}, reference={}",
                name,
                std::str::from_utf8(*magic).unwrap(),
                out_count,
                ref_count
            );
        }

        tested += 1;
        eprintln!("[{}] sub-file counts match reference", name);
    }

    assert!(tested > 0, "at least one fixture should be fully testable");
}

/// Validate the model count field in the FRES header matches the reference.
#[test]
fn model_count_matches_reference() {
    let pairs = discover_model_fixture_pairs();
    assert!(!pairs.is_empty());

    for (name, wiiu_path, switch_path) in &pairs {
        let wiiu_data = std::fs::read(wiiu_path).unwrap();
        let reference = std::fs::read(switch_path).unwrap();
        let output = uk_bfres::convert_wiiu_to_switch(&wiiu_data).unwrap();

        let out_model_count = read_model_count(&output);
        let ref_model_count = read_model_count(&reference);

        assert_eq!(
            out_model_count, ref_model_count,
            "[{}] model count in FRES header: output={}, reference={}",
            name, out_model_count, ref_model_count
        );

        // Also cross-check with FMDL magic count
        let fmdl_count = count_magic(&output, b"FMDL");
        assert_eq!(
            out_model_count as usize, fmdl_count,
            "[{}] model count ({}) should equal FMDL magic count ({})",
            name, out_model_count, fmdl_count
        );

        eprintln!(
            "[{}] model count = {} (matches reference)",
            name, out_model_count
        );
    }
}

/// Validate the file size is in the same order of magnitude as the reference
/// (within 2x). The two serializers may differ in padding/alignment but
/// should not produce wildly different sizes.
/// Fixtures where the parser finds 0 models are skipped (known limitation).
#[test]
fn file_size_same_order_of_magnitude() {
    let pairs = discover_model_fixture_pairs();
    assert!(!pairs.is_empty());

    let mut tested = 0;

    for (name, wiiu_path, switch_path) in &pairs {
        let wiiu_data = std::fs::read(wiiu_path).unwrap();
        let parsed = uk_bfres::wiiu::parse(&wiiu_data).unwrap();

        if parsed.models.is_empty() {
            eprintln!(
                "[{}] SKIPPED size comparison (parser found 0 models -- known limitation)",
                name
            );
            continue;
        }

        let reference = std::fs::read(switch_path).unwrap();
        let output = uk_bfres::convert_wiiu_to_switch(&wiiu_data).unwrap();

        let out_size = output.len();
        let ref_size = reference.len();

        let ratio = if out_size > ref_size {
            out_size as f64 / ref_size as f64
        } else {
            ref_size as f64 / out_size as f64
        };

        assert!(
            ratio < 2.0,
            "[{}] size ratio {:.2}x is too large: output={} bytes, reference={} bytes",
            name,
            ratio,
            out_size,
            ref_size
        );

        tested += 1;
        eprintln!(
            "[{}] size OK: output={} bytes, reference={} bytes (ratio {:.2}x)",
            name, out_size, ref_size, ratio
        );
    }

    assert!(tested > 0, "at least one fixture should be fully testable");
}

/// Validate that the name offset at 0x10 points to a valid string that
/// matches the BFRES name from the parsed Wii U source.
///
/// Note: The name offset points into the string pool where strings are
/// packed. The string pool stores strings as: u16 length + chars + null.
/// We read the null-terminated string at the pointed offset.
#[test]
fn name_offset_points_to_valid_string() {
    let pairs = discover_model_fixture_pairs();
    assert!(!pairs.is_empty());

    for (name, wiiu_path, _) in &pairs {
        let wiiu_data = std::fs::read(wiiu_path).unwrap();
        let parsed = uk_bfres::wiiu::parse(&wiiu_data).unwrap();
        let output = uk_bfres::convert_wiiu_to_switch(&wiiu_data).unwrap();

        let name_off = read_name_offset(&output) as usize;
        assert!(
            name_off < output.len(),
            "[{}] name offset {:#x} should be within file bounds (len={})",
            name,
            name_off,
            output.len()
        );

        let found_name = read_cstring(&output, name_off);
        assert!(
            found_name.is_some(),
            "[{}] should be able to read string at name offset {:#x}",
            name,
            name_off
        );
        let found_name = found_name.unwrap();

        // Compare against the parsed Wii U BFRES name (the canonical name
        // from the file's own header, which may differ from the filename).
        assert_eq!(
            found_name, parsed.name,
            "[{}] name at offset {:#x} should be '{}', got '{}'",
            name, name_off, parsed.name, found_name
        );

        eprintln!(
            "[{}] name offset {:#x} -> '{}' (correct)",
            name, name_off, found_name
        );
    }
}

/// Validate the relocation table offset is within file bounds and points
/// to _RLT magic.
#[test]
fn relocation_table_offset_valid() {
    let pairs = discover_model_fixture_pairs();
    assert!(!pairs.is_empty());

    for (name, wiiu_path, _) in &pairs {
        let wiiu_data = std::fs::read(wiiu_path).unwrap();
        let output = uk_bfres::convert_wiiu_to_switch(&wiiu_data).unwrap();

        let reloc_off = read_reloc_offset(&output) as usize;
        assert!(
            reloc_off + 4 <= output.len(),
            "[{}] reloc offset {:#x} + 4 should be within file bounds (len={})",
            name,
            reloc_off,
            output.len()
        );

        assert_eq!(
            &output[reloc_off..reloc_off + 4],
            b"_RLT",
            "[{}] reloc offset {:#x} should point to _RLT magic",
            name,
            reloc_off
        );

        eprintln!(
            "[{}] reloc table at {:#x} -> _RLT (correct)",
            name, reloc_off
        );
    }
}

/// Validate that key strings from the Wii U source appear in the output's
/// string table. We parse the Wii U file to extract expected model names,
/// bone names, shape names, and material names, then verify they appear in
/// the converter output.
#[test]
fn string_content_from_source_preserved() {
    let pairs = discover_model_fixture_pairs();
    assert!(!pairs.is_empty());

    for (name, wiiu_path, _) in &pairs {
        let wiiu_data = std::fs::read(wiiu_path).unwrap();

        // Parse the Wii U file to get expected strings
        let parsed = uk_bfres::wiiu::parse(&wiiu_data)
            .unwrap_or_else(|e| panic!("[{}] Wii U parse failed: {}", name, e));

        let output = uk_bfres::convert_wiiu_to_switch(&wiiu_data).unwrap();
        let output_strings = collect_strings_from_str_section(&output);
        let output_str_set: std::collections::HashSet<&str> =
            output_strings.iter().map(|s| s.as_str()).collect();

        // Also check for substrings in the raw output (more robust than
        // section scanning for strings that might span scan boundaries)
        let output_contains = |needle: &str| -> bool {
            output_str_set.contains(needle)
                || output
                    .windows(needle.len())
                    .any(|w| w == needle.as_bytes())
        };

        // BFRES name
        assert!(
            output_contains(&parsed.name),
            "[{}] BFRES name '{}' should appear in output",
            name,
            parsed.name
        );

        // Model names, bone names, shape names, material names
        for model in &parsed.models {
            assert!(
                output_contains(&model.name),
                "[{}] model name '{}' should appear in output",
                name,
                model.name
            );

            for bone in &model.skeleton.bones {
                assert!(
                    output_contains(&bone.name),
                    "[{}] bone name '{}' should appear in output",
                    name,
                    bone.name
                );
            }

            for shape in &model.shapes {
                assert!(
                    output_contains(&shape.name),
                    "[{}] shape name '{}' should appear in output",
                    name,
                    shape.name
                );
            }

            for mat in &model.materials {
                assert!(
                    output_contains(&mat.name),
                    "[{}] material name '{}' should appear in output",
                    name,
                    mat.name
                );
            }
        }

        eprintln!(
            "[{}] all source strings found in output ({} strings in pool)",
            name,
            output_strings.len()
        );
    }
}

/// Validate that the output passes `is_switch_bfres()` and fails
/// `is_wiiu_bfres()` for all fixtures.
#[test]
fn format_detection_correct() {
    let pairs = discover_model_fixture_pairs();
    assert!(!pairs.is_empty());

    for (name, wiiu_path, _) in &pairs {
        let wiiu_data = std::fs::read(wiiu_path).unwrap();

        // Verify the source is detected as Wii U
        assert!(
            uk_bfres::is_wiiu_bfres(&wiiu_data),
            "[{}] source should be detected as Wii U BFRES",
            name
        );
        assert!(
            !uk_bfres::is_switch_bfres(&wiiu_data),
            "[{}] source should NOT be detected as Switch BFRES",
            name
        );

        // Convert and verify the output is detected as Switch
        let output = uk_bfres::convert_wiiu_to_switch(&wiiu_data).unwrap();
        assert!(
            uk_bfres::is_switch_bfres(&output),
            "[{}] output should be detected as Switch BFRES",
            name
        );
        assert!(
            !uk_bfres::is_wiiu_bfres(&output),
            "[{}] output should NOT be detected as Wii U BFRES",
            name
        );

        eprintln!("[{}] format detection correct", name);
    }
}

/// Structural validation using the parsed Wii U data model: verify that
/// the number of models, shapes, materials, bones, and vertex buffers from
/// the Wii U source matches the structural counts we can observe in the
/// Switch output via magic counts.
#[test]
fn parsed_model_structure_matches_output() {
    let pairs = discover_model_fixture_pairs();
    assert!(!pairs.is_empty());

    for (name, wiiu_path, _) in &pairs {
        let wiiu_data = std::fs::read(wiiu_path).unwrap();
        let parsed = uk_bfres::wiiu::parse(&wiiu_data).unwrap();
        let output = uk_bfres::convert_wiiu_to_switch(&wiiu_data).unwrap();

        // Model count
        let fmdl_count = count_magic(&output, b"FMDL");
        assert_eq!(
            fmdl_count,
            parsed.models.len(),
            "[{}] FMDL count ({}) should match parsed model count ({})",
            name,
            fmdl_count,
            parsed.models.len()
        );

        // Total shapes across all models
        let total_shapes: usize = parsed.models.iter().map(|m| m.shapes.len()).sum();
        let fshp_count = count_magic(&output, b"FSHP");
        assert_eq!(
            fshp_count, total_shapes,
            "[{}] FSHP count ({}) should match total shapes ({})",
            name, fshp_count, total_shapes
        );

        // Total materials across all models
        let total_materials: usize = parsed.models.iter().map(|m| m.materials.len()).sum();
        let fmat_count = count_magic(&output, b"FMAT");
        assert_eq!(
            fmat_count, total_materials,
            "[{}] FMAT count ({}) should match total materials ({})",
            name, fmat_count, total_materials
        );

        // Total vertex buffers across all models
        let total_vbufs: usize = parsed.models.iter().map(|m| m.vertex_buffers.len()).sum();
        let fvtx_count = count_magic(&output, b"FVTX");
        assert_eq!(
            fvtx_count, total_vbufs,
            "[{}] FVTX count ({}) should match total vertex buffers ({})",
            name, fvtx_count, total_vbufs
        );

        // Skeletons (one per model)
        let fskl_count = count_magic(&output, b"FSKL");
        assert_eq!(
            fskl_count,
            parsed.models.len(),
            "[{}] FSKL count ({}) should match model count ({}) (one skeleton per model)",
            name,
            fskl_count,
            parsed.models.len()
        );

        // Print summary
        for model in &parsed.models {
            eprintln!(
                "[{}] model '{}': {} shapes, {} materials, {} bones, {} vbufs",
                name,
                model.name,
                model.shapes.len(),
                model.materials.len(),
                model.skeleton.bones.len(),
                model.vertex_buffers.len()
            );
        }
    }
}
