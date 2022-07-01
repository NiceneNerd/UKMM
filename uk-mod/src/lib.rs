#![feature(let_chains)]
pub mod data;

// #[cfg(test)]
// mod tests {
//     use std::collections::{BTreeMap, BTreeSet};

//     use rayon::prelude::*;
//     use roead::sarc::Sarc;
//     use xxhash_rust::xxh3::xxh3_64 as hash;

//     #[test]
//     fn create_hash_table() {
//         let content_dir = std::path::Path::new(
//             "/media/mrm/Data/Games/Cemu/mlc01/usr/title/00050000/101C9400/content",
//         );
//         let update_dir = std::path::Path::new(
//             "/media/mrm/Data/Games/Cemu/mlc01/usr/title/0005000E/101C9400/content",
//         );
//         let aoc_dir = std::path::Path::new(
//             "/media/mrm/Data/Games/Cemu/mlc01/usr/title/0005000C/101C9400/content/0010",
//         );

//         fn process_sarc_files(sarc: Sarc, aoc: bool) -> BTreeMap<u64, u64> {
//             let mut hashes = BTreeMap::<u64, u64>::new();
//             for file in sarc.files() {
//                 let data = roead::yaz0::decompress_if(file.data()).unwrap();
//                 let name = if aoc {
//                     ["Aoc/0010/", file.name_unchecked()].join("")
//                 } else {
//                     file.name_unchecked().to_owned()
//                 };
//                 hashes.insert(hash(name.replace(".s", ".").as_bytes()), hash(&data));
//                 let ext = std::path::Path::new(&name)
//                     .extension()
//                     .unwrap()
//                     .to_str()
//                     .unwrap();
//                 if data.starts_with(b"SARC")
//                     && !["blarc", "bfarc", "genvb", "sarc", "tera"].contains(&ext)
//                 {
//                     let sarc = Sarc::read(data.as_ref()).unwrap();
//                     hashes.extend(process_sarc_files(sarc, aoc));
//                 }
//             }
//             hashes
//         }

//         println!("Collecting content files...");
//         let content_files = jwalk::WalkDir::new(content_dir)
//             .into_iter()
//             .filter_map(Result::ok)
//             .filter(|f| f.file_type().is_file())
//             .map(|file| file.path())
//             .chain(
//                 jwalk::WalkDir::new(update_dir)
//                     .into_iter()
//                     .filter_map(Result::ok)
//                     .filter(|f| f.file_type().is_file())
//                     .map(|file| file.path()),
//             )
//             .collect::<BTreeSet<_>>();
//         println!("Hashing content files...");
//         let content_hashes = content_files
//             .into_par_iter()
//             .map(|file| {
//                 let mut hashes = BTreeMap::<u64, u64>::new();
//                 let data = std::fs::read(&file).unwrap();
//                 let data = roead::yaz0::decompress_if(&data).unwrap();
//                 hashes.insert(
//                     hash(
//                         file.strip_prefix(update_dir)
//                             .or_else(|_| file.strip_prefix(content_dir))
//                             .unwrap()
//                             .to_str()
//                             .unwrap()
//                             .replace(".s", ".")
//                             .as_bytes(),
//                     ),
//                     hash(&data),
//                 );
//                 if data.starts_with(b"SARC")
//                     && file.extension().unwrap().to_str().unwrap() != "sstera"
//                 {
//                     let sarc = Sarc::read(data.as_ref()).unwrap();
//                     hashes.extend(process_sarc_files(sarc, false));
//                 }
//                 hashes
//             })
//             .flatten()
//             .collect::<BTreeMap<_, _>>();

//         println!("Collecting aoc files...");
//         let aoc_files = jwalk::WalkDir::new(aoc_dir)
//             .into_iter()
//             .filter_map(Result::ok)
//             .filter(|f| f.file_type().is_file())
//             .map(|file| file.path())
//             .collect::<Vec<_>>();
//         println!("Hashing aoc files...");
//         let aoc_hashes = aoc_files
//             .into_par_iter()
//             .map(|file| {
//                 let mut hashes = BTreeMap::<u64, u64>::new();
//                 let data = std::fs::read(&file).unwrap();
//                 let data = roead::yaz0::decompress_if(&data).unwrap();
//                 hashes.insert(
//                     hash(
//                         [
//                             "Aoc/0010/",
//                             &file
//                                 .strip_prefix(aoc_dir)
//                                 .unwrap()
//                                 .to_str()
//                                 .unwrap()
//                                 .replace(".s", "."),
//                         ]
//                         .join("")
//                         .as_bytes(),
//                     ),
//                     hash(&data),
//                 );
//                 if data.starts_with(b"SARC")
//                     && file.extension().unwrap().to_str().unwrap() != "sstera"
//                 {
//                     let sarc = Sarc::read(data.as_ref()).unwrap();
//                     hashes.extend(process_sarc_files(sarc, true));
//                 }
//                 hashes
//             })
//             .flatten()
//             .collect::<BTreeMap<_, _>>();

//         println!("Serializing...");
//         let all_hashes = content_hashes
//             .into_iter()
//             .chain(aoc_hashes.into_iter())
//             .collect::<BTreeMap<_, _>>();
//         let mut out_file = std::fs::File::create("../.vscode/hashes.bin").unwrap();
//         out_file
//             .set_len((std::mem::size_of::<u64>() * all_hashes.len() * 2) as u64)
//             .unwrap();
//         use std::io::Write;
//         for k in all_hashes.keys() {
//             out_file.write_all(k.to_be_bytes().as_ref()).unwrap();
//         }
//         for v in all_hashes.values() {
//             out_file.write_all(v.to_be_bytes().as_ref()).unwrap();
//         }
//         out_file.flush().unwrap();
//     }

//     #[test]
//     fn compress_hashes() {
//         let data = std::fs::read("../.vscode/hashes.bin").unwrap();
//         std::fs::write(
//             "../.vscode/hashes.zbin",
//             zstd::encode_all(std::io::Cursor::new(data), zstd::DEFAULT_COMPRESSION_LEVEL).unwrap(),
//         )
//         .unwrap();
//     }
// }
