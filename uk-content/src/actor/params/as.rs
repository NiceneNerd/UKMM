use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
};

use itertools::Itertools;
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
    #[name = "ResetMorph"]
    reset_morph: Option<f32>,
    #[name = "SequenceLoop"]
    sequence_loop: Option<bool>,
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
                        .enumerate()
                        .map(|(pos, idx)| -> Result<(usize, Element)> {
                            let idx = idx.as_int()? as usize;
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

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Editable)]
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

// #[derive(Debug)]
// struct ParameterIOBuilder {
//     as_val: AS,
//     done:   Vec<Element>,
// }

// impl ParameterIOBuilder {
//     fn new(val: AS) -> Self {
//         Self {
//             as_val: val,
//             done:   Vec::new(),
//         }
//     }

//     fn add_element(&mut self, element: &Element, next: usize) -> (usize, Vec<ParameterList>) {
//         if let Some(idx) = self.done.iter().position(|e| e == element) {
//             (idx, vec![])
//         } else {
//             let idx = next;
//             let mut child_lists: Vec<ParameterList> =
//                 Vec::with_capacity(element.children.as_ref().map(|cl| cl.len()).unwrap_or(1));
//             self.done.push(element.clone());
//             let list = ParameterList {
//                 objects: [("Parameters", element.params.clone().into())]
//                     .into_iter()
//                     .chain(element.children.iter().map(|children| {
//                         (
//                             "Children",
//                             children
//                                 .iter()
//                                 .map(|(i, child)| {
//                                     let (index, child_list) = self.add_element(child, idx + i +
// 1);                                     child_lists.extend(child_list);
//                                     (
//                                         jstr!("Child{&lexical::to_string(*i)}"),
//                                         Parameter::Int(index as i32),
//                                     )
//                                 })
//                                 .collect(),
//                         )
//                     }))
//                     .collect(),
//                 lists:   element
//                     .extend
//                     .iter()
//                     .map(|extend| ("Extend", extend.clone()))
//                     .collect(),
//             };
//             child_lists.insert(0, list);
//             (idx, child_lists)
//         }
//     }

//     fn build(mut self) -> ParameterIO {
//         let as_val = std::mem::take(&mut self.as_val);
//         ParameterIO::new()
//             .with_list(
//                 "Elements",
//                 as_val
//                     .root
//                     .map(|element| {
//                         let (_, elements) = self.add_element(&element, 0);
//                         ParameterList {
//                             lists: elements
//                                 .into_iter()
//                                 .enumerate()
//                                 .map(|(i, list)| (jstr!("Element{&lexical::to_string(i)}"),
// list))                                 .collect(),
//                             ..Default::default()
//                         }
//                     })
//                     .unwrap_or_default(),
//             )
//             .with_objects(
//                 as_val
//                     .common_params
//                     .into_iter()
//                     .map(|p| ("CommonParams", p)),
//             )
//     }
// }

impl From<AS> for ParameterIO {
    fn from(val: AS) -> Self {
        #[derive(Debug)]
        struct ElementData<'el> {
            element:  &'el Element,
            refs:     Cell<usize>,
            tree_len: usize,
            index:    Cell<Option<usize>>,
        }
        impl PartialEq for ElementData<'_> {
            fn eq(&self, other: &Self) -> bool {
                self.element.eq(other.element)
            }
        }
        impl Eq for ElementData<'_> {}
        impl PartialOrd for ElementData<'_> {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.refs.cmp(&other.refs).then_with(|| {
                    self.element
                        .children
                        .as_ref()
                        .map(|c| c.len())
                        .unwrap_or(0)
                        .cmp(
                            &other
                                .element
                                .children
                                .as_ref()
                                .map(|c| c.len())
                                .unwrap_or(0),
                        )
                }))
            }
        }
        impl Ord for ElementData<'_> {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                self.partial_cmp(&other).unwrap()
            }
        }

        #[derive(Debug)]
        struct Builder<'a> {
            as_val: &'a AS,
            elements: Vec<ElementData<'a>>,
            map: RefCell<BTreeMap<usize, ParameterList>>,
        }

        impl<'a> Builder<'a> {
            fn new(as_val: &'a AS) -> Self {
                Self {
                    as_val,
                    elements: vec![],
                    map: Default::default(),
                }
            }

            fn collect_elements(&mut self, element: &'a Element) -> usize {
                if let Some(data) = self.elements.iter().find(|d| d.element == element) {
                    data.refs.set(data.refs.get() + 1);
                    0
                } else {
                    let mut data = ElementData {
                        element,
                        refs: Cell::new(0),
                        tree_len: 1,
                        index: Default::default(),
                    };
                    let mut len = 1;
                    if let Some(children) = element.children.as_ref() {
                        for child in children.values() {
                            len += self.collect_elements(child);
                        }
                    }
                    data.tree_len = len;
                    self.elements.push(data);
                    len
                }
            }

            fn write_element(&self, element: &'a Element, index: usize) {
                let mut list = ParameterList::new()
                    .with_object("Parameters", element.params.clone().into())
                    .with_lists(element.extend.as_ref().map(|ex| ("Extend", ex.clone())));
                if let Some(children) = element.children.as_ref() {
                    let mut current = index + 1;
                    let mut idx_map = BTreeMap::new();
                    for (i, child, data) in children
                        .iter()
                        .map(|(i, child)| {
                            (
                                i,
                                child,
                                self.elements.iter().find(|el| el.element == child).unwrap(),
                            )
                        })
                        .sorted_unstable_by(|(_, _, d1), (_, _, d2)| d1.cmp(d2))
                    {
                        if let Some(done_index) = data.index.get() && done_index > index {
                            idx_map.insert(i, done_index);
                        } else {
                            self.write_element(child, current);
                            data.index.set(Some(current));
                            idx_map.insert(i, current);
                            current += data.tree_len;
                        }
                    }
                    list.set_object(
                        "Children",
                        idx_map
                            .into_iter()
                            .map(|(i, idx)| (format!("Child{i}"), Parameter::Int(idx as i32)))
                            .collect(),
                    );
                }
                assert!(!self.map.borrow().contains_key(&index));
                self.map.borrow_mut().insert(index, list);
            }

            fn build(mut self) -> ParameterIO {
                if let Some(root) = self.as_val.root.as_ref() {
                    self.collect_elements(root);
                    self.elements.sort();
                    self.write_element(root, 0);
                }

                ParameterIO::new()
                    .with_objects(
                        self.as_val
                            .common_params
                            .iter()
                            .map(|p| ("CommonParams", p.clone())),
                    )
                    .with_list(
                        "Elements",
                        ParameterList::new().with_lists(
                            self.map
                                .into_inner()
                                .into_iter()
                                .map(|(i, list)| (format!("Element{i}"), list)),
                        ),
                    )
            }
        }

        Builder::new(&val).build()
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
