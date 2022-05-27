pub mod cdungeon;
pub mod mainfield;

#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct EntryPos {
    pub rotate: roead::byml::Byml,
    pub translate: roead::byml::Byml,
    pub player_state: Option<String>,
}
