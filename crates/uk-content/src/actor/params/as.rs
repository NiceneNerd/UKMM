use std::collections::BTreeMap;

use join_str::jstr;
use roead::aamp::*;
use serde::{Deserialize, Serialize};
use uk_content_derive::ParamData;
#[cfg(feature = "ui")]
use uk_ui_derive::Editable;
use uk_util::OptionResultExt;

use crate::{
    actor::ParameterResource,
    prelude::*,
    util::{self, IteratorExt},
    Result, UKError,
};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ParamData)]
#[cfg_attr(feature = "ui", derive(Editable))]
pub struct ElementParams {
    #[name = "TypeIndex"]
    pub type_index: i32,
    #[name = "NoSync"]
    pub no_sync: Option<bool>,
    #[name = "JudgeOnce"]
    pub judge_once: Option<bool>,
    #[name = "InputLimit"]
    pub input_limit: Option<f32>,
    #[name = "FileName"]
    pub file_name: Option<String64>,
    #[name = "Morph"]
    pub morph: Option<f32>,
    #[name = "ResetMorph"]
    pub reset_morph: Option<f32>,
    #[name = "SequenceLoop"]
    pub sequence_loop: Option<bool>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ui", derive(Editable))]
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
                        .enumerate()
                        .map(|(pos, idx)| -> Result<(usize, Element)> {
                            let idx = idx.as_int()?;
                            Ok((
                                pos,
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

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ui", derive(Editable))]
pub struct AS {
    pub root: Option<Element>,
    pub common_params: Option<ParameterObject>,
}

impl TryFrom<&ParameterIO> for AS {
    type Error = UKError;

    fn try_from(pio: &ParameterIO) -> Result<Self> {
        Ok(Self {
            root: pio
                .list("Elements")
                .ok_or(UKError::MissingAampKey("AS missing elements list", None))?
                .lists
                .0
                .values()
                .next()
                .map(|list| Element::try_from_plist(list, pio))
                .transpose()?,
            common_params: pio.object("CommonParams").cloned(),
        })
    }
}

impl From<AS> for ParameterIO {
    fn from(val: AS) -> Self {
        fn count_elements(element: &Element) -> usize {
            1 + element
                .children
                .as_ref()
                .map(|children| children.values().map(count_elements).sum())
                .unwrap_or(0)
        }

        fn add_element(element: Element, done: &mut Vec<(Element, ParameterList)>) {
            let index = done.len();
            done.push((element.clone(), Default::default()));
            let Element {
                params,
                children,
                extend,
            } = element;
            let should_hack = params.type_index == 94 && index == 0;
            let mut list = ParameterList::new();
            list.set_object("Parameters", params.into());
            if children.is_some() {
                list.set_object("Children", Default::default());
            }
            if let Some(extend) = extend {
                list.set_list("Extend", extend);
            }
            if let Some(children) = children {
                #[allow(clippy::unwrap_used)]
                if should_hack {
                    list.object_mut("Children").unwrap().extend(
                        children
                            .values()
                            .named_enumerate("Child")
                            .map(|(name, child)| {
                                let index = done.len();
                                add_element(child.clone(), done);
                                (name.into(), Parameter::I32(index as i32))
                            }),
                    );
                } else {
                    let first = children.values().next();
                    let all_same =
                        children.len() > 1 && children.values().all(|v| Some(v) == first);
                    let last_idx = children.len() - 1;
                    for child in children.values() {
                        if !done.iter().any(|(el, _)| child == el) {
                            add_element(child.clone(), done);
                        }
                    }
                    list.object_mut("Children").unwrap().extend(
                        children.values().named_enumerate("Child").enumerate().map(
                            |(i, (name, child))| {
                                let index = done.iter().position(|(el, _)| el == child).unwrap();
                                (
                                    name.into(),
                                    if all_same && i == last_idx {
                                        let clone_index = done.len();
                                        done.push(done[index].clone());
                                        Parameter::I32(clone_index as i32)
                                    } else {
                                        Parameter::I32(index as i32)
                                    },
                                )
                            },
                        ),
                    );
                }
            }
            done[index].1 = list;
        }

        let mut elements = Vec::with_capacity(val.root.as_ref().map(count_elements).unwrap_or(0));
        if let Some(root) = val.root {
            add_element(root, &mut elements);
        }
        ParameterIO::new()
            .with_objects(val.common_params.into_iter().map(|p| ("CommonParams", p)))
            .with_list(
                "Elements",
                ParameterList::new().with_lists(
                    elements
                        .into_iter()
                        .enumerate()
                        .map(|(i, (_, list))| (jstr!("Element{&lexical::to_string(i)}"), list)),
                ),
            )
    }
}

impl Mergeable for AS {
    fn diff(&self, other: &Self) -> Self {
        Self {
            root: other.root.as_ref().map(|other_root| {
                self.root
                    .as_ref()
                    .map(|self_root| self_root.diff(other_root))
                    .unwrap_or_else(|| other_root.clone())
            }),
            common_params: other.common_params.as_ref().map(|other_params| {
                self.common_params
                    .as_ref()
                    .map(|self_params| util::diff_pobj(self_params, other_params))
                    .unwrap_or_else(|| other_params.clone())
            }),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            root: diff
                .root
                .as_ref()
                .map(|diff_root| {
                    self.root
                        .as_ref()
                        .map(|self_root| self_root.merge(diff_root))
                        .unwrap_or_else(|| diff_root.clone())
                })
                .or_else(|| self.root.clone()),
            common_params: diff
                .common_params
                .as_ref()
                .map(|diff_params| {
                    self.common_params
                        .as_ref()
                        .map(|self_params| util::merge_pobj(self_params, diff_params))
                        .unwrap_or_else(|| diff_params.clone())
                })
                .or_else(|| self.common_params.clone()),
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
        path.as_ref()
            .extension()
            .and_then(|ext| ext.to_str())
            .contains(&"bas")
    }
}

#[allow(clippy::unwrap_used)]
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
                .unwrap(),
        )
        .unwrap();
        let as_data = super::AS::try_from(&pio).unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Guardian_A");
        let pio2 = ParameterIO::from_binary(
            actor2
                .get_data("Actor/AS/Guardian_MaterialTargetFound.bas")
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
                .unwrap(),
        )
        .unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Guardian_A");
        let as_data = super::AS::try_from(&pio).unwrap();
        let pio2 = ParameterIO::from_binary(
            actor2
                .get_data("Actor/AS/Guardian_MaterialTargetFound.bas")
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

    #[test]
    fn node_order() {
        for file in ["Player_Warp", "Player_Wait", "Player_WeaponEquipOff"] {
            let pio = ParameterIO::from_binary(
                std::fs::read(
                    std::path::Path::new("test/Actor/AS")
                        .join(file)
                        .with_extension("bas"),
                )
                .unwrap(),
            )
            .unwrap();
            let as_data = super::AS::try_from(&pio).unwrap();
            let pio2 = ParameterIO::from(as_data.clone());
            let as_data2 = super::AS::try_from(&pio2).unwrap();
            assert_eq!(as_data, as_data2);
            // if pio != pio2 {
            //     println!("{}", pio.to_text());
            //     println!("{}", pio2.to_text());
            //     panic!("Node data changed in {file}");
            // }
        }
    }
}
