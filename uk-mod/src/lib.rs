#![feature(let_chains, seek_stream_len)]
pub mod data;

// #[cfg(test)]
// mod tests {
//     use rayon::prelude::*;
//     // use roead::sarc::Sarc;
//     use sarc_rs::Sarc;
//     use std::{
//         collections::{BTreeMap, BTreeSet},
//         path::Path,
//     };
//     use walkdir::WalkDir;
//     use xxhash_rust::xxh3::xxh3_64 as hash;

//     #[test]
//     fn create_hash_table() {
//         let content_dir = Path::new(r"E:\Downloads\Botw USA Base Game (9400)\The Legend of Zelda Breath of the Wild\content");
//         let update_dir = Path::new(r"E:\Downloads\The Legend of Zelda BotW Update 1.5.0 v208 USA\content");
//         let aoc_dir = Path::new(r"E:\Downloads\BOTW DLC v80 (3.0) USA 9400 unpacked\mlc01\usr\title\00050000\101C9400\aoc\content\0010");
//         // let content_dir = std::path::Path::new(r"E:\GameDuo\Modding\BOTW\nxdump\update\romfs");
//         // let aoc_dir = std::path::Path::new(r"E:\GameDuo\Modding\BOTW\nxdump\dlc\romfs");

//         fn process_sarc_files(sarc: Sarc, aoc: bool) -> BTreeMap<u64, u64> {
//             let mut hashes = BTreeMap::<u64, u64>::new();
//             for file in sarc.files() {
//                 let data = roead::yaz0::decompress_if(file.data).unwrap();
//                 let name = if aoc {
//                     ["Aoc/0010/", file.name.unwrap()].join("")
//                 } else {
//                     file.name.unwrap().to_owned()
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
//                     let sarc = Sarc::new(data.as_ref()).unwrap();
//                     hashes.extend(process_sarc_files(sarc, aoc));
//                 }
//             }
//             hashes
//         }

//         println!("Collecting content files...");
//         let content_files = WalkDir::new(content_dir)
//             .into_iter()
//             .filter_map(Result::ok)
//             .filter(|f| f.file_type().is_file())
//             .map(|file| file.into_path())
//             .chain(
//                 WalkDir::new(update_dir)
//             .into_iter()
//             .filter_map(Result::ok)
//             .filter(|f| f.file_type().is_file())
//             .map(|file| file.into_path())
//             )
//             .collect::<BTreeSet<_>>();
//         println!("Hashing {} content files...", content_files.len());
//         let content_hashes = content_files
//             .into_par_iter()
//             .map(|file| {
//                 let mut hashes = BTreeMap::<u64, u64>::new();
//                 let data = std::fs::read(&file).unwrap();
//                 let data = roead::yaz0::decompress_if(&data).unwrap();
//                 let canon = uk_content::canonicalize(file.strip_prefix(update_dir).or_else(|_| file.strip_prefix(content_dir)).unwrap());
//                 println!("Canonical path {}", &canon);
//                 hashes.insert(hash(canon.as_bytes()), hash(&data));
//                 if data.starts_with(b"SARC")
//                     && file.extension().unwrap().to_str().unwrap() != "sstera"
//                 {
//                     let sarc = Sarc::new(data.as_ref()).unwrap();
//                     hashes.extend(process_sarc_files(sarc, false));
//                 }
//                 hashes
//             })
//             .flatten()
//             .collect::<BTreeMap<_, _>>();

//         println!("Collecting aoc files...");
//         let aoc_files = WalkDir::new(aoc_dir)
//             .into_iter()
//             .filter_map(Result::ok)
//             .filter(|f| f.file_type().is_file())
//             .map(|file| file.into_path())
//             .collect::<Vec<_>>();
//         println!("Hashing {} aoc files...", aoc_files.len());
//         let aoc_hashes = aoc_files
//             .into_par_iter()
//             .map(|file| {
//                 let mut hashes = BTreeMap::<u64, u64>::new();
//                 let data = std::fs::read(&file).unwrap();
//                 let data = roead::yaz0::decompress_if(&data).unwrap();
//                 let canon = uk_content::canonicalize(
//                     Path::new("Aoc/0010").join(file.strip_prefix(aoc_dir).unwrap()),
//                 );
//                 println!("Canonical path {}", &canon);
//                 hashes.insert(hash(canon.as_bytes()), hash(&data));
//                 if data.starts_with(b"SARC")
//                     && file.extension().unwrap().to_str().unwrap() != "sstera"
//                 {
//                     let sarc = Sarc::new(data.as_ref()).unwrap();
//                     hashes.extend(process_sarc_files(sarc, true));
//                 }
//                 hashes
//             })
//             .flatten()
//             .collect::<BTreeMap<_, _>>();

//         println!("Serializing...");
//         let all_hashes = content_hashes
//             .into_par_iter()
//             .chain(aoc_hashes.into_par_iter())
//             .collect::<BTreeMap<_, _>>();
//         println!("Collected {} hashes", all_hashes.len());
//         let mut buffer = std::io::Cursor::new(Vec::with_capacity(
//             std::mem::size_of::<u64>() * all_hashes.len() * 2,
//         ));
//         use std::io::{Seek, Write};
//         for k in all_hashes.keys() {
//             buffer.write_all(k.to_be_bytes().as_ref()).unwrap();
//         }
//         for v in all_hashes.values() {
//             buffer.write_all(v.to_be_bytes().as_ref()).unwrap();
//         }
//         buffer.flush().unwrap();
//         println!("Buffer at {} bytes", buffer.stream_len().unwrap());
//         buffer.rewind().unwrap();
//         let data = zstd::encode_all(buffer, *zstd::compression_level_range().end()).unwrap();
//         println!("Compressed at {} bytes", data.len());
//         std::fs::write(r"C:\Temp\hashes_u.bin", data).unwrap();
//     }
// }
