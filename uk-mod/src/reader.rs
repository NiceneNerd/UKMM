use fs_err::File;
use std::{
    io::BufReader,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::{Manifest, Meta};

type TarReader<'a> = Arc<Mutex<tar::Archive<zstd::Decoder<'a, BufReader<File>>>>>;

pub struct ModReader<'a> {
    path: PathBuf,
    meta: Meta,
    manifest: Manifest,
    masters: Vec<Arc<uk_reader::GameROMReader>>,
    tar: TarReader<'a>,
}
