use anyhow::Context;
use roead::{objs, params, aamp::ParameterList, lists};
use serde::{Deserialize, Serialize};
use crate::prelude::Mergeable;
use crate::util::DeleteMap;
use crate::{UKError, Result};
use super::{ExtType, ResType, Extension};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Resource {
    type_index: Option<ResType>,
    extensions: Option<DeleteMap<ExtType, Extension>>,
}

impl TryFrom<&ParameterList> for Resource {
    type Error = UKError;

    fn try_from(value: &ParameterList) -> Result<Self> {
        Ok(Self {
            // These are fine because AnimSeq already verified these exist
            // in order to call this or one of the TryFrom parents
            type_index: Some(unsafe { value.objects
                .get("Parameters")
                .unwrap_unchecked()
                .get("TypeIndex")
                .unwrap_unchecked()
                .as_i32()
                .unwrap_unchecked()
                .into() }),
            extensions: value.lists
                .get("Extend")
                .map(|pl| pl.lists
                    .iter()
                    .map(|(k, v)| { Ok((k.try_into()?, (k, v).try_into()?)) })
                    .collect::<Result<_>>()
                )
                .transpose()
                .context("Resource has invalid Extend")?,
        })
    }
}

impl From<Resource> for ParameterList {
    fn from(value: Resource) -> Self {
        Self {
            objects: objs!(
                "Parameters" => params!(
                    "TypeIndex" => value.type_index
                        .expect("Resource TypeIndex should have been read on import")
                        .into(),
                )
            ),
            lists: value.extensions
                .map(|m| lists!(
                    "Extend" => ParameterList::new().with_lists(
                        m.into_iter().map(|(_, v)| v.into())
                    )
                ))
                .unwrap_or_default(),
        }
    }
}

impl Mergeable for Resource {
    #[allow(clippy::obfuscated_if_else)]
    fn diff(&self, other: &Self) -> Self {
        Self {
            type_index: other.type_index
                .ne(&self.type_index)
                .then_some(other.type_index)
                .unwrap_or_default(),
            extensions: other.extensions
                .as_ref()
                .map(|o| self.extensions
                    .as_ref()
                    .map(|s| s.diff(o))
                    .unwrap_or_else(|| o.clone())
                ),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            type_index: diff.type_index
                .or(self.type_index),
            extensions: diff.extensions
                .as_ref()
                .map(|d| self.extensions
                    .as_ref()
                    .map(|s| s.merge(d))
                    .unwrap_or_else(|| d.clone())
                ),
        }
    }
}
