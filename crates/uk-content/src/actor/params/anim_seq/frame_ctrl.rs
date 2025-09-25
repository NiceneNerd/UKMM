use anyhow::{anyhow, Context, Error, Result};
use roead::{objs, aamp::{ParameterList, ParameterObject}};
use serde::{Deserialize, Serialize};

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
    type Error = Error;

    fn try_from(value: &ParameterList) -> Result<Self> {
        let obj = value.objects
            .get("FrameCtrl0")
            .ok_or(anyhow!("Missing FrameCtrl0"))?;
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
