use std::path::PathBuf;

use uk_mod::Meta;

#[derive(Debug, Clone)]
pub struct Project {
    pub path: PathBuf,
    pub meta: Meta,
}
