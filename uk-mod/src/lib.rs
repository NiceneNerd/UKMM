#![feature(let_chains, seek_stream_len)]

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use uk_content::prelude::Endian;
pub mod reader;
pub mod writer;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Manifest {
    #[serde(rename = "content")]
    pub content_files: BTreeSet<String>,
    #[serde(rename = "aoc")]
    pub aoc_files: BTreeSet<String>,
}

impl Manifest {
    pub fn resources(&self) -> impl Iterator<Item = String> + '_ {
        self.content_files
            .iter()
            .map(|s| s.replace(".s", "."))
            .chain(
                self.aoc_files
                    .iter()
                    .map(|s| ["Aoc/0010/", &s.replace(".s", ".")].join("")),
            )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Meta {
    pub name: String,
    pub version: f32,
    pub author: String,
    pub description: String,
    pub platform: Endian,
    pub url: Option<String>,
    pub id: Option<u64>,
    pub masters: Vec<u64>,
}
