use crate::{prelude::*, util, Result, UKError};
use roead::aamp::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Element {
    pub params: ParameterObject,
    pub children: Option<BTreeMap<usize, Element>>,
    pub extend: Option<ParameterList>,
}

impl Element {
    fn try_from_plist(list: &ParameterList, pio: &ParameterIO) -> Result<Self> {
        Ok(Self {
            params: list
                .object("Parameters")
                .ok_or(UKError::MissingAampKey("AS node missing parameters"))?
                .clone(),
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
                                    pio.list("Elements")
                                        .unwrap()
                                        .lists
                                        .0
                                        .values()
                                        .nth(idx)
                                        .ok_or_else(|| {
                                            UKError::MissingAampKeyD(format!(
                                                "AS control node missing child at index {}",
                                                idx,
                                            ))
                                        })?,
                                    pio,
                                )?,
                            ))
                        })
                        .collect::<Result<_>>()
                })
                .transpose()?,
            extend: list.list("Extend").cloned(),
        })
    }
}

impl Mergeable<()> for Element {
    fn diff(&self, other: &Self) -> Self {
        Self {
            params: util::diff_pobj(&self.params, &other.params),
            children: other.children.as_ref().map(|other_children| {
                self.children
                    .as_ref()
                    .map(|self_children| util::simple_index_diff(self_children, other_children))
                    .unwrap_or_else(|| other_children.clone())
            }),
            extend: other.extend.as_ref().map(|other_extend| {
                self.extend
                    .as_ref()
                    .map(|self_extend| util::diff_plist(self_extend, other_extend))
                    .unwrap_or_else(|| other_extend.clone())
            }),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        todo!()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AS(pub Option<Element>);

impl TryFrom<&ParameterIO> for AS {
    type Error = UKError;

    fn try_from(pio: &ParameterIO) -> Result<Self> {
        Ok(Self(
            pio.list("Elements")
                .ok_or(UKError::MissingAampKey("AS missing elements list"))?
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
    done: Vec<Element>,
}

impl ParameterIOBuilder {
    fn new(val: AS) -> Self {
        Self {
            as_val: val,
            done: Vec::new(),
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
                objects: [("Parameters", element.params.clone())]
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
                                    (format!("Child{}", i), Parameter::Int(index as i32))
                                })
                                .collect(),
                        )
                    }))
                    .collect(),
                lists: element
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
                        .map(|(i, list)| (format!("Element{}", i), list))
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

impl Mergeable<ParameterIO> for AS {
    fn diff(&self, other: &Self) -> Self {
        todo!()
    }

    fn merge(&self, diff: &Self) -> Self {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use pretty_assertions::assert_eq;
    use roead::aamp::*;

    #[test]
    fn serde() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = ParameterIO::from_binary(
            actor
                .get_file_data("Actor/AS/Guardian_MaterialTargetFound.bas")
                .unwrap(),
        )
        .unwrap();
        let as_data = super::AS::try_from(&pio).unwrap();
        let data = as_data.clone().into_pio().to_binary();
        let pio2 = ParameterIO::from_binary(&data).unwrap();
        let as_data2 = super::AS::try_from(&pio2).unwrap();
        assert_eq!(as_data, as_data2);
    }
}
