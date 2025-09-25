use std::sync::LazyLock;
use anyhow::{anyhow, Context, Error, Result};
use roead::aamp::{Name, ParameterList};
use serde::{Deserialize, Serialize};
use bit_index::BitIndex;
use blender_bone::BlenderBone;
use float_array::FloatArray;
use frame_ctrl::FrameCtrl;
use hold_events::HoldEvents;
use int_array::IntArray;
use ranges::Ranges;
use res::Resource;
//use res_asset::AssetResource;
use res_asset_ex::AssetExResource;
use res_blender::BlenderResource;
use res_children::ResourceWithChildren;
use res_selector::SelectorResource;
use res_seq_play::SequencePlayContainerResource;
use res_skel_asset::SkeletalAssetResource;
use string_array::StringArray;
use trigger_events::TriggerEvents;
use crate::util::HashMap;

mod anim_seq;
mod bit_index;
mod blender_bone;
mod float_array;
mod frame_ctrl;
mod hold_events;
mod int_array;
mod ranges;
mod res;
mod res_asset;
mod res_asset_ex;
mod res_blender;
mod res_children;
mod res_selector;
mod res_seq_play;
mod res_skel_asset;
mod string_array;
mod trigger_events;
mod traverser;

pub(crate) fn get_child_index(hash: u32) -> Result<i32> {
    const CHILD_HASHES: LazyLock<HashMap<u32, i32>> = LazyLock::new(||
        minicbor_ser::from_slice(include_bytes!("../../../../data/child_hashes.bin")).unwrap()
    );
    CHILD_HASHES
        .get(&hash)
        .map(|s| *s)
        .ok_or(anyhow!("Invalid Child hash"))
}

pub(crate) fn get_element_index(hash: u32) -> Result<i32> {
    const ELEMENT_HASHES: LazyLock<HashMap<u32, i32>> = LazyLock::new(||
        minicbor_ser::from_slice(include_bytes!("../../../../data/element_hashes.bin")).unwrap()
    );
    ELEMENT_HASHES
        .get(&hash)
        .map(|s| *s)
        .ok_or(anyhow!("Invalid Element hash"))
}

pub(crate) fn get_event_index(hash: u32) -> Result<i32> {
    const EVENT_HASHES: LazyLock<HashMap<u32, i32>> = LazyLock::new(||
        minicbor_ser::from_slice(include_bytes!("../../../../data/event_hashes.bin")).unwrap()
    );
    EVENT_HASHES
        .get(&hash)
        .map(|s| *s)
        .ok_or(anyhow!("Invalid Event hash"))
}

pub(crate) fn get_range_index(hash: u32) -> Result<i32> {
    const RANGE_HASHES: LazyLock<HashMap<u32, i32>> = LazyLock::new(||
        minicbor_ser::from_slice(include_bytes!("../../../../data/range_hashes.bin")).unwrap()
    );
    RANGE_HASHES
        .get(&hash)
        .map(|s| *s)
        .ok_or(anyhow!("Invalid Range hash"))
}

pub(crate) fn get_value_index(hash: u32) -> Result<i32> {
    const VALUE_HASHES: LazyLock<HashMap<u32, i32>> = LazyLock::new(||
        minicbor_ser::from_slice(include_bytes!("../../../../data/value_hashes.bin")).unwrap()
    );
    VALUE_HASHES
        .get(&hash)
        .map(|s| *s)
        .ok_or(anyhow!("Invalid Value hash"))
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
    type Error = Error;

    fn try_from(value: &ParameterList) -> Result<Self> {
        let type_index = value.objects
            .get("Parameters")
            .ok_or(anyhow!("Missing Parameters"))?
            .get("TypeIndex")
            .ok_or(anyhow!("Missing TypeIndex"))?
            .as_i32()
            .context("Invalid TypeIndex")?;
        match type_index {
            0|5|12|14|16|29|34|36|42|51|53|55|57|65|68|70|
            72|74|76|78|81|85|88|90|92|95|100|102|103|105 => Ok(Element::Blender(
                value.try_into().context("Invalid Blender")?
            )),
            1|2|3|4|7|8|9|11|13|15|17|18|19|20|21|22|23|
            24|25|26|27|28|30|31|32|33|35|37|38|40|43|44|
            45|46|47|48|49|50|52|54|56|58|59|60|66|69|71|
            73|75|77|79|82|84|86|87|89|91|93|94|96|97|98|
            99|101|104|106 => Ok(Element::Selector(
                value.try_into().context("Invalid Selector")?
            )),
            6|39|62|63|64|83 => Ok(Element::AssetEx(
                value.try_into().context("Invalid AssetEx")?
            )),
            10|41 => Ok(Element::Resource(
                value.try_into().context("Invalid Resource")?
            )),
            61 => Ok(Element::SequencePlayContainer(
                value.try_into().context("Invalid SequencePlayContainer")?
            )),
            67 => Ok(Element::SkeletalAsset(
                value.try_into().context("Invalid SkeletalAsset")?
            )),
            80 => Ok(Element::ResourceWithChildren(
                value.try_into().context("Invalid ResourceWithChildren")?
            )),
            _ => Err(anyhow!("Invalid Element TypeIndex {}", type_index)),
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
    type Error = Error;

    fn try_from(value: (&Name, &ParameterList)) -> Result<Self> {
        let (n, l) = value;
        match n.hash() {
            4007221886 => Ok(Extension::FrameCtrl(l.try_into().context("Invalid FrameCtrl")?)),
            679723989 => Ok(Extension::TriggerEvents(l.try_into().context("Invalid TriggerEvents")?)),
            4033433482 => Ok(Extension::HoldEvents(l.try_into().context("Invalid HoldEvents")?)),
            203374876 => Ok(Extension::StringArray(l.try_into().context("Invalid StringArray")?)),
            322024531 => Ok(Extension::Ranges(l.try_into().context("Invalid Ranges")?)),
            3627016478 => Ok(Extension::FloatArray(l.try_into().context("Invalid FloatArray")?)),
            3190114414 => Ok(Extension::IntArray(l.try_into().context("Invalid IntArray")?)),
            127394560 => Ok(Extension::BitIndex(l.try_into().context("Invalid BitIndex")?)),
            3977185723 => Ok(Extension::BlenderBone(l.try_into().context("Invalid BlenderBone")?)),
            _ => Err(anyhow!("Invalid Extend hash")),
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
