use roead::sarc::*;
use serde::{Deserialize, Serialize};
use smartstring::alias::String;

use crate::{
    prelude::*,
    util::{HashSet, IndexMap},
    Result, UKError,
};

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct LayoutArchive(pub IndexMap<String, Vec<u8>>);

impl TryFrom<&'_ Sarc<'_>> for LayoutArchive {
    type Error = UKError;

    fn try_from(sarc: &'_ Sarc) -> Result<Self> {
        Ok(Self(
            sarc.files()
                .filter_map(|f| f.name.map(|n| (n.into(), f.data.to_vec())))
                .collect(),
        ))
    }
}

impl MergeableImpl for LayoutArchive {
    fn diff(&self, other: &Self) -> Self {
        Self(
            other
                .0
                .iter()
                .filter_map(|(n, d)| (self.0.get(n) != Some(d)).then(|| (n.clone(), d.clone())))
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

impl Resource for LayoutArchive {
    fn from_binary(data: impl AsRef<[u8]>) -> crate::Result<Self> {
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
            .map(|x| x.ends_with("blarc"))
            .unwrap_or(false)
    }
}

#[cfg(feature = "ui")]
impl nk_ui::editor::EditableValue for LayoutArchive {
    const DISPLAY: nk_ui::editor::EditableDisplay = nk_ui::editor::EditableDisplay::Block;

    fn edit_ui(&mut self, ui: &mut nk_ui::egui::Ui) -> nk_ui::egui::Response {
        ui.label("Cannot visually edit BLARC")
    }

    fn edit_ui_with_id(
        &mut self,
        ui: &mut nk_ui::egui::Ui,
        _id: impl std::hash::Hash,
    ) -> nk_ui::egui::Response {
        self.edit_ui(ui)
    }
}
