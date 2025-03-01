use std::{borrow::Cow, collections::BTreeMap, path::Path};

use anyhow::{Context, Result};
use roead::{aamp::ParameterIO, byml::Byml, sarc::Sarc};
use serde::{Deserialize, Serialize};

pub use crate::{
    actor::{
        info::ActorInfo,
        params::{
            aiprog::AIProgram, aischedule::AISchedule, animinfo::AnimationInfo, r#as::AS,
            aslist::ASList, atcl::AttClient, atcllist::AttClientList, aware::Awareness,
            bonectrl::BoneControl, chemical::Chemical, damage::DamageParam, drop::DropTable,
            general::GeneralParamList, life::LifeCondition, link::ActorLink, lod::Lod,
            modellist::ModelList, physics::Physics, recipe::Recipe, rgbw::RagdollBlendWeight,
            rgconfig::RagdollConfig, rgconfiglist::RagdollConfigList, shop::ShopData, umii::UMii,
        },
        residents::ResidentActors,
        // Actor,
    },
    chemical::chmres::ChemicalRes,
    cooking::data::CookData,
    data::{gamedata::GameDataPack, savedata::SaveDataPack, shop::ShopGameDataInfo},
    demo::Demo,
    eco::{areadata::AreaData, level::LevelSensor, status::StatusEffectList},
    event::{info::EventInfo, residents::ResidentEvents},
    font::FontArchive,
    layout::LayoutArchive,
    map::{lazy::LazyTraverseList, mainfield::location::Location, static_::{MainStatic, Static}, unit::MapUnit},
    message::MessagePack,
    quest::product::QuestProduct,
    sound::barslist::BarslistInfo,
    tips::Tips,
    util::SortedDeleteMap,
    worldmgr::info::WorldInfo,
};
use crate::{prelude::*, util::SortedDeleteSet};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]

pub enum MergeableResource {
    // Actor(Box<Actor>),
    ActorInfo(Box<ActorInfo>),
    ActorLink(Box<ActorLink>),
    AIProgram(Box<AIProgram>),
    AISchedule(Box<AISchedule>),
    AnimationInfo(Box<AnimationInfo>),
    AreaData(Box<AreaData>),
    AS(Box<AS>),
    ASList(Box<ASList>),
    AttClient(Box<AttClient>),
    AttClientList(Box<AttClientList>),
    Awareness(Box<Awareness>),
    BarslistInfo(Box<BarslistInfo>),
    BoneControl(Box<BoneControl>),
    Chemical(Box<Chemical>),
    ChemicalRes(Box<ChemicalRes>),
    CookData(Box<CookData>),
    DamageParam(Box<DamageParam>),
    Demo(Box<Demo>),
    DropTable(Box<DropTable>),
    EventInfo(Box<EventInfo>),
    FontArchive(Box<FontArchive>),
    GameDataPack(Box<GameDataPack>),
    GeneralParamList(Box<GeneralParamList>),
    LayoutArchive(Box<LayoutArchive>),
    LazyTraverseList(Box<LazyTraverseList>),
    LevelSensor(Box<LevelSensor>),
    LifeCondition(Box<LifeCondition>),
    Location(Box<Location>),
    Lod(Box<Lod>),
    MainStatic(Box<MainStatic>),
    MapUnit(Box<MapUnit>),
    MessagePack(Box<MessagePack>),
    ModelList(Box<ModelList>),
    Physics(Box<Physics>),
    QuestProduct(Box<QuestProduct>),
    RagdollBlendWeight(Box<RagdollBlendWeight>),
    RagdollConfig(Box<RagdollConfig>),
    RagdollConfigList(Box<RagdollConfigList>),
    Recipe(Box<Recipe>),
    ResidentActors(Box<ResidentActors>),
    ResidentEvents(Box<ResidentEvents>),
    SaveDataPack(Box<SaveDataPack>),
    ShopData(Box<ShopData>),
    ShopGameDataInfo(Box<ShopGameDataInfo>),
    Static(Box<Static>),
    StatusEffectList(Box<StatusEffectList>),
    Tips(Box<Tips>),
    UMii(Box<UMii>),
    WorldInfo(Box<WorldInfo>),
    GenericAamp(Box<ParameterIO>),
    GenericByml(Box<Byml>),
    BinaryOverride(Box<(Vec<u8>, String)>),
}

impl std::fmt::Display for MergeableResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // Self::Actor(_) => "Actor",
            Self::ActorInfo(_) => "ActorInfo",
            Self::ActorLink(_) => "ActorLink",
            Self::AIProgram(_) => "AIProgram",
            Self::AISchedule(_) => "AISchedule",
            Self::AnimationInfo(_) => "AnimationInfo",
            Self::AreaData(_) => "AreaData",
            Self::AS(_) => "AS",
            Self::ASList(_) => "ASList",
            Self::AttClient(_) => "AttClient",
            Self::AttClientList(_) => "AttClientList",
            Self::Awareness(_) => "Awareness",
            Self::BarslistInfo(_) => "BarslistInfo",
            Self::BoneControl(_) => "BoneControl",
            Self::Chemical(_) => "Chemical",
            Self::ChemicalRes(_) => "ChemicalRes",
            Self::CookData(_) => "CookData",
            Self::DamageParam(_) => "DamageParam",
            Self::Demo(_) => "Demo",
            Self::DropTable(_) => "DropTable",
            Self::EventInfo(_) => "EventInfo",
            Self::FontArchive(_) => "FontArchive",
            Self::GameDataPack(_) => "GameDataPack",
            Self::GeneralParamList(_) => "GeneralParamList",
            Self::LazyTraverseList(_) => "LazyTraverseList",
            Self::LayoutArchive(_) => "LayoutArchive",
            Self::LevelSensor(_) => "LevelSensor",
            Self::LifeCondition(_) => "LifeCondition",
            Self::Location(_) => "Location",
            Self::Lod(_) => "Lod",
            Self::MainStatic(_) => "MainStatic",
            Self::MapUnit(_) => "MapUnit",
            Self::MessagePack(_) => "MessagePack",
            Self::ModelList(_) => "ModelList",
            Self::Physics(_) => "Physics",
            Self::QuestProduct(_) => "QuestProduct",
            Self::RagdollBlendWeight(_) => "RagdollBlendWeight",
            Self::RagdollConfig(_) => "RagdollConfig",
            Self::RagdollConfigList(_) => "RagdollConfigList",
            Self::Recipe(_) => "Recipe",
            Self::ResidentActors(_) => "ResidentActors",
            Self::ResidentEvents(_) => "ResidentEvents",
            Self::SaveDataPack(_) => "SaveDataPack",
            Self::ShopData(_) => "ShopData",
            Self::ShopGameDataInfo(_) => "ShopGameDataInfo",
            Self::Static(_) => "Static",
            Self::StatusEffectList(_) => "StatusEffectList",
            Self::Tips(_) => "Tips",
            Self::UMii(_) => "UMii",
            Self::WorldInfo(_) => "WorldInfo",
            Self::GenericAamp(_) => "GenericAamp",
            Self::GenericByml(_) => "GenericByml",
            Self::BinaryOverride(_) => "BinaryOverride",
        }
        .fmt(f)
    }
}

macro_rules! impl_from_res {
    ($type:ident) => {
        impl From<$type> for MergeableResource {
            fn from(res: $type) -> Self {
                MergeableResource::$type(Box::new(res))
            }
        }

        impl From<$type> for ResourceData {
            fn from(res: $type) -> Self {
                ResourceData::Mergeable(res.into())
            }
        }

        impl TryFrom<MergeableResource> for $type {
            type Error = anyhow::Error;

            fn try_from(res: MergeableResource) -> Result<Self> {
                match res {
                    MergeableResource::$type(res) => Ok(*res),
                    _ => Err(anyhow::anyhow!("Expected {}", stringify!($type))),
                }
            }
        }

        impl TryFrom<ResourceData> for $type {
            type Error = anyhow::Error;

            fn try_from(res: ResourceData) -> Result<Self> {
                match res {
                    ResourceData::Mergeable(MergeableResource::$type(res)) => Ok(*res),
                    _ => Err(anyhow::anyhow!("Expected {}", stringify!($type))),
                }
            }
        }
    };
}

// impl_from_res!(Actor);
impl_from_res!(ActorInfo);
impl_from_res!(ActorLink);
impl_from_res!(AIProgram);
impl_from_res!(AISchedule);
impl_from_res!(AnimationInfo);
impl_from_res!(AreaData);
impl_from_res!(AS);
impl_from_res!(ASList);
impl_from_res!(AttClient);
impl_from_res!(AttClientList);
impl_from_res!(Awareness);
impl_from_res!(BarslistInfo);
impl_from_res!(BoneControl);
impl_from_res!(Chemical);
impl_from_res!(ChemicalRes);
impl_from_res!(CookData);
impl_from_res!(DamageParam);
impl_from_res!(Demo);
impl_from_res!(DropTable);
impl_from_res!(EventInfo);
impl_from_res!(FontArchive);
impl_from_res!(GameDataPack);
impl_from_res!(GeneralParamList);
impl_from_res!(LazyTraverseList);
impl_from_res!(LayoutArchive);
impl_from_res!(LevelSensor);
impl_from_res!(LifeCondition);
impl_from_res!(Location);
impl_from_res!(Lod);
impl_from_res!(MainStatic);
impl_from_res!(MapUnit);
impl_from_res!(MessagePack);
impl_from_res!(ModelList);
impl_from_res!(Physics);
impl_from_res!(QuestProduct);
impl_from_res!(RagdollBlendWeight);
impl_from_res!(RagdollConfig);
impl_from_res!(RagdollConfigList);
impl_from_res!(Recipe);
impl_from_res!(ResidentActors);
impl_from_res!(ResidentEvents);
impl_from_res!(SaveDataPack);
impl_from_res!(ShopData);
impl_from_res!(ShopGameDataInfo);
impl_from_res!(Static);
impl_from_res!(StatusEffectList);
impl_from_res!(Tips);
impl_from_res!(UMii);
impl_from_res!(WorldInfo);

impl Mergeable for MergeableResource {
    fn diff(&self, other: &Self) -> Self {
        match (self, other) {
            // (Self::Actor(a), Self::Actor(b)) => Self::Actor(Box::new(a.diff(b))),
            (Self::ActorInfo(a), Self::ActorInfo(b)) => Self::ActorInfo(Box::new(a.diff(b))),
            (Self::ActorLink(a), Self::ActorLink(b)) => Self::ActorLink(Box::new(a.diff(b))),
            (Self::AIProgram(a), Self::AIProgram(b)) => Self::AIProgram(Box::new(a.diff(b))),
            (Self::AISchedule(a), Self::AISchedule(b)) => Self::AISchedule(Box::new(a.diff(b))),
            (Self::AnimationInfo(a), Self::AnimationInfo(b)) => {
                Self::AnimationInfo(Box::new(a.diff(b)))
            }
            (Self::AreaData(a), Self::AreaData(b)) => Self::AreaData(Box::new(a.diff(b))),
            (Self::AS(a), Self::AS(b)) => Self::AS(Box::new(a.diff(b))),
            (Self::ASList(a), Self::ASList(b)) => Self::ASList(Box::new(a.diff(b))),
            (Self::AttClient(a), Self::AttClient(b)) => Self::AttClient(Box::new(a.diff(b))),
            (Self::AttClientList(a), Self::AttClientList(b)) => {
                Self::AttClientList(Box::new(a.diff(b)))
            }
            (Self::Awareness(a), Self::Awareness(b)) => Self::Awareness(Box::new(a.diff(b))),
            (Self::BarslistInfo(a), Self::BarslistInfo(b)) => {
                Self::BarslistInfo(Box::new(a.diff(b)))
            }
            (Self::BoneControl(a), Self::BoneControl(b)) => Self::BoneControl(Box::new(a.diff(b))),
            (Self::Chemical(a), Self::Chemical(b)) => Self::Chemical(Box::new(a.diff(b))),
            (Self::ChemicalRes(a), Self::ChemicalRes(b)) => Self::ChemicalRes(Box::new(a.diff(b))),
            (Self::CookData(a), Self::CookData(b)) => Self::CookData(Box::new(a.diff(b))),
            (Self::DamageParam(a), Self::DamageParam(b)) => Self::DamageParam(Box::new(a.diff(b))),
            (Self::Demo(a), Self::Demo(b)) => Self::Demo(Box::new(a.diff(b))),
            (Self::DropTable(a), Self::DropTable(b)) => Self::DropTable(Box::new(a.diff(b))),
            (Self::EventInfo(a), Self::EventInfo(b)) => Self::EventInfo(Box::new(a.diff(b))),
            (Self::FontArchive(a), Self::FontArchive(b)) => Self::FontArchive(Box::new(a.diff(b))),
            (Self::GameDataPack(a), Self::GameDataPack(b)) => {
                Self::GameDataPack(Box::new(a.diff(b)))
            }
            (Self::GeneralParamList(a), Self::GeneralParamList(b)) => {
                Self::GeneralParamList(Box::new(a.diff(b)))
            }
            (Self::LazyTraverseList(a), Self::LazyTraverseList(b)) => {
                Self::LazyTraverseList(Box::new(a.diff(b)))
            }
            (Self::LayoutArchive(a), Self::LayoutArchive(b)) => {
                Self::LayoutArchive(Box::new(a.diff(b)))
            }
            (Self::LevelSensor(a), Self::LevelSensor(b)) => Self::LevelSensor(Box::new(a.diff(b))),
            (Self::LifeCondition(a), Self::LifeCondition(b)) => {
                Self::LifeCondition(Box::new(a.diff(b)))
            }
            (Self::Location(a), Self::Location(b)) => Self::Location(Box::new(a.diff(b))),
            (Self::Lod(a), Self::Lod(b)) => Self::Lod(Box::new(a.diff(b))),
            (Self::MainStatic(a), Self::MainStatic(b)) => Self::MainStatic(Box::new(a.diff(b))),
            (Self::MapUnit(a), Self::MapUnit(b)) => Self::MapUnit(Box::new(a.diff(b))),
            (Self::MessagePack(a), Self::MessagePack(b)) => Self::MessagePack(Box::new(a.diff(b))),
            (Self::ModelList(a), Self::ModelList(b)) => Self::ModelList(Box::new(a.diff(b))),
            (Self::Physics(a), Self::Physics(b)) => Self::Physics(Box::new(a.diff(b))),
            (Self::QuestProduct(a), Self::QuestProduct(b)) => {
                Self::QuestProduct(Box::new(a.diff(b)))
            }
            (Self::RagdollBlendWeight(a), Self::RagdollBlendWeight(b)) => {
                Self::RagdollBlendWeight(Box::new(a.diff(b)))
            }
            (Self::RagdollConfig(a), Self::RagdollConfig(b)) => {
                Self::RagdollConfig(Box::new(a.diff(b)))
            }
            (Self::RagdollConfigList(a), Self::RagdollConfigList(b)) => {
                Self::RagdollConfigList(Box::new(a.diff(b)))
            }
            (Self::Recipe(a), Self::Recipe(b)) => Self::Recipe(Box::new(a.diff(b))),
            (Self::ResidentActors(a), Self::ResidentActors(b)) => {
                Self::ResidentActors(Box::new(a.diff(b)))
            }
            (Self::ResidentEvents(a), Self::ResidentEvents(b)) => {
                Self::ResidentEvents(Box::new(a.diff(b)))
            }
            (Self::SaveDataPack(a), Self::SaveDataPack(b)) => {
                Self::SaveDataPack(Box::new(a.diff(b)))
            }
            (Self::ShopData(a), Self::ShopData(b)) => Self::ShopData(Box::new(a.diff(b))),
            (Self::ShopGameDataInfo(a), Self::ShopGameDataInfo(b)) => {
                Self::ShopGameDataInfo(Box::new(a.diff(b)))
            }
            (Self::Static(a), Self::Static(b)) => Self::Static(Box::new(a.diff(b))),
            (Self::StatusEffectList(a), Self::StatusEffectList(b)) => {
                Self::StatusEffectList(Box::new(a.diff(b)))
            }
            (Self::Tips(a), Self::Tips(b)) => Self::Tips(Box::new(a.diff(b))),
            (Self::UMii(a), Self::UMii(b)) => Self::UMii(Box::new(a.diff(b))),
            (Self::WorldInfo(a), Self::WorldInfo(b)) => Self::WorldInfo(Box::new(a.diff(b))),
            (Self::GenericByml(a), Self::GenericByml(b)) => Self::GenericByml(Box::new(a.diff(b))),
            (Self::GenericAamp(a), Self::GenericAamp(b)) => Self::GenericAamp(Box::new(a.diff(b))),
            (Self::BinaryOverride(_), anything) => anything.clone(),
            (_anything, Self::BinaryOverride(bin)) => Self::BinaryOverride(bin.clone()),
            _ => {
                panic!(
                    "Tried to diff incompatible resources: {} and {}",
                    &self, &other
                )
            }
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        match (self, diff) {
            // (Self::Actor(a), Self::Actor(b)) => Self::Actor(Box::new(a.merge(b))),
            (Self::ActorInfo(a), Self::ActorInfo(b)) => Self::ActorInfo(Box::new(a.merge(b))),
            (Self::ActorLink(a), Self::ActorLink(b)) => Self::ActorLink(Box::new(a.merge(b))),
            (Self::AIProgram(a), Self::AIProgram(b)) => Self::AIProgram(Box::new(a.merge(b))),
            (Self::AISchedule(a), Self::AISchedule(b)) => Self::AISchedule(Box::new(a.merge(b))),
            (Self::AnimationInfo(a), Self::AnimationInfo(b)) => {
                Self::AnimationInfo(Box::new(a.merge(b)))
            }
            (Self::AreaData(a), Self::AreaData(b)) => Self::AreaData(Box::new(a.merge(b))),
            (Self::AS(a), Self::AS(b)) => Self::AS(Box::new(a.merge(b))),
            (Self::ASList(a), Self::ASList(b)) => Self::ASList(Box::new(a.merge(b))),
            (Self::AttClient(a), Self::AttClient(b)) => Self::AttClient(Box::new(a.merge(b))),
            (Self::AttClientList(a), Self::AttClientList(b)) => {
                Self::AttClientList(Box::new(a.merge(b)))
            }
            (Self::Awareness(a), Self::Awareness(b)) => Self::Awareness(Box::new(a.merge(b))),
            (Self::BarslistInfo(a), Self::BarslistInfo(b)) => {
                Self::BarslistInfo(Box::new(a.merge(b)))
            }
            (Self::BoneControl(a), Self::BoneControl(b)) => Self::BoneControl(Box::new(a.merge(b))),
            (Self::Chemical(a), Self::Chemical(b)) => Self::Chemical(Box::new(a.merge(b))),
            (Self::ChemicalRes(a), Self::ChemicalRes(b)) => Self::ChemicalRes(Box::new(a.merge(b))),
            (Self::CookData(a), Self::CookData(b)) => Self::CookData(Box::new(a.merge(b))),
            (Self::DamageParam(a), Self::DamageParam(b)) => Self::DamageParam(Box::new(a.merge(b))),
            (Self::Demo(a), Self::Demo(b)) => Self::Demo(Box::new(a.merge(b))),
            (Self::DropTable(a), Self::DropTable(b)) => Self::DropTable(Box::new(a.merge(b))),
            (Self::EventInfo(a), Self::EventInfo(b)) => Self::EventInfo(Box::new(a.merge(b))),
            (Self::FontArchive(a), Self::FontArchive(b)) => Self::FontArchive(Box::new(a.merge(b))),
            (Self::GameDataPack(a), Self::GameDataPack(b)) => {
                Self::GameDataPack(Box::new(a.merge(b)))
            }
            (Self::GeneralParamList(a), Self::GeneralParamList(b)) => {
                Self::GeneralParamList(Box::new(a.merge(b)))
            }
            (Self::LazyTraverseList(a), Self::LazyTraverseList(b)) => {
                Self::LazyTraverseList(Box::new(a.merge(b)))
            }
            (Self::LayoutArchive(a), Self::LayoutArchive(b)) => {
                Self::LayoutArchive(Box::new(a.merge(b)))
            }
            (Self::LevelSensor(a), Self::LevelSensor(b)) => Self::LevelSensor(Box::new(a.merge(b))),
            (Self::LifeCondition(a), Self::LifeCondition(b)) => {
                Self::LifeCondition(Box::new(a.merge(b)))
            }
            (Self::Location(a), Self::Location(b)) => Self::Location(Box::new(a.merge(b))),
            (Self::Lod(a), Self::Lod(b)) => Self::Lod(Box::new(a.merge(b))),
            (Self::MainStatic(a), Self::MainStatic(b)) => Self::MainStatic(Box::new(a.merge(b))),
            (Self::MapUnit(a), Self::MapUnit(b)) => Self::MapUnit(Box::new(a.merge(b))),
            (Self::MessagePack(a), Self::MessagePack(b)) => Self::MessagePack(Box::new(a.merge(b))),
            (Self::ModelList(a), Self::ModelList(b)) => Self::ModelList(Box::new(a.merge(b))),
            (Self::Physics(a), Self::Physics(b)) => Self::Physics(Box::new(a.merge(b))),
            (Self::QuestProduct(a), Self::QuestProduct(b)) => {
                Self::QuestProduct(Box::new(a.merge(b)))
            }
            (Self::RagdollBlendWeight(a), Self::RagdollBlendWeight(b)) => {
                Self::RagdollBlendWeight(Box::new(a.merge(b)))
            }
            (Self::RagdollConfig(a), Self::RagdollConfig(b)) => {
                Self::RagdollConfig(Box::new(a.merge(b)))
            }
            (Self::RagdollConfigList(a), Self::RagdollConfigList(b)) => {
                Self::RagdollConfigList(Box::new(a.merge(b)))
            }
            (Self::Recipe(a), Self::Recipe(b)) => Self::Recipe(Box::new(a.merge(b))),
            (Self::ResidentActors(a), Self::ResidentActors(b)) => {
                Self::ResidentActors(Box::new(a.merge(b)))
            }
            (Self::ResidentEvents(a), Self::ResidentEvents(b)) => {
                Self::ResidentEvents(Box::new(a.merge(b)))
            }
            (Self::SaveDataPack(a), Self::SaveDataPack(b)) => {
                Self::SaveDataPack(Box::new(a.merge(b)))
            }
            (Self::ShopData(a), Self::ShopData(b)) => Self::ShopData(Box::new(a.merge(b))),
            (Self::ShopGameDataInfo(a), Self::ShopGameDataInfo(b)) => {
                Self::ShopGameDataInfo(Box::new(a.merge(b)))
            }
            (Self::Static(a), Self::Static(b)) => Self::Static(Box::new(a.merge(b))),
            (Self::StatusEffectList(a), Self::StatusEffectList(b)) => {
                Self::StatusEffectList(Box::new(a.merge(b)))
            }
            (Self::Tips(a), Self::Tips(b)) => Self::Tips(Box::new(a.merge(b))),
            (Self::UMii(a), Self::UMii(b)) => Self::UMii(Box::new(a.merge(b))),
            (Self::WorldInfo(a), Self::WorldInfo(b)) => Self::WorldInfo(Box::new(a.merge(b))),
            (Self::GenericByml(a), Self::GenericByml(b)) => Self::GenericByml(Box::new(a.merge(b))),
            (Self::GenericAamp(a), Self::GenericAamp(b)) => Self::GenericAamp(Box::new(a.merge(b))),
            (Self::BinaryOverride(bin), _anything) => Self::BinaryOverride(bin.clone()),
            (_anything, Self::BinaryOverride(bin)) => Self::BinaryOverride(bin.clone()),
            _ => {
                panic!(
                    "Tried to merge incompatible resources: {} and {}",
                    &self, &diff
                )
            }
        }
    }
}

impl MergeableResource {
    pub fn from_binary(name: &Path, data: &[u8]) -> Result<Option<MergeableResource>> {
        let result: Result<Option<MergeableResource>> = if ActorInfo::path_matches(name) {
            Ok(Some(Self::ActorInfo(Box::new(ActorInfo::from_binary(
                data,
            )?))))
        } else if ActorLink::path_matches(name) {
            Ok(Some(Self::ActorLink(Box::new(ActorLink::from_binary(
                data,
            )?))))
        } else if AIProgram::path_matches(name) {
            Ok(Some(Self::AIProgram(Box::new(AIProgram::from_binary(
                data,
            )?))))
        } else if AISchedule::path_matches(name) {
            Ok(Some(Self::AISchedule(Box::new(AISchedule::from_binary(
                data,
            )?))))
        } else if AnimationInfo::path_matches(name) {
            Ok(Some(Self::AnimationInfo(Box::new(
                AnimationInfo::from_binary(data)?,
            ))))
        } else if AreaData::path_matches(name) {
            Ok(Some(Self::AreaData(Box::new(AreaData::from_binary(data)?))))
        } else if AS::path_matches(name) {
            Ok(Some(Self::AS(Box::new(AS::from_binary(data)?))))
        } else if ASList::path_matches(name) {
            Ok(Some(Self::ASList(Box::new(ASList::from_binary(data)?))))
        } else if AttClient::path_matches(name) {
            Ok(Some(Self::AttClient(Box::new(AttClient::from_binary(
                data,
            )?))))
        } else if AttClientList::path_matches(name) {
            Ok(Some(Self::AttClientList(Box::new(
                AttClientList::from_binary(data)?,
            ))))
        } else if Awareness::path_matches(name) {
            Ok(Some(Self::Awareness(Box::new(Awareness::from_binary(
                data,
            )?))))
        } else if BarslistInfo::path_matches(name) {
            Ok(Some(Self::BarslistInfo(Box::new(
                BarslistInfo::from_binary(data)?,
            ))))
        } else if BoneControl::path_matches(name) {
            Ok(Some(Self::BoneControl(Box::new(BoneControl::from_binary(
                data,
            )?))))
        } else if Chemical::path_matches(name) {
            Ok(Some(Self::Chemical(Box::new(Chemical::from_binary(data)?))))
        } else if ChemicalRes::path_matches(name) {
            Ok(Some(Self::ChemicalRes(Box::new(ChemicalRes::from_binary(
                data,
            )?))))
        } else if CookData::path_matches(name) {
            Ok(Some(Self::CookData(Box::new(CookData::from_binary(data)?))))
        } else if DamageParam::path_matches(name) {
            Ok(Some(Self::DamageParam(Box::new(DamageParam::from_binary(
                data,
            )?))))
        } else if Demo::path_matches(name) {
            Ok(Some(Self::Demo(Box::new(Demo::from_binary(data)?))))
        } else if DropTable::path_matches(name) {
            Ok(Some(Self::DropTable(Box::new(DropTable::from_binary(
                data,
            )?))))
        } else if EventInfo::path_matches(name) {
            Ok(Some(Self::EventInfo(Box::new(EventInfo::from_binary(
                data,
            )?))))
        } else if FontArchive::path_matches(name) {
            Ok(Some(Self::FontArchive(Box::new(FontArchive::from_binary(
                data,
            )?))))
        } else if GameDataPack::path_matches(name) {
            Ok(Some(Self::GameDataPack(Box::new(
                GameDataPack::from_binary(data)?,
            ))))
        } else if GeneralParamList::path_matches(name) {
            Ok(Some(Self::GeneralParamList(Box::new(
                GeneralParamList::from_binary(data)?,
            ))))
        } else if LayoutArchive::path_matches(name) {
            Ok(Some(Self::LayoutArchive(Box::new(
                LayoutArchive::from_binary(data)?,
            ))))
        } else if LazyTraverseList::path_matches(name) {
            Ok(Some(Self::LazyTraverseList(Box::new(
                LazyTraverseList::from_binary(data)?,
            ))))
        } else if LevelSensor::path_matches(name) {
            Ok(Some(Self::LevelSensor(Box::new(LevelSensor::from_binary(
                data,
            )?))))
        } else if LifeCondition::path_matches(name) {
            Ok(Some(Self::LifeCondition(Box::new(
                LifeCondition::from_binary(data)?,
            ))))
        } else if Location::path_matches(name) {
            Ok(Some(Self::Location(Box::new(Location::from_binary(data)?))))
        } else if Lod::path_matches(name) {
            Ok(Some(Self::Lod(Box::new(Lod::from_binary(data)?))))
        } else if MainStatic::path_matches(name) {
            Ok(Some(Self::MainStatic(Box::new(MainStatic::from_binary(data)?))))
        } else if MapUnit::path_matches(name) {
            Ok(Some(Self::MapUnit(Box::new(MapUnit::from_binary(data)?))))
        } else if MessagePack::path_matches(name) {
            Ok(Some(Self::MessagePack(Box::new(MessagePack::from_binary(
                data,
            )?))))
        } else if ModelList::path_matches(name) {
            Ok(Some(Self::ModelList(Box::new(ModelList::from_binary(
                data,
            )?))))
        } else if Physics::path_matches(name) {
            Ok(Some(Self::Physics(Box::new(Physics::from_binary(data)?))))
        } else if QuestProduct::path_matches(name) {
            Ok(Some(Self::QuestProduct(Box::new(
                QuestProduct::from_binary(data)?,
            ))))
        } else if RagdollBlendWeight::path_matches(name) {
            Ok(Some(Self::RagdollBlendWeight(Box::new(
                RagdollBlendWeight::from_binary(data)?,
            ))))
        } else if RagdollConfig::path_matches(name) {
            Ok(Some(Self::RagdollConfig(Box::new(
                RagdollConfig::from_binary(data)?,
            ))))
        } else if RagdollConfigList::path_matches(name) {
            Ok(Some(Self::RagdollConfigList(Box::new(
                RagdollConfigList::from_binary(data)?,
            ))))
        } else if Recipe::path_matches(name) {
            Ok(Some(Self::Recipe(Box::new(Recipe::from_binary(data)?))))
        } else if ResidentActors::path_matches(name) {
            Ok(Some(Self::ResidentActors(Box::new(
                ResidentActors::from_binary(data)?,
            ))))
        } else if ResidentEvents::path_matches(name) {
            Ok(Some(Self::ResidentEvents(Box::new(
                ResidentEvents::from_binary(data)?,
            ))))
        } else if SaveDataPack::path_matches(name) {
            Ok(Some(Self::SaveDataPack(Box::new(
                SaveDataPack::from_binary(data)?,
            ))))
        } else if ShopData::path_matches(name) {
            Ok(Some(Self::ShopData(Box::new(ShopData::from_binary(data)?))))
        } else if ShopGameDataInfo::path_matches(name) {
            Ok(Some(Self::ShopGameDataInfo(Box::new(
                ShopGameDataInfo::from_binary(data)?,
            ))))
        } else if Static::path_matches(name) {
            Ok(Some(Self::Static(Box::new(Static::from_binary(data)?))))
        } else if StatusEffectList::path_matches(name) {
            Ok(Some(Self::StatusEffectList(Box::new(
                StatusEffectList::from_binary(data)?,
            ))))
        } else if Tips::path_matches(name) {
            Ok(Some(Self::Tips(Box::new(Tips::from_binary(data)?))))
        } else if UMii::path_matches(name) {
            Ok(Some(Self::UMii(Box::new(UMii::from_binary(data)?))))
        } else if WorldInfo::path_matches(name) {
            Ok(Some(Self::WorldInfo(Box::new(WorldInfo::from_binary(
                data,
            )?))))
        } else if data.len() > 4 && &data[0..4] == b"AAMP" {
            Ok(Some(Self::GenericAamp(Box::new(
                ParameterIO::from_binary(data)?,
            ))))
        } else if data.len() > 2 && matches!(&data[..2], b"BY" | b"YB") {
            Ok(Some(Self::GenericByml(Box::new(Byml::from_binary(data)?))))
        } else {
            Ok(None)
        };
        match result {
            Err(e) => {
                Ok(Some(Self::BinaryOverride(Box::new((
                    data.to_vec(),
                    e.to_string().into(),
                )))))
            }
            ok => ok,
        }
    }

    pub fn into_binary(self, endian: Endian) -> Vec<u8> {
        match self {
            // Self::Actor(v) => v.into_binary(endian),
            Self::ActorInfo(v) => v.into_binary(endian),
            Self::ActorLink(v) => v.into_binary(endian),
            Self::AIProgram(v) => v.into_binary(endian),
            Self::AISchedule(v) => v.into_binary(endian),
            Self::AnimationInfo(v) => v.into_binary(endian),
            Self::AreaData(v) => v.into_binary(endian),
            Self::AS(v) => v.into_binary(endian),
            Self::ASList(v) => v.into_binary(endian),
            Self::AttClient(v) => v.into_binary(endian),
            Self::AttClientList(v) => v.into_binary(endian),
            Self::Awareness(v) => v.into_binary(endian),
            Self::BarslistInfo(v) => v.into_binary(endian),
            Self::BoneControl(v) => v.into_binary(endian),
            Self::Chemical(v) => v.into_binary(endian),
            Self::ChemicalRes(v) => v.into_binary(endian),
            Self::CookData(v) => v.into_binary(endian),
            Self::DamageParam(v) => v.into_binary(endian),
            Self::Demo(v) => v.into_binary(endian),
            Self::DropTable(v) => v.into_binary(endian),
            Self::EventInfo(v) => v.into_binary(endian),
            Self::FontArchive(v) => v.into_binary(endian),
            Self::GameDataPack(v) => v.into_binary(endian),
            Self::GeneralParamList(v) => v.into_binary(endian),
            Self::LayoutArchive(v) => v.into_binary(endian),
            Self::LazyTraverseList(v) => v.into_binary(endian),
            Self::LevelSensor(v) => v.into_binary(endian),
            Self::LifeCondition(v) => v.into_binary(endian),
            Self::Location(v) => v.into_binary(endian),
            Self::Lod(v) => v.into_binary(endian),
            Self::MainStatic(v) => v.into_binary(endian),
            Self::MapUnit(v) => v.into_binary(endian),
            Self::MessagePack(v) => v.into_binary(endian),
            Self::ModelList(v) => v.into_binary(endian),
            Self::Physics(v) => v.into_binary(endian),
            Self::QuestProduct(v) => v.into_binary(endian),
            Self::RagdollBlendWeight(v) => v.into_binary(endian),
            Self::RagdollConfig(v) => v.into_binary(endian),
            Self::RagdollConfigList(v) => v.into_binary(endian),
            Self::Recipe(v) => v.into_binary(endian),
            Self::ResidentActors(v) => v.into_binary(endian),
            Self::ResidentEvents(v) => v.into_binary(endian),
            Self::SaveDataPack(v) => v.into_binary(endian),
            Self::ShopData(v) => v.into_binary(endian),
            Self::ShopGameDataInfo(v) => v.into_binary(endian),
            Self::Static(v) => v.into_binary(endian),
            Self::StatusEffectList(v) => v.into_binary(endian),
            Self::Tips(v) => v.into_binary(endian),
            Self::UMii(v) => v.into_binary(endian),
            Self::WorldInfo(v) => v.into_binary(endian),
            Self::GenericAamp(v) => v.to_binary(),
            Self::GenericByml(v) => v.to_binary(endian.into()),
            Self::BinaryOverride(v) => {
                let (bin, _) = *v;
                bin
            }
        }
    }
}

pub trait ResourceRegister {
    fn contains_resource(&self, canon: &str) -> bool;
    fn add_resource(&self, canon: &str, resource: ResourceData) -> Result<()>;
}

impl ResourceRegister for std::cell::RefCell<BTreeMap<String, ResourceData>> {
    fn contains_resource(&self, canon: &str) -> bool {
        self.borrow().contains_key(canon)
    }

    fn add_resource(&self, canon: &str, resource: ResourceData) -> Result<()> {
        self.borrow_mut().insert(canon.into(), resource);
        Ok(())
    }
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub struct SarcMap {
    pub alignment: usize,
    pub files:     SortedDeleteSet<String>,
}

impl Mergeable for SarcMap {
    fn diff(&self, other: &Self) -> Self {
        Self {
            alignment: self.alignment,
            files:     self.files.diff(&other.files),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            alignment: self.alignment,
            files:     self.files.merge(&diff.files),
        }
    }
}

impl SarcMap {
    pub fn from_binary(data: impl AsRef<[u8]>) -> Result<Self> {
        let sarc = Sarc::new(data.as_ref())?;
        let sarc_map = Self {
            alignment: sarc.guess_min_alignment(),
            files:     sarc
                .files()
                .map(|file| -> Result<String> {
                    Ok(file.name().context("SARC file missing name")?.into())
                })
                .collect::<Result<_>>()?,
        };
        Ok(sarc_map)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ResourceData {
    Binary(Vec<u8>),
    Mergeable(MergeableResource),
    Sarc(SarcMap),
}

impl From<Vec<u8>> for ResourceData {
    fn from(data: Vec<u8>) -> Self {
        Self::Binary(data)
    }
}

pub const EXCLUDE_EXTS: &[&str] = &["genvb", "sarc", "arc"];
pub const EXCLUDE_NAMES: &[&str] = &["tera_resource.Nin_NX_NVN", "tera_resource.Cafe_Cafe_GX2"];

pub fn is_mergeable_sarc(name: impl AsRef<Path>, data: impl AsRef<[u8]>) -> bool {
    fn inner(name: &Path, data: &[u8]) -> bool {
        static MAGIC: &[u8; 4] = b"SARC";
        data.len() >= 0x40
            && (&data[..4] == MAGIC || &data[0x11..0x15] == MAGIC)
            && name
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| !EXCLUDE_EXTS.contains(&e.strip_prefix('s').unwrap_or(e)))
                .unwrap_or(false)
            && name
                .file_stem()
                .and_then(|n| n.to_str())
                .map(|n| !EXCLUDE_NAMES.iter().any(|xn| n.starts_with(xn)))
                .unwrap_or(false)
    }
    inner(name.as_ref(), data.as_ref())
}

impl ResourceData {
    pub fn from_binary<'a>(name: impl AsRef<Path>, data: impl Into<Cow<'a, [u8]>>) -> Result<Self> {
        fn inner(name: &Path, data: Cow<'_, [u8]>) -> Result<ResourceData> {
            let stem = name
                .file_stem()
                .unwrap_or_default()
                .to_str()
                .unwrap_or_default();
            if stem == "Dummy" || data.len() < 0x10 {
                return Ok(ResourceData::Binary(data.into()));
            }
            if let Some(mergeable) = MergeableResource::from_binary(name, &data)
                .with_context(|| format!("Failed to parse resource {}", name.display()))?
            {
                Ok(ResourceData::Mergeable(mergeable))
            } else if is_mergeable_sarc(name, &data) {
                Ok(ResourceData::Sarc(SarcMap::from_binary(data)?))
            } else {
                Ok(ResourceData::Binary(data.to_vec()))
            }
        }
        inner(name.as_ref(), data.into())
    }

    #[inline]
    pub fn take_mergeable(self) -> Option<MergeableResource> {
        match self {
            ResourceData::Mergeable(resource) => Some(resource),
            _ => None,
        }
    }

    #[inline]
    pub fn as_mergeable(&self) -> Option<&MergeableResource> {
        match self {
            ResourceData::Mergeable(resource) => Some(resource),
            _ => None,
        }
    }

    #[inline]
    pub fn take_binary(self) -> Option<Vec<u8>> {
        match self {
            ResourceData::Binary(data) => Some(data),
            _ => None,
        }
    }

    #[inline]
    pub fn as_binary(&self) -> Option<&[u8]> {
        match self {
            ResourceData::Binary(data) => Some(data),
            _ => None,
        }
    }

    #[inline]
    pub fn take_sarc(self) -> Option<SarcMap> {
        match self {
            ResourceData::Sarc(sarc) => Some(sarc),
            _ => None,
        }
    }

    #[inline]
    pub fn as_sarc(&self) -> Option<&SarcMap> {
        match self {
            ResourceData::Sarc(sarc) => Some(sarc),
            _ => None,
        }
    }
}
