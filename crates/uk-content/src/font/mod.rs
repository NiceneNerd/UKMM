use roead::sarc::*;
use serde::{Deserialize, Serialize};
use smartstring::alias::String;

use crate::{
    prelude::*,
    util::{HashSet, IndexMap},
    Result, UKError,
};

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct FontArchive(pub IndexMap<String, Vec<u8>>);

impl TryFrom<&'_ Sarc<'_>> for FontArchive {
    type Error = UKError;

    fn try_from(sarc: &'_ Sarc) -> Result<Self> {
        Ok(Self(
            sarc.files()
                .filter_map(|f| f.name.map(|n| (n.into(), f.data.to_vec())))
                .collect(),
        ))
    }
}

impl Mergeable for FontArchive {
    fn diff(&self, other: &Self) -> Self {
        Self(
            other
                .0
                .iter()
                .filter(|&(n, d)| self.0.get(n) != Some(d))
                .map(|(n, d)| (n.clone(), d.clone()))
                .collect(),
        )
    }

    #[allow(clippy::unwrap_used)]
    fn merge(&self, diff: &Self) -> Self {
        let keys: HashSet<String> = self.0.keys().chain(diff.0.keys()).cloned().collect();
        Self(
            keys.into_iter()
                .map(|k| {
                    let v = diff
                        .0
                        .get(&k)
                        .or_else(|| self.0.get(&k))
                        .map(|v| v.to_vec())
                        .unwrap();
                    (k, v)
                })
                .collect(),
        )
    }
}

impl Resource for FontArchive {
    fn from_binary(data: impl AsRef<[u8]>) -> Result<Self> {
        let sarc = Sarc::new(data.as_ref())?;
        Self::try_from(&sarc)
    }

    fn into_binary(self, endian: Endian) -> Vec<u8> {
        SarcWriter::new(endian.into())
            .with_legacy_mode(true)
            .with_min_alignment(4)
            .with_files(self.0)
            .to_binary()
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref()
            .extension()
            .and_then(|x| x.to_str())
            .map(|x| x.ends_with("bfarc"))
            .unwrap_or(false)
    }
}
