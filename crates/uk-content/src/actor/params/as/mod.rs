use std::sync::LazyLock;
use anyhow::{anyhow, Context};
use roead::aamp::{Name, ParameterList};
use serde::{Deserialize, Serialize};
use ext_bit_index::BitIndex;
use ext_blender_bone::BlenderBone;
use ext_float_array::FloatArray;
use ext_frame_ctrl::FrameCtrl;
use ext_hold_events::HoldEvents;
use ext_int_array::IntArray;
use ext_ranges::Ranges;
use ext_string_array::StringArray;
use ext_trigger_events::TriggerEvents;
use ext_type::ExtType;
use res::Resource;
//use res_asset::AssetResource;
use res_asset_ex::AssetExResource;
use res_blender::BlenderResource;
use res_children::ResourceWithChildren;
use res_selector::SelectorResource;
use res_seq_play::SequencePlayContainerResource;
use res_skel_asset::SkeletalAssetResource;
use res_type::ResType;
use crate::prelude::Mergeable;
use crate::util::HashMap;
use crate::{UKError, Result};

pub(crate) mod anim_seq;
mod ext_bit_index;
mod ext_blender_bone;
mod ext_float_array;
mod ext_frame_ctrl;
mod ext_hold_events;
mod ext_int_array;
mod ext_ranges;
mod ext_string_array;
mod ext_trigger_events;
mod res;
mod res_asset;
mod res_asset_ex;
mod res_blender;
mod res_children;
mod res_selector;
mod res_seq_play;
mod res_skel_asset;
mod res_type;
mod traverser;
mod ext_type;

pub(crate) fn get_child_index(hash: u32) -> Result<i32> {
    static CHILD_HASHES: LazyLock<HashMap<u32, i32>> = LazyLock::new(||
        minicbor_ser::from_slice(include_bytes!("../../../../data/child_hashes.bin"))
            .expect("child_hashes should not be broken")
    );
    CHILD_HASHES.get(&hash).copied().ok_or(UKError::Other("Key not of Child# format or # is above 255"))
}

pub(crate) fn get_element_index(hash: u32) -> Result<i32> {
    static ELEMENT_HASHES: LazyLock<HashMap<u32, i32>> = LazyLock::new(||
        minicbor_ser::from_slice(include_bytes!("../../../../data/element_hashes.bin"))
            .expect("element_hashes should not be broken")
    );
    ELEMENT_HASHES.get(&hash).copied().ok_or(UKError::Other("Key not of Element# format or # is above 511"))
}

pub(crate) fn get_event_index(hash: u32) -> Result<i32> {
    static EVENT_HASHES: LazyLock<HashMap<u32, i32>> = LazyLock::new(||
        minicbor_ser::from_slice(include_bytes!("../../../../data/event_hashes.bin"))
            .expect("event_hashes should not be broken")
    );
    EVENT_HASHES.get(&hash).copied().ok_or(UKError::Other("Key not of Event# format or # is above 255"))
}

pub(crate) fn get_range_index(hash: u32) -> Result<i32> {
    static RANGE_HASHES: LazyLock<HashMap<u32, i32>> = LazyLock::new(||
        minicbor_ser::from_slice(include_bytes!("../../../../data/range_hashes.bin"))
            .expect("range_hashes should not be broken")
    );
    RANGE_HASHES.get(&hash).copied().ok_or(UKError::Other("Key not of Range# format or # is above 255"))
}

pub(crate) fn get_value_index(hash: u32) -> Result<i32> {
    static VALUE_HASHES: LazyLock<HashMap<u32, i32>> = LazyLock::new(||
        minicbor_ser::from_slice(include_bytes!("../../../../data/value_hashes.bin"))
            .expect("value_hashes should not be broken")
    );
    VALUE_HASHES.get(&hash).copied().ok_or(UKError::Other("Key not of Value# format or # is above 255"))
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum Element {
    Resource(Resource),
    ResourceWithChildren(ResourceWithChildren),
    SequencePlayContainer(SequencePlayContainerResource),
    Selector(SelectorResource),
    Blender(BlenderResource),
    //Asset(AssetResource),
    AssetEx(AssetExResource),
    SkeletalAsset(SkeletalAssetResource),
}

impl TryFrom<&ParameterList> for Element {
    type Error = UKError;

    fn try_from(value: &ParameterList) -> Result<Self> {
        let type_index = value.objects
            .get("Parameters")
            .context("Missing Parameters")?
            .get("TypeIndex")
            .context("Missing TypeIndex")?
            .as_i32()
            .context("TypeIndex not i32")?;
        match type_index.into() {
            ResType::AbsTemperatureBlender |
            ResType::BoneBlender |
            ResType::DiffAngleYBlender |
            ResType::DirectionAngleBlender |
            ResType::DistanceBlender |
            ResType::ForwardBentBlender |
            ResType::GroundNormalBlender |
            ResType::GroundNormalSideBlender |
            ResType::NoLoopStickAngleBlender |
            ResType::RightStickAngleBlender |
            ResType::RightStickValueBlender |
            ResType::RightStickXBlender |
            ResType::RightStickYBlender |
            ResType::SizeBlender |
            ResType::SpeedBlender |
            ResType::StickAngleBlender |
            ResType::StickValueBlender |
            ResType::StickXBlender |
            ResType::StickYBlender |
            ResType::StressBlender |
            ResType::TemperatureBlender |
            ResType::TiredBlender |
            ResType::UserAngle2Blender |
            ResType::UserAngleBlender |
            ResType::UserSpeedBlender |
            ResType::WallAngleBlender |
            ResType::WeightBlender |
            ResType::WindVelocityBlender |
            ResType::YSpeedBlender |
            ResType::ZEx00ExposureBlender => Ok(Element::Blender(
                value.try_into().context("Bas file has invalid Blender")?
            )),
            ResType::AbsTemperatureSelector |
            ResType::ArmorSelector |
            ResType::ArrowSelector |
            ResType::AttentionSelector |
            ResType::BoolSelector |
            ResType::ButtonSelector |
            ResType::ChargeSelector |
            ResType::ComboSelector |
            ResType::DiffAngleYSelector |
            ResType::DirectionAngleSelector |
            ResType::DistanceSelector |
            ResType::DungeonClearSelector |
            ResType::DungeonNumberSelector |
            ResType::EmotionSelector |
            ResType::EventFlagSelector |
            ResType::EyeSelector |
            ResType::EyebrowSelector |
            ResType::FaceEmotionSelector |
            ResType::FootBLLifeSelector |
            ResType::FootBRLifeSelector |
            ResType::FootFLLifeSelector |
            ResType::FootFRLifeSelector |
            ResType::ForwardBentSelector |
            ResType::GearSelector |
            ResType::GenerationSelector |
            ResType::GrabTypeSelector |
            ResType::GroundNormalSelector |
            ResType::GroundNormalSideSelector |
            ResType::MaskSelector |
            ResType::MouthSelector |
            ResType::NoLoopStickAngleSelector |
            ResType::NodePosSelector |
            ResType::PersonalitySelector |
            ResType::PostureSelector |
            ResType::PreASSelector |
            ResType::PreExclusionRandomSelector |
            ResType::RandomSelector |
            ResType::RideSelector |
            ResType::RightStickAngleSelector |
            ResType::RightStickValueSelector |
            ResType::RightStickXSelector |
            ResType::RightStickYSelector |
            ResType::SelfHeightSelector |
            ResType::SelfWeightSelector |
            ResType::SizeSelector |
            ResType::SpeedSelector |
            ResType::StickAngleSelector |
            ResType::StickValueSelector |
            ResType::StickXSelector |
            ResType::StickYSelector |
            ResType::StressSelector |
            ResType::TemperatureSelector |
            ResType::TimeSelector |
            ResType::TiredSelector |
            ResType::UseItemSelector |
            ResType::UserAngle2Selector |
            ResType::UserAngleSelector |
            ResType::UserSpeedSelector |
            ResType::VariationSelector |
            ResType::WallAngleSelector |
            ResType::WeaponDetailSelector |
            ResType::WeaponSelector |
            ResType::WeatherSelector |
            ResType::WeightSelector |
            ResType::YSpeedSelector |
            ResType::ZEx00ExposureSelector => Ok(Element::Selector(
                value.try_into().context("Bas file has invalid Selector")?
            )),
            ResType::BoneVisibilityAsset |
            ResType::MatVisibilityAsset |
            ResType::ShaderParamAsset |
            ResType::ShaderParamColorAsset |
            ResType::ShaderParamTexSRTAsset |
            ResType::TexturePatternAsset => Ok(Element::AssetEx(
                value.try_into().context("Bas file has invalid AssetEx")?
            )),
            ResType::ClearMatAnmAsset |
            ResType::NoAnmAsset => Ok(Element::Resource(
                value.try_into().context("Bas file has invalid Resource")?
            )),
            ResType::SequencePlayContainer => Ok(Element::SequencePlayContainer(
                value.try_into().context("Bas file has invalid SequencePlayContainer")?
            )),
            ResType::SkeletalAsset => Ok(Element::SkeletalAsset(
                value.try_into().context("Bas file has invalid SkeletalAsset")?
            )),
            ResType::SyncPlayContainer => Ok(Element::ResourceWithChildren(
                value.try_into().context("Bas file has invalid ResourceWithChildren")?
            )),
            ResType::Invalid => Err(UKError::Any(anyhow!(
                "Bas file has invalid Element (TypeIndex: {})",
                type_index
            ))),
        }
    }
}

impl From<Element> for ParameterList {
    fn from(value: Element) -> Self {
        match value {
            Element::Resource(e) => e.into(),
            Element::ResourceWithChildren(e) => e.into(),
            Element::SequencePlayContainer(e) => e.into(),
            Element::Selector(e) => e.into(),
            Element::Blender(e) => e.into(),
            Element::AssetEx(e) => e.into(),
            Element::SkeletalAsset(e) => e.into(),
        }
    }
}

impl Element {
    pub fn children(&self) -> Box<dyn Iterator<Item = &i32> + '_> {
        match self {
            Element::Selector(e) => e.children(),
            Element::Blender(e) => e.children(),
            Element::SequencePlayContainer(e) => e.children(),
            Element::Resource(_) => Box::new(std::iter::empty::<&i32>()),
            Element::ResourceWithChildren(e) => Box::new(e.children.values()),
            Element::AssetEx(_) => Box::new(std::iter::empty::<&i32>()),
            Element::SkeletalAsset(_) => Box::new(std::iter::empty::<&i32>()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum Extension {
    FrameCtrl(FrameCtrl),
    TriggerEvents(TriggerEvents),
    HoldEvents(HoldEvents),
    StringArray(StringArray),
    Ranges(Ranges),
    FloatArray(FloatArray),
    IntArray(IntArray),
    BitIndex(BitIndex),
    BlenderBone(BlenderBone),
}

impl TryFrom<(&Name, &ParameterList)> for Extension {
    type Error = UKError;

    fn try_from(value: (&Name, &ParameterList)) -> Result<Self> {
        let (n, l) = value;
        match n.hash() {
            4007221886 => Ok(Extension::FrameCtrl(l.try_into().context("Extension has invalid FrameCtrl")?)),
            679723989 => Ok(Extension::TriggerEvents(l.try_into().context("Extension has invalid TriggerEvents")?)),
            4033433482 => Ok(Extension::HoldEvents(l.try_into().context("Extension has invalid HoldEvents")?)),
            203374876 => Ok(Extension::StringArray(l.try_into().context("Extension has invalid StringArray")?)),
            322024531 => Ok(Extension::Ranges(l.try_into().context("Extension has invalid Ranges")?)),
            3627016478 => Ok(Extension::FloatArray(l.try_into().context("Extension has invalid FloatArray")?)),
            3190114414 => Ok(Extension::IntArray(l.try_into().context("Extension has invalid IntArray")?)),
            127394560 => Ok(Extension::BitIndex(l.try_into().context("Extension has invalid BitIndex")?)),
            3977185723 => Ok(Extension::BlenderBone(l.try_into().context("Extension has invalid BlenderBone")?)),
            _ => Err(UKError::Any(anyhow!(
                "Extension has invalid key: {}, hash: {}",
                n,
                n.hash()
            ))),
        }
    }
}

impl From<Extension> for (Name, ParameterList) {
    fn from(value: Extension) -> Self {
        match value {
            Extension::FrameCtrl(e) => (Name::from_str("FrameCtrl"), e.into()),
            Extension::TriggerEvents(e) => (Name::from_str("TriggerEvents"), e.into()),
            Extension::HoldEvents(e) => (Name::from_str("HoldEvents"), e.into()),
            Extension::StringArray(e) => (Name::from_str("StringArray"), e.into()),
            Extension::Ranges(e) => (Name::from_str("Ranges"), e.into()),
            Extension::FloatArray(e) => (Name::from_str("FloatArray"), e.into()),
            Extension::IntArray(e) => (Name::from_str("IntArray"), e.into()),
            Extension::BitIndex(e) => (Name::from_str("BitIndex"), e.into()),
            Extension::BlenderBone(e) => (Name::from_str("BlenderBone"), e.into()),
        }
    }
}

impl Mergeable for Extension {
    fn diff(&self, other: &Self) -> Self {
        match (self, other) {
            (Extension::FrameCtrl(a), Extension::FrameCtrl(b)) =>
                Extension::FrameCtrl(a.diff(b)),
            (Extension::TriggerEvents(a), Extension::TriggerEvents(b)) =>
                Extension::TriggerEvents(a.diff(b)),
            (Extension::HoldEvents(a), Extension::HoldEvents(b)) =>
                Extension::HoldEvents(a.diff(b)),
            (Extension::StringArray(a), Extension::StringArray(b)) =>
                Extension::StringArray(a.diff(b)),
            (Extension::Ranges(a), Extension::Ranges(b)) =>
                Extension::Ranges(a.diff(b)),
            (Extension::FloatArray(a), Extension::FloatArray(b)) =>
                Extension::FloatArray(a.diff(b)),
            (Extension::IntArray(a), Extension::IntArray(b)) =>
                Extension::IntArray(a.diff(b)),
            (Extension::BitIndex(a), Extension::BitIndex(b)) =>
                Extension::BitIndex(a.diff(b)),
            (Extension::BlenderBone(a), Extension::BlenderBone(b)) =>
                Extension::BlenderBone(a.diff(b)),
            _ => panic!("Attempted to diff invalid Extensions!"),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        match (self, diff) {
            (Extension::FrameCtrl(a), Extension::FrameCtrl(b)) =>
                Extension::FrameCtrl(a.merge(b)),
            (Extension::TriggerEvents(a), Extension::TriggerEvents(b)) =>
                Extension::TriggerEvents(a.merge(b)),
            (Extension::HoldEvents(a), Extension::HoldEvents(b)) =>
                Extension::HoldEvents(a.merge(b)),
            (Extension::StringArray(a), Extension::StringArray(b)) =>
                Extension::StringArray(a.merge(b)),
            (Extension::Ranges(a), Extension::Ranges(b)) =>
                Extension::Ranges(a.merge(b)),
            (Extension::FloatArray(a), Extension::FloatArray(b)) =>
                Extension::FloatArray(a.merge(b)),
            (Extension::IntArray(a), Extension::IntArray(b)) =>
                Extension::IntArray(a.merge(b)),
            (Extension::BitIndex(a), Extension::BitIndex(b)) =>
                Extension::BitIndex(a.merge(b)),
            (Extension::BlenderBone(a), Extension::BlenderBone(b)) =>
                Extension::BlenderBone(a.merge(b)),
            _ => panic!("Attempted to merge invalid Extensions!"),
        }
    }
}
