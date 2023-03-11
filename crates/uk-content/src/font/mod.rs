use roead::sarc::*;
use serde::{Deserialize, Serialize};
use smartstring::alias::String;

use crate::{prelude::*, util::IndexMap, Result, UKError};

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
                .filter_map(|(n, d)| (self.0.get(n) != Some(d)).then(|| (n.clone(), d.clone())))
                .collect(),
        )
    }

    fn merge(&self, diff: &Self) -> Self {
        Self(
            self.0
                .iter()
                .map(|(k, v)| (k.clone(), diff.0.get(k).unwrap_or(v).to_vec()))
                .collect(),
        )
    }
}

impl Resource for FontArchive {
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
            .map(|x| x.ends_with("bfarc"))
            .unwrap_or(false)
    }
}

#[cfg(feature = "ui")]
impl uk_ui::editor::EditableValue for FontArchive {
    const DISPLAY: uk_ui::editor::EditableDisplay = uk_ui::editor::EditableDisplay::Block;

    fn edit_ui(&mut self, ui: &mut uk_ui::egui::Ui) -> uk_ui::egui::Response {
        ui.label("Cannot visually edit BFARC")
    }

    fn edit_ui_with_id(
        &mut self,
        ui: &mut uk_ui::egui::Ui,
        _id: impl std::hash::Hash,
    ) -> uk_ui::egui::Response {
        self.edit_ui(ui)
    }
}
