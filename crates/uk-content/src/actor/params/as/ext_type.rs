use roead::aamp::Name;
use serde::{Deserialize, Serialize};
use crate::{UKError, Result};

#[derive(Debug, Clone, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum ExtType {
    BitIndex,
    BlenderBone,
    FloatArray,
    FrameCtrl,
    HoldEvents,
    IntArray,
    Ranges,
    StringArray,
    TriggerEvents,
}

impl TryFrom<&Name> for ExtType {
    type Error = UKError;

    fn try_from(value: &Name) -> Result<Self> {
        match value.hash() {
            4007221886 => Ok(Self::FrameCtrl),
            679723989 => Ok(Self::TriggerEvents),
            4033433482 => Ok(Self::HoldEvents),
            203374876 => Ok(Self::StringArray),
            322024531 => Ok(Self::Ranges),
            3627016478 => Ok(Self::FloatArray),
            3190114414 => Ok(Self::IntArray),
            127394560 => Ok(Self::BitIndex),
            3977185723 => Ok(Self::BlenderBone),
            _ => Err(UKError::Other("AnimSeq Element Extend contains invalid Extension key")),
        }
    }
}