pub mod info;
pub mod params;
pub mod residents;

use crate::{prelude::*, util::DeleteMap, Result, UKError};
use roead::{
    aamp::ParameterIO,
    byml::Byml,
    sarc::{Sarc, SarcWriter},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum LinkTarget<T: Clone + Mergeable + ParameterResource> {
    Dummy,
    External(String),
    Included { path: String, params: T },
}

impl<T: Clone + Mergeable + ParameterResource> Default for LinkTarget<T> {
    fn default() -> Self {
        Self::Dummy
    }
}

impl<T: Clone + Mergeable + ParameterResource> Mergeable for LinkTarget<T> {
    fn diff(&self, other: &Self) -> Self {
        match (self, other) {
            (
                Self::Included { path, params },
                Self::Included {
                    path: path2,
                    params: params2,
                },
            ) => {
                if path == path2 {
                    Self::Included {
                        path: path.clone(),
                        params: params.diff(params2),
                    }
                } else {
                    other.clone()
                }
            }
            _ => other.clone(),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        match (self, diff) {
            (
                Self::Included { path, params },
                Self::Included {
                    path: path2,
                    params: params2,
                },
            ) => {
                if path == path2 {
                    Self::Included {
                        path: path.clone(),
                        params: params.merge(params2),
                    }
                } else {
                    diff.clone()
                }
            }
            _ => diff.clone(),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub struct Actor {
    pub name: String,
    pub link: params::link::ActorLink,
    pub ai_program: LinkTarget<params::aiprog::AIProgram>,
    pub ai_schedule: LinkTarget<params::aischedule::AISchedule>,
    pub as_list: LinkTarget<params::aslist::ASList>,
    pub as_files: DeleteMap<String, params::r#as::AS>,
    pub attention: LinkTarget<params::atcllist::AttClientList>,
    pub awareness: LinkTarget<params::aware::Awareness>,
    pub bone_control: LinkTarget<params::bonectrl::BoneControl>,
    pub chemical: LinkTarget<params::chemical::Chemical>,
    pub damage_param: LinkTarget<params::damage::DamageParam>,
    pub drop: LinkTarget<params::drop::DropTable>,
    pub gparam: LinkTarget<params::general::GeneralParamList>,
    pub life_condition: LinkTarget<params::life::LifeCondition>,
    pub lod: LinkTarget<params::lod::Lod>,
    pub model: LinkTarget<params::modellist::ModelList>,
    pub physics: LinkTarget<params::physics::Physics>,
    pub rg_blend_weight: LinkTarget<params::rgbw::RagdollBlendWeight>,
    pub rg_config_list: LinkTarget<params::rgconfiglist::RagdollConfigList>,
    pub rg_config_files: DeleteMap<String, params::rgconfig::RagdollConfig>,
    pub recipe: LinkTarget<params::recipe::Recipe>,
    pub shop: LinkTarget<params::shop::ShopData>,
    pub umii: LinkTarget<params::umii::UMii>,
    pub anim_info: LinkTarget<params::animinfo::AnimationInfo>,
    pub assets: crate::Assets,
}

macro_rules! extract_target {
    ($link:expr, $sarc:expr, $userkey:expr, $type:ty, $mtype:ty) => {{
        let _user = $link
            .targets
            .param($userkey)
            .ok_or(crate::UKError::MissingAampKey(concat!(
                "Actor link missing ",
                $userkey
            )))?
            .as_string()?;
        if _user == "Dummy" {
            LinkTarget::Dummy
        } else {
            if let Some((_path, _target)) = $sarc
                .iter()
                .position(|f| {
                    f.0.as_ref()
                        .map(|n| n == &<$type>::path(_user))
                        .unwrap_or(false)
                })
                .map(|i| -> Result<(String, $type)> {
                    let (_path, _data) = $sarc.swap_remove(i);
                    Ok((
                        _path.unwrap().to_owned(),
                        <$type>::try_from(&<$mtype>::from_binary(&_data)?)?,
                    ))
                })
                .transpose()?
            {
                LinkTarget::Included {
                    path: _path,
                    params: _target,
                }
            } else {
                LinkTarget::External(_user.to_owned())
            }
        }
    }};
}

impl Actor {
    pub fn from_sarc(sarc: &Sarc) -> Result<Self> {
        let mut sarc = sarc.clone().into_files();
        let link_file = sarc.swap_remove(
            sarc.iter()
                .position(|(f, _)| f.as_ref().map(|n| n.ends_with(".bxml")).unwrap_or_default())
                .ok_or(UKError::MissingSarcFile("Actor link"))?,
        );
        let actorlink: params::link::ActorLink =
            ParameterIO::from_binary(&link_file.1)?.try_into()?;
        Ok(Self {
            name: link_file.0.unwrap(),
            ai_program: extract_target!(
                actorlink,
                sarc,
                "AIProgramUser",
                params::aiprog::AIProgram,
                roead::aamp::ParameterIO
            ),
            ai_schedule: extract_target!(
                actorlink,
                sarc,
                "AIScheduleUser",
                params::aischedule::AISchedule,
                roead::byml::Byml
            ),
            as_list: extract_target!(
                actorlink,
                sarc,
                "ASUser",
                params::aslist::ASList,
                roead::aamp::ParameterIO
            ),
            attention: extract_target!(
                actorlink,
                sarc,
                "AttentionUser",
                params::atcllist::AttClientList,
                roead::aamp::ParameterIO
            ),
            awareness: extract_target!(
                actorlink,
                sarc,
                "AwarenessUser",
                params::aware::Awareness,
                roead::aamp::ParameterIO
            ),
            bone_control: extract_target!(
                actorlink,
                sarc,
                "BoneControlUser",
                params::bonectrl::BoneControl,
                roead::aamp::ParameterIO
            ),
            chemical: extract_target!(
                actorlink,
                sarc,
                "ChemicalUser",
                params::chemical::Chemical,
                roead::aamp::ParameterIO
            ),
            damage_param: extract_target!(
                actorlink,
                sarc,
                "DamageParamUser",
                params::damage::DamageParam,
                roead::aamp::ParameterIO
            ),
            drop: extract_target!(
                actorlink,
                sarc,
                "DropTableUser",
                params::drop::DropTable,
                roead::aamp::ParameterIO
            ),
            gparam: extract_target!(
                actorlink,
                sarc,
                "GParamUser",
                params::general::GeneralParamList,
                roead::aamp::ParameterIO
            ),
            life_condition: extract_target!(
                actorlink,
                sarc,
                "LifeConditionUser",
                params::life::LifeCondition,
                roead::aamp::ParameterIO
            ),
            lod: extract_target!(
                actorlink,
                sarc,
                "LODUser",
                params::lod::Lod,
                roead::aamp::ParameterIO
            ),
            model: extract_target!(
                actorlink,
                sarc,
                "ModelUser",
                params::modellist::ModelList,
                roead::aamp::ParameterIO
            ),
            physics: extract_target!(
                actorlink,
                sarc,
                "PhysicsUser",
                params::physics::Physics,
                roead::aamp::ParameterIO
            ),
            rg_blend_weight: extract_target!(
                actorlink,
                sarc,
                "RgBlendWeightUser",
                params::rgbw::RagdollBlendWeight,
                roead::aamp::ParameterIO
            ),
            rg_config_list: extract_target!(
                actorlink,
                sarc,
                "RgConfigListUser",
                params::rgconfiglist::RagdollConfigList,
                roead::aamp::ParameterIO
            ),
            recipe: extract_target!(
                actorlink,
                sarc,
                "RecipeUser",
                params::recipe::Recipe,
                roead::aamp::ParameterIO
            ),
            shop: extract_target!(
                actorlink,
                sarc,
                "ShopDataUser",
                params::shop::ShopData,
                roead::aamp::ParameterIO
            ),
            umii: extract_target!(
                actorlink,
                sarc,
                "UMiiUser",
                params::umii::UMii,
                roead::aamp::ParameterIO
            ),
            anim_info: extract_target!(
                actorlink,
                sarc,
                "AnimationInfo",
                params::animinfo::AnimationInfo,
                roead::byml::Byml
            ),
            link: actorlink,
            as_files: sarc
                .drain_filter(|(f, _)| f.as_ref().map(|n| n.ends_with(".bas")).unwrap_or(false))
                .map(|(f, d)| -> Result<(String, params::r#as::AS)> {
                    Ok((
                        f.unwrap(),
                        params::r#as::AS::try_from(&roead::aamp::ParameterIO::from_binary(&d)?)?,
                    ))
                })
                .collect::<Result<_>>()?,
            rg_config_files: sarc
                .drain_filter(|(f, _)| {
                    f.as_ref()
                        .map(|n| n.ends_with(".brgconfig"))
                        .unwrap_or(false)
                })
                .map(
                    |(f, d)| -> Result<(String, params::rgconfig::RagdollConfig)> {
                        Ok((
                            f.unwrap(),
                            params::rgconfig::RagdollConfig::try_from(
                                &roead::aamp::ParameterIO::from_binary(&d)?,
                            )?,
                        ))
                    },
                )
                .collect::<Result<_>>()?,
            assets: sarc
                .into_iter()
                .filter_map(|(f, d)| f.map(|f| (f, d)))
                .collect(),
        })
    }

    pub fn into_sarc(self, endian: Endian) -> SarcWriter {
        let mut sarc = SarcWriter::new(endian.into());
        sarc.add_file(
            &params::link::ActorLink::path(&self.name),
            ParameterIO::from(self.link).to_binary(),
        );
        sarc.add_files(self.assets.into_iter());
        sarc
    }
}

impl Mergeable for Actor {
    fn diff(&self, other: &Self) -> Self {
        Self {
            name: self.name.clone(),
            link: self.link.diff(&other.link),
            ai_program: self.ai_program.diff(&other.ai_program),
            ai_schedule: self.ai_schedule.diff(&other.ai_schedule),
            as_list: self.as_list.diff(&other.as_list),
            attention: self.attention.diff(&other.attention),
            awareness: self.awareness.diff(&other.awareness),
            bone_control: self.bone_control.diff(&other.bone_control),
            chemical: self.chemical.diff(&other.chemical),
            damage_param: self.damage_param.diff(&other.damage_param),
            drop: self.drop.diff(&other.drop),
            gparam: self.gparam.diff(&other.gparam),
            life_condition: self.life_condition.diff(&other.life_condition),
            lod: self.lod.diff(&other.lod),
            model: self.model.diff(&other.model),
            physics: self.physics.diff(&other.physics),
            rg_blend_weight: self.rg_blend_weight.diff(&other.rg_blend_weight),
            rg_config_list: self.rg_config_list.diff(&other.rg_config_list),
            recipe: self.recipe.diff(&other.recipe),
            shop: self.shop.diff(&other.shop),
            umii: self.umii.diff(&other.umii),
            anim_info: self.anim_info.diff(&other.anim_info),
            as_files: self.as_files.deep_diff(&other.as_files),
            rg_config_files: self.rg_config_files.deep_diff(&other.rg_config_files),
            assets: self.assets.diff(&other.assets),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            name: self.name.clone(),
            link: self.link.merge(&diff.link),
            ai_program: self.ai_program.merge(&diff.ai_program),
            ai_schedule: self.ai_schedule.merge(&diff.ai_schedule),
            as_list: self.as_list.merge(&diff.as_list),
            attention: self.attention.merge(&diff.attention),
            awareness: self.awareness.merge(&diff.awareness),
            bone_control: self.bone_control.merge(&diff.bone_control),
            chemical: self.chemical.merge(&diff.chemical),
            damage_param: self.damage_param.merge(&diff.damage_param),
            drop: self.drop.merge(&diff.drop),
            gparam: self.gparam.merge(&diff.gparam),
            life_condition: self.life_condition.merge(&diff.life_condition),
            lod: self.lod.merge(&diff.lod),
            model: self.model.merge(&diff.model),
            physics: self.physics.merge(&diff.physics),
            rg_blend_weight: self.rg_blend_weight.merge(&diff.rg_blend_weight),
            rg_config_list: self.rg_config_list.merge(&diff.rg_config_list),
            recipe: self.recipe.merge(&diff.recipe),
            shop: self.shop.merge(&diff.shop),
            umii: self.umii.merge(&diff.umii),
            anim_info: self.anim_info.merge(&diff.anim_info),
            as_files: self.as_files.deep_merge(&diff.as_files),
            rg_config_files: self.rg_config_files.deep_merge(&diff.rg_config_files),
            assets: self.assets.merge(&diff.assets),
        }
    }
}

pub(crate) fn extract_info_param<T: TryFrom<roead::aamp::Parameter> + Into<Byml> + Clone>(
    obj: &roead::aamp::ParameterObject,
    key: &str,
) -> Result<Option<Byml>> {
    Ok(obj
        .param(key)
        .map(|v| -> Result<T> {
            v.clone()
                .try_into()
                .map_err(|_| crate::UKError::WrongAampType(roead::aamp::AampError::TypeError))
        })
        .transpose()?
        .map(|v| v.into()))
}

macro_rules! info_params {
    (
        $o: expr,
        $i: expr,
        {
            $(($k: expr, $v: expr, $t: ty)),* $(,)?
        }
    ) => {
        $i.extend(
            [
                $(
                    ($k, crate::actor::extract_info_param::<$t>($o, $v)?),
                )*
            ]
                .into_iter()
                .filter_map(|(k, v)| v.map(|v| (k.to_owned(), v))),
        );
    };
}

macro_rules! info_params_filtered {
    (
        $o: expr,
        $i: expr,
        {
            $(($k: expr, $v: expr, $t: ty)),* $(,)?
        }
    ) => {
        $i.extend(
            [
                $(
                    ($k, crate::actor::extract_info_param::<$t>($o, $v)?),
                )*
            ]
                .into_iter()
                .filter_map(|(k, v)| {
                    v.and_then(|v| (!crate::actor::is_byml_null(&v)).then(|| (k.to_owned(), v)))
                }),
        );
    };
}

pub(crate) fn is_byml_null(byml: &Byml) -> bool {
    match byml {
        Byml::Float(v) => *v == 0.0,
        Byml::Int(v) => *v == 0,
        Byml::String(v) => v.is_empty(),
        _ => true,
    }
}

pub(crate) use info_params;
pub(crate) use info_params_filtered;

pub trait InfoSource {
    fn update_info(&self, info: &mut roead::byml::Hash) -> Result<()>;
}

pub trait ParameterResource
where
    Self: std::marker::Sized,
{
    fn path(name: &str) -> String;
    fn from_binary(data: impl AsRef<[u8]>) -> Result<Self>;
    fn into_binary(self) -> Vec<u8>;
}
