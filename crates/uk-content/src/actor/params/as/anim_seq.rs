use anyhow::Context;
use join_str::jstr;
use roead::aamp::{Parameter, ParameterIO, ParameterList};
use serde::{Deserialize, Serialize};
use uk_util::OptionResultExt;
use crate::{
    prelude::{Endian, Mergeable, Resource},
    util::{SortedDeleteMap, DeleteMap},
    actor::ParameterResource,
    Result,
    UKError
};
use super::{Element, traverser::Traverser};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnimSeq {
    elements: SortedDeleteMap<i32, Element>,
    common_params: Option<DeleteMap<u32, Parameter>>
}

impl TryFrom<&ParameterIO> for AnimSeq {
    type Error = UKError;

    fn try_from(value: &ParameterIO) -> Result<AnimSeq> {
        let anim_seq = Self {
            elements: value.param_root
                .lists
                .get("Elements")
                .ok_or(UKError::MissingAampKey("Missing Elements", Box::from(None)))?
                .lists
                .iter()
                .map(|(n, l)| -> Result<(i32, Element)> {
                    Ok((super::get_element_index(n.hash())?, l.try_into()?))
                })
                .collect::<Result<_>>()?,
            common_params: value.param_root
                .objects
                .get("CommonParams")
                .map(|obj| {
                    obj.0
                        .iter()
                        .map(|(n, p)| (n.hash(), p.clone()))
                        .collect::<DeleteMap<u32, Parameter>>()
                })
        };
        if !anim_seq.elements.is_empty() {
            anim_seq.traverse()?;
        }
        Ok(anim_seq)
    }
}

impl From<AnimSeq> for ParameterIO {
    fn from(anim_seq: AnimSeq) -> Self {
        Self::new()
            .with_objects(anim_seq.common_params.into_iter().map(|m|
                ("CommonParams", m.into_iter().collect())
            ))
            .with_list(
                "Elements",
                ParameterList::new().with_lists(anim_seq.elements
                    .into_iter()
                    .map(|(i, e)| (format!("Element{}", i), e.into())))
            )
    }
}

impl Mergeable for AnimSeq {
    fn diff(&self, other: &Self) -> Self {
        other.clone()
    }

    fn merge(&self, diff: &Self) -> Self {
        diff.clone()
    }
}

impl AnimSeq {
    pub fn traverse(&self) -> anyhow::Result<()> {
        Traverser::new(&self.elements.values().collect()).traverse(0).context("Failed to traverse AnimSeq")
    }
}

impl ParameterResource for AnimSeq {
    fn path(name: &str) -> String {
        jstr!("Actor/AS/{name}.bas")
    }
}

impl Resource for AnimSeq {
    fn from_binary(data: impl AsRef<[u8]>) -> Result<Self> {
        (&ParameterIO::from_binary(data.as_ref())?).try_into()
    }

    fn into_binary(self, _endian: Endian) -> Vec<u8> {
        ParameterIO::from(self).to_binary()
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref()
            .extension()
            .and_then(|ext| ext.to_str())
            .contains(&"bas")
    }
}
