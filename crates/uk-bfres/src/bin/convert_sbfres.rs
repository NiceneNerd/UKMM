//! Batch converter for .sbfres files (Yaz0-compressed BFRES).
//! Converts Wii U big-endian BFRES to Switch little-endian format.
//!
//! Usage: convert_sbfres <input_dir> <output_dir>
//!   Finds all .sbfres files under input_dir, converts Wii U ones to Switch format,
//!   and writes them to the same relative path under output_dir.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

fn find_sbfres_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let entries = match std::fs::read_dir(&current) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|e| e.to_str()) == Some("sbfres") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

/// Yaz0 decompress. Returns raw data if not Yaz0-compressed.
fn yaz0_decompress(data: &[u8]) -> Vec<u8> {
    if data.len() < 16 || &data[0..4] != b"Yaz0" {
        return data.to_vec();
    }
    let decompressed_size = u32::from_be_bytes([data[4], data[5], data[6], data[7]]) as usize;
    let mut out = vec![0u8; decompressed_size];
    let mut src = 16usize;
    let mut dst = 0usize;

    while dst < decompressed_size && src < data.len() {
        let code = data[src];
        src += 1;

        for bit in (0..8).rev() {
            if dst >= decompressed_size || src >= data.len() {
                break;
            }
            if (code >> bit) & 1 == 1 {
                // Literal byte
                out[dst] = data[src];
                src += 1;
                dst += 1;
            } else {
                // Back-reference
                if src + 1 >= data.len() {
                    break;
                }
                let b0 = data[src] as usize;
                let b1 = data[src + 1] as usize;
                src += 2;

                let dist = ((b0 & 0x0F) << 8) | b1;
                let copy_src = dst.wrapping_sub(dist + 1);

                let length = if (b0 >> 4) == 0 {
                    if src >= data.len() {
                        break;
                    }
                    let b2 = data[src] as usize;
                    src += 1;
                    b2 + 0x12
                } else {
                    (b0 >> 4) + 2
                };

                for i in 0..length {
                    if dst >= decompressed_size {
                        break;
                    }
                    out[dst] = out[copy_src + i];
                    dst += 1;
                }
            }
        }
    }
    out
}

/// Yaz0 compress with simple greedy matching.
fn yaz0_compress(data: &[u8]) -> Vec<u8> {
    const SEARCH_RANGE: usize = 0x1000;
    const MAX_MATCH: usize = 0x111;

    let input_size = data.len();
    let mut out = Vec::with_capacity(input_size + input_size / 8 + 256);

    // Header
    out.extend_from_slice(b"Yaz0");
    out.extend_from_slice(&(input_size as u32).to_be_bytes());
    out.extend_from_slice(&[0u8; 8]);

    let mut pos = 0usize;
    while pos < input_size {
        let code_pos = out.len();
        out.push(0); // placeholder for code byte
        let mut code = 0u8;

        for bit in 0..8 {
            if pos >= input_size {
                break;
            }

            // Find best match
            let search_start = pos.saturating_sub(SEARCH_RANGE);
            let max_match = std::cmp::min(MAX_MATCH, input_size - pos);
            let mut best_len = 2usize;
            let mut best_dist = 0usize;

            for s in search_start..pos {
                let mut len = 0;
                while len < max_match && data[s + len] == data[pos + len] {
                    len += 1;
                }
                if len > best_len {
                    best_len = len;
                    best_dist = pos - s - 1;
                    if best_len >= max_match {
                        break;
                    }
                }
            }

            if best_len >= 3 {
                // Back-reference
                if best_len <= 0x11 {
                    out.push((((best_len - 2) << 4) | ((best_dist >> 8) & 0x0F)) as u8);
                    out.push((best_dist & 0xFF) as u8);
                } else {
                    out.push(((best_dist >> 8) & 0x0F) as u8);
                    out.push((best_dist & 0xFF) as u8);
                    out.push((best_len - 0x12) as u8);
                }
                pos += best_len;
            } else {
                // Literal
                code |= 1 << (7 - bit);
                out.push(data[pos]);
                pos += 1;
            }
        }
        out[code_pos] = code;
    }
    out
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: convert_sbfres <input_dir> <output_dir>");
        eprintln!("  Converts Wii U .sbfres files to Switch format.");
        std::process::exit(1);
    }

    let input_dir = Path::new(&args[1]);
    let output_dir = Path::new(&args[2]);

    if !input_dir.is_dir() {
        eprintln!("Error: input directory does not exist: {}", input_dir.display());
        std::process::exit(1);
    }

    let files = find_sbfres_files(input_dir);
    eprintln!("Found {} .sbfres files", files.len());

    let converted = AtomicUsize::new(0);
    let skipped = AtomicUsize::new(0);
    let failed = AtomicUsize::new(0);

    for (i, file) in files.iter().enumerate() {
        let rel_path = file.strip_prefix(input_dir).unwrap();
        let out_path = output_dir.join(rel_path);

        // Read and decompress
        let raw = match std::fs::read(file) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("[{}/{}] FAIL read {}: {}", i + 1, files.len(), rel_path.display(), e);
                failed.fetch_add(1, Ordering::Relaxed);
                continue;
            }
        };

        let decompressed = yaz0_decompress(&raw);

        // Check if it's a Wii U BFRES
        if !uk_bfres::is_wiiu_bfres(&decompressed) {
            if uk_bfres::is_switch_bfres(&decompressed) {
                eprintln!("[{}/{}] SKIP (already Switch): {}", i + 1, files.len(), rel_path.display());
            } else {
                eprintln!("[{}/{}] SKIP (not BFRES): {}", i + 1, files.len(), rel_path.display());
            }
            skipped.fetch_add(1, Ordering::Relaxed);
            continue;
        }

        // Convert
        let switch_data = match uk_bfres::convert_wiiu_to_switch(&decompressed) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("[{}/{}] FAIL convert {}: {}", i + 1, files.len(), rel_path.display(), e);
                failed.fetch_add(1, Ordering::Relaxed);
                continue;
            }
        };

        // Re-compress with Yaz0
        let compressed = yaz0_compress(&switch_data);

        // Write output
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        match std::fs::write(&out_path, &compressed) {
            Ok(_) => {
                let c = converted.fetch_add(1, Ordering::Relaxed) + 1;
                if c % 50 == 0 || i + 1 == files.len() {
                    eprintln!("[{}/{}] Converted {} so far...", i + 1, files.len(), c);
                }
            }
            Err(e) => {
                eprintln!("[{}/{}] FAIL write {}: {}", i + 1, files.len(), rel_path.display(), e);
                failed.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    let c = converted.load(Ordering::Relaxed);
    let s = skipped.load(Ordering::Relaxed);
    let f = failed.load(Ordering::Relaxed);
    eprintln!();
    eprintln!("Done: {} converted, {} skipped, {} failed (of {} total)", c, s, f, files.len());
}
