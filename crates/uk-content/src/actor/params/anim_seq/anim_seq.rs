use anyhow::{Context, Error, Result};
use roead::aamp::{Parameter, ParameterIO, ParameterList};
use serde::{Deserialize, Serialize};
use crate::actor::params::anim_seq::traverser::Traverser;
use crate::util::{SortedDeleteMap, DeleteMap};
use super::Element;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnimSeq {
    elements: SortedDeleteMap<i32, Element>,
    common_params: Option<DeleteMap<u32, Parameter>>
}

impl TryFrom<&ParameterIO> for AnimSeq {
    type Error = Error;

    fn try_from(value: &ParameterIO) -> Result<Self> {
        Ok(AnimSeq {
            elements: value.param_root
                .lists
                .get("Elements")
                .ok_or(anyhow::anyhow!("Missing Elements"))?
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
        })
    }
}

impl From<AnimSeq> for ParameterIO {
    fn from(anim_seq: AnimSeq) -> Self {
        ParameterIO::new()
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

impl AnimSeq {
    pub fn traverse(&self) -> Result<()> {
        Traverser::new(&self.elements.values().collect(), Default::default()).traverse(0).context("Failed to traverse anim")
    }
}
