use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use uk_content::prelude::*;

// #[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
// pub struct DataTree {
//     pub file_map: BTreeMap<String, String>,
//     pub resources: BTreeMap<String, Box<dyn Mergeable>>,
// }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MergeableResource {
    BarslistInfo(Box<uk_content::sound::barslist::BarslistInfo>),
    WorldInfo(Box<uk_content::worldmgr::info::WorldInfo>),
    QuestProduct(Box<uk_content::quest::product::QuestProduct>),
    Tips(Box<uk_content::tips::Tips>),
    Demo(Box<uk_content::demo::Demo>),
    EventInfo(Box<uk_content::event::info::EventInfo>),
    MapUnit(Box<uk_content::map::unit::MapUnit>),
    ResidentEvents(Box<uk_content::event::residents::ResidentEvents>),
    Static(Box<uk_content::map::static_::Static>),
    StatusEffectList(Box<uk_content::eco::status::StatusEffectList>),
    LazyTraverseList(Box<uk_content::map::lazy::LazyTraverseList>),
    Location(Box<uk_content::map::mainfield::location::Location>),
    ShopGameDataInfo(Box<uk_content::data::shop::ShopGameDataInfo>),
    LevelSensor(Box<uk_content::eco::level::LevelSensor>),
    ChemicalRes(Box<uk_content::chemical::chmres::ChemicalRes>),
    CookData(Box<uk_content::cooking::data::CookData>),
    AreaData(Box<uk_content::eco::areadata::AreaData>),
    ResidentActors(Box<uk_content::actor::residents::ResidentActors>),
    SaveDataPack(Box<uk_content::data::savedata::SaveDataPack>),
    UMii(Box<uk_content::actor::params::umii::UMii>),
    ActorInfo(Box<uk_content::actor::info::ActorInfo>),
    Actor(Box<uk_content::actor::Actor>),
    AttClientList(Box<uk_content::actor::params::atcllist::AttClientList>),
    DropTable(Box<uk_content::actor::params::drop::DropTable>),
    RagdollConfigList(Box<uk_content::actor::params::rgconfiglist::RagdollConfigList>),
    Lod(Box<uk_content::actor::params::lod::Lod>),
    Recipe(Box<uk_content::actor::params::recipe::Recipe>),
    ShopData(Box<uk_content::actor::params::shop::ShopData>),
    GameDataPack(Box<uk_content::data::gamedata::GameDataPack>),
    RagdollConfig(Box<uk_content::actor::params::rgconfig::RagdollConfig>),
    DamageParam(Box<uk_content::actor::params::damage::DamageParam>),
    Physics(Box<uk_content::actor::params::physics::Physics>),
    GeneralParamList(Box<uk_content::actor::params::general::GeneralParamList>),
    RagdollBlendWeight(Box<uk_content::actor::params::rgbw::RagdollBlendWeight>),
    ModelList(Box<uk_content::actor::params::modellist::ModelList>),
    ActorLink(Box<uk_content::actor::params::link::ActorLink>),
    Awareness(Box<uk_content::actor::params::aware::Awareness>),
    AnimationInfo(Box<uk_content::actor::params::animinfo::AnimationInfo>),
    Chemical(Box<uk_content::actor::params::chemical::Chemical>),
    AISchedule(Box<uk_content::actor::params::aischedule::AISchedule>),
    BoneControl(Box<uk_content::actor::params::bonectrl::BoneControl>),
    AttClient(Box<uk_content::actor::params::atcl::AttClient>),
    ASList(Box<uk_content::actor::params::aslist::ASList>),
    AIProgram(Box<uk_content::actor::params::aiprog::AIProgram>),
    AS(Box<uk_content::actor::params::r#as::AS>),
    LifeCondition(Box<uk_content::actor::params::life::LifeCondition>),
}

pub enum ResourceData {
    Binary(Vec<u8>),
    Mergeable(MergeableResource),
}
