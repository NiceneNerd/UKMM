use crate::{Result, UKError};
use indexmap::IndexMap;
use roead::aamp::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct AIEntry {
    def: ParameterObject,
    params: Option<ParameterObject>,
    children: IndexMap<u32, ChildEntry>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum ChildEntry {
    AI(AIEntry),
    Action(ParameterList),
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct AIProgram {
    pub demos: IndexMap<u32, u32>,
    pub tree: IndexMap<String, AIEntry>,
    pub queries: IndexMap<String, ParameterList>,
}

fn plist_to_ai(plist: &ParameterList, pio: &ParameterIO, action_offset: usize) -> Result<AIEntry> {
    Ok(AIEntry {
        def: plist
            .object("Def")
            .ok_or_else(|| UKError::MissingAampKey("AI entry missing Def object".to_owned()))?
            .clone(),
        params: plist.object("SInst").cloned(),
        children: plist
            .object("ChildIdx")
            .ok_or_else(|| UKError::MissingAampKey("AI entry missing ChildIdx object".to_owned()))?
            .0
            .iter()
            .map(|(k, v)| -> Result<(u32, ChildEntry)> {
                let idx = v.as_int().unwrap() as usize;
                Ok((
                    *k,
                    if idx < action_offset {
                        ChildEntry::AI(plist_to_ai(
                            pio.list("AI")
                                .unwrap()
                                .lists
                                .0
                                .values()
                                .nth(idx)
                                .ok_or_else(|| {
                                    UKError::MissingAampKey(format!("AI missing entry at {}", idx))
                                })?,
                            pio,
                            action_offset,
                        )?)
                    } else {
                        ChildEntry::Action(
                            pio.list("Action")
                                .unwrap()
                                .lists
                                .0
                                .values()
                                .nth(idx - action_offset)
                                .ok_or_else(|| {
                                    UKError::MissingAampKey(format!("AI missing entry at {}", idx))
                                })?
                                .clone(),
                        )
                    },
                ))
            })
            .collect::<Result<IndexMap<_, _>>>()?,
    })
}

impl TryFrom<&ParameterIO> for AIProgram {
    type Error = UKError;

    fn try_from(pio: &ParameterIO) -> Result<Self> {
        Ok(Self {
            demos: pio
                .object("DemoAIActionIdx")
                .ok_or_else(|| {
                    UKError::MissingAampKey("AI program missing Demo action indexes".to_owned())
                })?
                .0
                .iter()
                .map(|(k, v)| v.as_int().map(|v| (*k, v as u32)))
                .collect::<std::result::Result<IndexMap<u32, u32>, roead::aamp::AampError>>()?,
            tree: {
                if let Some(ai_list) = pio.list("AI") {
                    let child_indexes: HashSet<usize> = ai_list
                        .lists
                        .0
                        .values()
                        .filter_map(|ai| {
                            ai.object("ChildIdx").map(|ci| {
                                ci.params()
                                    .values()
                                    .flat_map(|i| i.as_int().map(|i| i as usize).ok())
                            })
                        })
                        .flatten()
                        .collect();
                    let roots: Vec<ParameterList> = ai_list
                        .lists
                        .0
                        .values()
                        .enumerate()
                        .filter(|(i, _)| !child_indexes.contains(i))
                        .map(|(_, ai)| ai)
                        .cloned()
                        .collect();
                    let action_offset = ai_list.lists.len();
                    roots
                        .iter()
                        .map(|root| -> Result<(String, AIEntry)> {
                            Ok((
                                root.object("Def")
                                    .ok_or_else(|| {
                                        UKError::MissingAampKey(
                                            "AI entry missing Def object".to_owned(),
                                        )
                                    })?
                                    .param("ClassName")
                                    .ok_or_else(|| {
                                        UKError::MissingAampKey(
                                            "AI def missing ClassName".to_owned(),
                                        )
                                    })?
                                    .as_string()?
                                    .to_owned(),
                                plist_to_ai(root, pio, action_offset)?,
                            ))
                        })
                        .collect::<Result<IndexMap<_, _>>>()?
                } else {
                    return Err(UKError::MissingAampKey(
                        "AI program missing AI list".to_owned(),
                    ));
                }
            },
            queries: pio
                .list("Query")
                .ok_or_else(|| {
                    UKError::MissingAampKey("AI program missing Queries list".to_owned())
                })?
                .lists
                .0
                .values()
                .cloned()
                .map(|query| -> Result<(String, ParameterList)> {
                    Ok((
                        query
                            .object("Def")
                            .ok_or_else(|| {
                                UKError::MissingAampKey("Query missing Def object".to_owned())
                            })?
                            .param("ClassName")
                            .ok_or_else(|| {
                                UKError::MissingAampKey("AI def missing ClassName".to_owned())
                            })?
                            .as_string()?
                            .to_owned(),
                        query,
                    ))
                })
                .collect::<Result<IndexMap<_, _>>>()?,
        })
    }
}

// impl AIProgram {
//     pub fn diff(&self, other: &AIProgram) -> Self {
//         Self {
//             demos: other
//                 .demos
//                 .iter()
//                 .filter(|(k, v)| !self.demos.contains_key(k) || self.demos.get(k).unwrap() != *v)
//                 .map(|(k, v)| (*k, *v))
//                 .collect(),
//             ais: {
//                 let mut new_ais: Vev<ParameterList> = vec![];
//                 for index in other.ais.keys() {

//                 }
//             },
//         }
//     }
// }
