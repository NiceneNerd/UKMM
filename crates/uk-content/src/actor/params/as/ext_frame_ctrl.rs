use anyhow::Context;
use roead::{objs, aamp::{ParameterList, ParameterObject}};
use serde::{Deserialize, Serialize};
use crate::prelude::Mergeable;
use crate::{UKError, Result};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FrameCtrl {
    rate: Option<f32>,
    start_frame: Option<f32>,
    end_frame: Option<f32>,
    loop_stop_count: Option<f32>,
    loop_stop_count_random: Option<f32>,
    reverse_play: Option<bool>,
    use_global_frame: Option<bool>,
    connect: Option<i32>,
    foot_type: Option<i32>,
    anm_loop: Option<i32>,
}

impl TryFrom<&ParameterList> for FrameCtrl {
    type Error = UKError;

    fn try_from(value: &ParameterList) -> Result<Self> {
        let obj = value.objects
            .get("FrameCtrl0")
            .ok_or(UKError::Other("AnimSeq Element FrameCtrl missing FrameCtrl0"))?;
        Ok(Self {
            rate: obj.get("Rate")
                .map(|p| p.as_f32().context("Invalid Rate"))
                .transpose()?,
            start_frame: obj.get("StartFrame")
                .map(|p| p.as_f32().context("Invalid StartFrame"))
                .transpose()?,
            end_frame: obj.get("EndFrame")
                .map(|p| p.as_f32().context("Invalid EndFrame"))
                .transpose()?,
            loop_stop_count: obj.get("LoopStopCount")
                .map(|p| p.as_f32().context("Invalid LoopStopCount"))
                .transpose()?,
            loop_stop_count_random: obj.get("LoopStopCountRandom")
                .map(|p| p.as_f32().context("Invalid LoopStopCountRandom"))
                .transpose()?,
            reverse_play: obj.get("ReversePlay")
                .map(|p| p.as_bool().context("Invalid ReversePlay"))
                .transpose()?,
            use_global_frame: obj.get("UseGlobalFrame")
                .map(|p| p.as_bool().context("Invalid UseGlobalFrame"))
                .transpose()?,
            connect: obj.get("Connect")
                .map(|p| p.as_i32().context("Invalid Connect"))
                .transpose()?,
            foot_type: obj.get("FootType")
                .map(|p| p.as_i32().context("Invalid FootType"))
                .transpose()?,
            anm_loop: obj.get("AnmLoop")
                .map(|p| p.as_i32().context("Invalid AnmLoop"))
                .transpose()?,
        })
    }
}

impl From<FrameCtrl> for ParameterList {
    fn from(value: FrameCtrl) -> Self {
        let mut params = ParameterObject::new();
        value.rate.into_iter()
            .for_each(|f| params.insert("Rate", f.into()));
        value.start_frame.into_iter()
            .for_each(|f| params.insert("StartFrame", f.into()));
        value.end_frame.into_iter()
            .for_each(|f| params.insert("EndFrame", f.into()));
        value.loop_stop_count.into_iter()
            .for_each(|f| params.insert("LoopStopCount", f.into()));
        value.loop_stop_count_random.into_iter()
            .for_each(|f| params.insert("LoopStopCountRandom", f.into()));
        value.reverse_play.into_iter()
            .for_each(|b| params.insert("ReversePlay", b.into()));
        value.use_global_frame.into_iter()
            .for_each(|b| params.insert("UseGlobalFrame", b.into()));
        value.connect.into_iter()
            .for_each(|i| params.insert("Connect", i.into()));
        value.foot_type.into_iter()
            .for_each(|i| params.insert("FootType", i.into()));
        value.anm_loop.into_iter()
            .for_each(|i| params.insert("AnmLoop", i.into()));
        Self {
            objects: objs!(
                "FrameCtrl0" => params,
            ),
            lists: Default::default(),
        }
    }
}

impl Mergeable for FrameCtrl {
    #[allow(clippy::obfuscated_if_else)]
    fn diff(&self, other: &Self) -> Self {
        Self {
            rate: other.rate
                .ne(&self.rate)
                .then_some(other.rate)
                .unwrap_or_default(),
            start_frame: other.start_frame
                .ne(&self.start_frame)
                .then_some(other.start_frame)
                .unwrap_or_default(),
            end_frame: other.end_frame
                .ne(&self.end_frame)
                .then_some(other.end_frame)
                .unwrap_or_default(),
            loop_stop_count: other.loop_stop_count
                .ne(&self.loop_stop_count)
                .then_some(other.loop_stop_count)
                .unwrap_or_default(),
            loop_stop_count_random: other.loop_stop_count_random
                .ne(&self.loop_stop_count_random)
                .then_some(other.loop_stop_count_random)
                .unwrap_or_default(),
            reverse_play: other.reverse_play
                .ne(&self.reverse_play)
                .then_some(other.reverse_play)
                .unwrap_or_default(),
            use_global_frame: other.use_global_frame
                .ne(&self.use_global_frame)
                .then_some(other.use_global_frame)
                .unwrap_or_default(),
            connect: other.connect
                .ne(&self.connect)
                .then_some(other.connect)
                .unwrap_or_default(),
            foot_type: other.foot_type
                .ne(&self.foot_type)
                .then_some(other.foot_type)
                .unwrap_or_default(),
            anm_loop: other.anm_loop
                .ne(&self.anm_loop)
                .then_some(other.anm_loop)
                .unwrap_or_default(),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            rate: diff.rate
                .or(self.rate),
            start_frame: diff.start_frame
                .or(self.start_frame),
            end_frame: diff.end_frame
                .or(self.end_frame),
            loop_stop_count: diff.loop_stop_count
                .or(self.loop_stop_count),
            loop_stop_count_random: diff.loop_stop_count_random
                .or(self.loop_stop_count_random),
            reverse_play: diff.reverse_play
                .or(self.reverse_play),
            use_global_frame: diff.use_global_frame
                .or(self.use_global_frame),
            connect: diff.connect
                .or(self.connect),
            foot_type: diff.foot_type
                .or(self.foot_type),
            anm_loop: diff.anm_loop
                .or(self.anm_loop),
        }
    }
}
