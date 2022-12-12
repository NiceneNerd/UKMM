use std::collections::BTreeMap;

use join_str::jstr;
use roead::aamp::*;
use serde::{Deserialize, Serialize};
use uk_content_derive::ParamData;
use uk_ui_derive::Editable;

use crate::{actor::ParameterResource, prelude::*, util, Result, UKError};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Editable, ParamData)]
pub struct ElementParams {
    #[name = "TypeIndex"]
    type_index: i32,
    #[name = "NoSync"]
    no_sync: Option<bool>,
    #[name = "JudgeOnce"]
    judge_once: Option<bool>,
    #[name = "InputLimit"]
    input_limit: Option<f32>,
    #[name = "FileName"]
    file_name: Option<String64>,
    #[name = "Morph"]
    morph: Option<f32>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Editable)]
pub struct Element {
    pub params:   ElementParams,
    pub children: Option<BTreeMap<usize, Element>>,
    pub extend:   Option<ParameterList>,
}

impl Element {
    fn try_from_plist(list: &ParameterList, pio: &ParameterIO) -> Result<Self> {
        // This is sound because this function is never called until this list
        // is confirmed to exist.
        let element_list = unsafe { pio.list("Elements").unwrap_unchecked() };
        Ok(Self {
            params:   list
                .object("Parameters")
                .ok_or(UKError::MissingAampKey("AS node missing parameters", None))?
                .try_into()?,
            children: list
                .object("Children")
                .map(|children| -> Result<BTreeMap<usize, Element>> {
                    children
                        .0
                        .values()
                        .map(|idx| -> Result<(usize, Element)> {
                            let idx = idx.as_int()? as usize;
                            Ok((
                                idx,
                                Element::try_from_plist(
                                    element_list.lists.0.values().nth(idx).ok_or_else(|| {
                                        UKError::MissingAampKeyD(jstr!(
                                            "AS control node missing child at index \
                                             {&lexical::to_string(idx)}"
                                        ))
                                    })?,
                                    pio,
                                )?,
                            ))
                        })
                        .collect::<Result<_>>()
                })
                .transpose()?,
            extend:   list.list("Extend").cloned(),
        })
    }
}

impl Mergeable for Element {
    fn diff(&self, other: &Self) -> Self {
        Self {
            params:   other.params.clone(),
            children: other.children.as_ref().map(|other_children| {
                self.children
                    .as_ref()
                    .map(|self_children| {
                        other_children
                            .iter()
                            .filter_map(|(k, other_v)| {
                                if let Some(self_v) = self_children.get(k) {
                                    if self_v != other_v {
                                        Some((*k, self_v.diff(other_v)))
                                    } else {
                                        None
                                    }
                                } else {
                                    Some((*k, other_v.clone()))
                                }
                            })
                            .collect()
                    })
                    .unwrap_or_else(|| other_children.clone())
            }),
            extend:   other.extend.as_ref().map(|other_extend| {
                self.extend
                    .as_ref()
                    .map(|self_extend| util::diff_plist(self_extend, other_extend))
                    .unwrap_or_else(|| other_extend.clone())
            }),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            params:   diff.params.clone(),
            children: diff.children.as_ref().map(|diff_children| {
                self.children
                    .as_ref()
                    .map(|self_children| {
                        self_children
                            .iter()
                            .map(|(i, self_v)| {
                                if let Some(other_v) = diff_children.get(i) {
                                    (*i, self_v.merge(other_v))
                                } else {
                                    (*i, self_v.clone())
                                }
                            })
                            .collect()
                    })
                    .unwrap_or_else(|| diff_children.clone())
            }),
            extend:   diff.extend.as_ref().map(|diff_extend| {
                self.extend
                    .as_ref()
                    .map(|self_extend| util::merge_plist(self_extend, diff_extend))
                    .unwrap_or_else(|| diff_extend.clone())
            }),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Editable)]
pub struct AS(pub Option<Element>);

impl TryFrom<&ParameterIO> for AS {
    type Error = UKError;

    fn try_from(pio: &ParameterIO) -> Result<Self> {
        Ok(Self(
            pio.list("Elements")
                .ok_or(UKError::MissingAampKey("AS missing elements list", None))?
                .lists
                .0
                .values()
                .next()
                .map(|list| Element::try_from_plist(list, pio))
                .transpose()?,
        ))
    }
}

#[derive(Debug)]
struct ParameterIOBuilder {
    as_val: AS,
    done:   Vec<Element>,
}

impl ParameterIOBuilder {
    fn new(val: AS) -> Self {
        Self {
            as_val: val,
            done:   Vec::new(),
        }
    }

    fn add_element(&mut self, element: &Element, next: usize) -> (usize, Vec<ParameterList>) {
        if let Some(idx) = self.done.iter().position(|e| e == element) {
            (idx, vec![])
        } else {
            let idx = next;
            let mut child_lists: Vec<ParameterList> =
                Vec::with_capacity(element.children.as_ref().map(|cl| cl.len()).unwrap_or(1));
            self.done.push(element.clone());
            let list = ParameterList {
                objects: [("Parameters", element.params.clone().into())]
                    .into_iter()
                    .chain(element.children.iter().map(|children| {
                        (
                            "Children",
                            children
                                .iter()
                                .enumerate()
                                .map(|(count, (i, child))| {
                                    let (index, child_list) =
                                        self.add_element(child, idx + count + 1);
                                    child_lists.extend(child_list);
                                    (
                                        jstr!("Child{&lexical::to_string(index)}"),
                                        Parameter::Int((*i + 1) as i32),
                                    )
                                })
                                .collect(),
                        )
                    }))
                    .collect(),
                lists:   element
                    .extend
                    .iter()
                    .map(|extend| ("Extend", extend.clone()))
                    .collect(),
            };
            child_lists.insert(0, list);
            (idx, child_lists)
        }
    }

    fn build(mut self) -> ParameterIO {
        let root = std::mem::take(&mut self.as_val.0);
        ParameterIO::new().with_list(
            "Elements",
            root.map(|element| {
                let (_, elements) = self.add_element(&element, 0);
                ParameterList {
                    lists: elements
                        .into_iter()
                        .enumerate()
                        .map(|(i, list)| (jstr!("Element{&lexical::to_string(i)}"), list))
                        .collect(),
                    ..Default::default()
                }
            })
            .unwrap_or_default(),
        )
    }
}

impl From<AS> for ParameterIO {
    fn from(val: AS) -> Self {
        ParameterIOBuilder::new(val).build()
    }
}

impl Mergeable for AS {
    fn diff(&self, other: &Self) -> Self {
        if let Some(self_as) = self.0.as_ref() && let Some(other_as) = other.0.as_ref() {
            Self(Some(self_as.diff(other_as)))
        } else {
            Self(other.0.clone())
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        if let Some(self_as) = self.0.as_ref() && let Some(diff_as) = diff.0.as_ref() {
            Self(Some(self_as.merge(diff_as)))
        } else {
            Self(diff.0.clone().or_else(|| self.0.clone()))
        }
    }
}

impl ParameterResource for AS {
    fn path(name: &str) -> std::string::String {
        jstr!("Actor/AS/{name}.bas")
    }
}

impl Resource for AS {
    fn from_binary(data: impl AsRef<[u8]>) -> Result<Self> {
        (&ParameterIO::from_binary(data.as_ref())?).try_into()
    }

    fn into_binary(self, _endian: Endian) -> Vec<u8> {
        ParameterIO::from(self).to_binary()
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref().extension().and_then(|ext| ext.to_str()) == Some("bas")
    }
}

#[cfg(test)]
mod tests {
    use roead::aamp::*;

    use crate::prelude::*;

    #[test]
    fn serde() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = ParameterIO::from_binary(
            actor
                .get_data("Actor/AS/Guardian_MaterialTargetFound.bas")
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let as_data = super::AS::try_from(&pio).unwrap();
        let data = roead::aamp::ParameterIO::from(as_data.clone()).to_binary();
        let pio2 = ParameterIO::from_binary(data).unwrap();
        let as_data2 = super::AS::try_from(&pio2).unwrap();
        assert_eq!(as_data, as_data2);
    }

    #[test]
    fn diff() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = ParameterIO::from_binary(
            actor
                .get_data("Actor/AS/Guardian_MaterialTargetFound.bas")
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let as_data = super::AS::try_from(&pio).unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Guardian_A");
        let pio2 = ParameterIO::from_binary(
            actor2
                .get_data("Actor/AS/Guardian_MaterialTargetFound.bas")
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let as_data2 = super::AS::try_from(&pio2).unwrap();
        let _diff = as_data.diff(&as_data2);
        dbg!(_diff);
    }

    #[test]
    fn merge() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = ParameterIO::from_binary(
            actor
                .get_data("Actor/AS/Guardian_MaterialTargetFound.bas")
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Guardian_A");
        let as_data = super::AS::try_from(&pio).unwrap();
        let pio2 = ParameterIO::from_binary(
            actor2
                .get_data("Actor/AS/Guardian_MaterialTargetFound.bas")
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let as_data2 = super::AS::try_from(&pio2).unwrap();
        let diff = as_data.diff(&as_data2);
        let merged = as_data.merge(&diff);
        assert_eq!(as_data2, merged);
    }

    #[test]
    fn identify() {
        let path = std::path::Path::new(
            "content/Actor/Pack/Enemy_Guardian_A.sbactorpack//Actor/AS/Guardian_MaterialDefault.\
             bas",
        );
        assert!(super::AS::path_matches(path));
    }
}
