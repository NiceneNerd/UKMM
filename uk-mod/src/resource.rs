use uk_content::actor::info::ActorInfo;
use uk_content::actor::params::aiprog::AIProgram;
use uk_content::actor::params::aischedule::AISchedule;
use uk_content::actor::params::animinfo::AnimationInfo;
use uk_content::actor::params::aslist::ASList;
use uk_content::actor::params::atcl::AttClient;
use uk_content::actor::params::atcllist::AttClientList;
use uk_content::actor::params::aware::Awareness;
use uk_content::actor::params::bonectrl::BoneControl;
use uk_content::actor::params::chemical::Chemical;
use uk_content::actor::params::damage::DamageParam;
use uk_content::actor::params::drop::DropTable;
use uk_content::actor::params::general::GeneralParamList;
use uk_content::actor::params::life::LifeCondition;
use uk_content::actor::params::link::ActorLink;
use uk_content::actor::params::lod::Lod;
use uk_content::actor::params::modellist::ModelList;
use uk_content::actor::params::physics::Physics;
use uk_content::actor::params::r#as::AS;
use uk_content::actor::params::recipe::Recipe;
use uk_content::actor::params::rgbw::RagdollBlendWeight;
use uk_content::actor::params::rgconfig::RagdollConfig;
use uk_content::actor::params::rgconfiglist::RagdollConfigList;
use uk_content::actor::params::shop::ShopData;
use uk_content::actor::params::umii::UMii;
use uk_content::actor::residents::ResidentActors;
use uk_content::actor::Actor;
use uk_content::chemical::chmres::ChemicalRes;
use uk_content::cooking::data::CookData;
use uk_content::data::gamedata::GameDataPack;
use uk_content::data::savedata::SaveDataPack;
use uk_content::data::shop::ShopGameDataInfo;
use uk_content::demo::Demo;
use uk_content::eco::areadata::AreaData;
use uk_content::eco::level::LevelSensor;
use uk_content::eco::status::StatusEffectList;
use uk_content::event::info::EventInfo;
use uk_content::event::residents::ResidentEvents;
use uk_content::map::lazy::LazyTraverseList;
use uk_content::map::mainfield::location::Location;
use uk_content::map::static_::Static;
use uk_content::map::unit::MapUnit;
use uk_content::prelude::*;
use uk_content::quest::product::QuestProduct;
use uk_content::sound::barslist::BarslistInfo;
use uk_content::tips::Tips;
use uk_content::worldmgr::info::WorldInfo;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum MergeableResource {
    Actor(Box<Actor>),
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
    GameDataPack(Box<GameDataPack>),
    GeneralParamList(Box<GeneralParamList>),
    LazyTraverseList(Box<LazyTraverseList>),
    LevelSensor(Box<LevelSensor>),
    LifeCondition(Box<LifeCondition>),
    Location(Box<Location>),
    Lod(Box<Lod>),
    MapUnit(Box<MapUnit>),
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
}

impl Mergeable for MergeableResource {
    fn diff(&self, other: &Self) -> Self {
        match (self, other) {
            (Self::Actor(a), Self::Actor(b)) => Self::Actor(Box::new(a.diff(b))),
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
            (Self::GameDataPack(a), Self::GameDataPack(b)) => {
                Self::GameDataPack(Box::new(a.diff(b)))
            }
            (Self::GeneralParamList(a), Self::GeneralParamList(b)) => {
                Self::GeneralParamList(Box::new(a.diff(b)))
            }
            (Self::LazyTraverseList(a), Self::LazyTraverseList(b)) => {
                Self::LazyTraverseList(Box::new(a.diff(b)))
            }
            (Self::LevelSensor(a), Self::LevelSensor(b)) => Self::LevelSensor(Box::new(a.diff(b))),
            (Self::LifeCondition(a), Self::LifeCondition(b)) => {
                Self::LifeCondition(Box::new(a.diff(b)))
            }
            (Self::Location(a), Self::Location(b)) => Self::Location(Box::new(a.diff(b))),
            (Self::Lod(a), Self::Lod(b)) => Self::Lod(Box::new(a.diff(b))),
            (Self::MapUnit(a), Self::MapUnit(b)) => Self::MapUnit(Box::new(a.diff(b))),
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
            _ => panic!(
                "Tried to diff incompatible resources: {:?} and {:?}",
                &self, &other
            ),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        match (self, diff) {
            (Self::Actor(a), Self::Actor(b)) => Self::Actor(Box::new(a.merge(b))),
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
            (Self::GameDataPack(a), Self::GameDataPack(b)) => {
                Self::GameDataPack(Box::new(a.merge(b)))
            }
            (Self::GeneralParamList(a), Self::GeneralParamList(b)) => {
                Self::GeneralParamList(Box::new(a.merge(b)))
            }
            (Self::LazyTraverseList(a), Self::LazyTraverseList(b)) => {
                Self::LazyTraverseList(Box::new(a.merge(b)))
            }
            (Self::LevelSensor(a), Self::LevelSensor(b)) => Self::LevelSensor(Box::new(a.merge(b))),
            (Self::LifeCondition(a), Self::LifeCondition(b)) => {
                Self::LifeCondition(Box::new(a.merge(b)))
            }
            (Self::Location(a), Self::Location(b)) => Self::Location(Box::new(a.merge(b))),
            (Self::Lod(a), Self::Lod(b)) => Self::Lod(Box::new(a.merge(b))),
            (Self::MapUnit(a), Self::MapUnit(b)) => Self::MapUnit(Box::new(a.merge(b))),
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
            _ => panic!(
                "Tried to merge incompatible resources: {:?} and {:?}",
                &self, &diff
            ),
        }
    }
}

impl MergeableResource {
    pub fn into_binary(self, endian: Endian) -> Vec<u8> {
        match self {
            Self::Actor(v) => v.into_binary(endian),
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
            Self::GameDataPack(v) => v.into_binary(endian),
            Self::GeneralParamList(v) => v.into_binary(endian),
            Self::LazyTraverseList(v) => v.into_binary(endian),
            Self::LevelSensor(v) => v.into_binary(endian),
            Self::LifeCondition(v) => v.into_binary(endian),
            Self::Location(v) => v.into_binary(endian),
            Self::Lod(v) => v.into_binary(endian),
            Self::MapUnit(v) => v.into_binary(endian),
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
        }
    }
}
