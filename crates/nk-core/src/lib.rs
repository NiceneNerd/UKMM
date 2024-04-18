use dyn_partial_eq::dyn_partial_eq;
use smartstring::alias::String;
use thiserror::Error;

#[repr(transparent)]
pub struct MergeableResource(Box<dyn Mergeable>);

#[dyn_partial_eq]
pub trait Mergeable {
    #[must_use]
    fn diff(&self, other: &MergeableResource) -> MergeableResource;
    #[must_use]
    fn merge(&self, diff: &MergeableResource) -> MergeableResource;
}

#[derive(Debug, Clone)]
pub enum ContextData {
    Parameter(roead::aamp::Parameter),
    List(roead::aamp::ParameterList),
    Object(roead::aamp::ParameterObject),
    Byml(roead::byml::Byml),
}

impl From<roead::aamp::Parameter> for ContextData {
    fn from(param: roead::aamp::Parameter) -> Self {
        ContextData::Parameter(param)
    }
}

impl From<&roead::aamp::Parameter> for ContextData {
    fn from(param: &roead::aamp::Parameter) -> Self {
        ContextData::Parameter(param.clone())
    }
}

impl From<roead::aamp::ParameterList> for ContextData {
    fn from(list: roead::aamp::ParameterList) -> Self {
        ContextData::List(list)
    }
}

impl From<roead::aamp::ParameterObject> for ContextData {
    fn from(obj: roead::aamp::ParameterObject) -> Self {
        ContextData::Object(obj)
    }
}

impl From<&roead::aamp::ParameterList> for ContextData {
    fn from(list: &roead::aamp::ParameterList) -> Self {
        ContextData::List(list.clone())
    }
}

impl From<&roead::aamp::ParameterObject> for ContextData {
    fn from(obj: &roead::aamp::ParameterObject) -> Self {
        ContextData::Object(obj.clone())
    }
}

impl From<roead::byml::Byml> for ContextData {
    fn from(by: roead::byml::Byml) -> Self {
        ContextData::Byml(by)
    }
}

impl From<&roead::byml::Byml> for ContextData {
    fn from(by: &roead::byml::Byml) -> Self {
        ContextData::Byml(by.clone())
    }
}

#[derive(Debug, Error)]
pub enum NKError {
    #[error("Parameter file missing key: {0}")]
    MissingAampKey(&'static str, Option<ContextData>),
    #[error("Parameter file missing key: {0}")]
    MissingAampKeyD(std::string::String),
    #[error("BYML file missing key: {0}")]
    MissingBymlKey(&'static str),
    #[error("BYML file missing key: {0}")]
    MissingBymlKeyD(std::string::String),
    #[error("Wrong type for BYML value: found {0}, expected {1}")]
    WrongBymlType(std::string::String, &'static str),
    #[error("{0} missing from SARC")]
    MissingSarcFile(&'static str),
    #[error("{0} missing from SARC")]
    MissingSarcFileD(std::string::String),
    #[error("Invalid weather value: {0}")]
    InvalidWeatherOrTime(std::string::String),
    #[error("Missing resource at {0}")]
    MissingResource(std::string::String),
    #[error("{0}")]
    Other(&'static str),
    #[error("{0}")]
    OtherD(std::string::String),
    #[error(transparent)]
    _Infallible(#[from] std::convert::Infallible),
    #[error(transparent)]
    RoeadError(#[from] roead::Error),
    #[error(transparent)]
    Any(#[from] anyhow::Error),
    #[error("Invalid BYML data for field {0}: {1:#?}")]
    InvalidByml(String, roead::byml::Byml),
    #[error("Invalid parameter data for field {0}: {1:#?}")]
    InvalidParameter(String, roead::aamp::Parameter),
}

impl NKError {
    pub fn context_data(&self) -> Option<ContextData> {
        match self {
            Self::MissingAampKey(_, data) => data.clone(),
            Self::InvalidByml(_, data) => Some(ContextData::Byml(data.clone())),
            Self::InvalidParameter(_, data) => Some(ContextData::Parameter(data.clone())),
            _ => None,
        }
    }
}

pub trait Game {
    fn content_prefix(&self) -> &str;
    fn dlc_prefix(&self) -> &str;
    fn canonicalize(&self, path: impl AsRef<std::path::Path>) -> String;
    fn update_rstb(
        &self,
        output: impl AsRef<std::path::Path>,
        updates: impl Iterator<Item = (String, Option<u32>)>,
    ) -> std::result::Result<(), NKError>;
}
