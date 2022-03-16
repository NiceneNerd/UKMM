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
    behaviors: Option<IndexMap<u32, ParameterList>>,
}

impl AIEntry {
    fn full_name(&self) -> String {
        self.def
            .0
            .values()
            .filter_map(|p| p.as_string().ok())
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ActionEntry {
    def: ParameterObject,
    params: Option<ParameterObject>,
    behaviors: Option<IndexMap<u32, ParameterList>>,
}

impl ActionEntry {
    fn full_name(&self) -> String {
        self.def
            .0
            .values()
            .filter_map(|p| p.as_string().ok())
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum ChildEntry {
    AI(AIEntry),
    Action(ActionEntry),
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct AIProgram {
    pub demos: IndexMap<u32, ActionEntry>,
    pub tree: IndexMap<String, AIEntry>,
    pub queries: IndexMap<String, ParameterList>,
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

mod parse {
    use super::*;

    fn plist_to_ai(
        plist: &ParameterList,
        pio: &ParameterIO,
        action_offset: usize,
    ) -> Result<AIEntry> {
        Ok(AIEntry {
            def: plist
                .object("Def")
                .ok_or_else(|| UKError::MissingAampKey("AI entry missing Def object".to_owned()))?
                .clone(),
            params: plist.object("SInst").cloned(),
            children: plist
                .object("ChildIdx")
                .ok_or_else(|| {
                    UKError::MissingAampKey("AI entry missing ChildIdx object".to_owned())
                })?
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
                                        UKError::MissingAampKey(format!(
                                            "AI program missing entry at {}",
                                            idx
                                        ))
                                    })?,
                                pio,
                                action_offset,
                            )?)
                        } else {
                            ChildEntry::Action(plist_to_action(
                                pio.list("Action")
                                    .unwrap()
                                    .lists
                                    .0
                                    .values()
                                    .nth(idx - action_offset)
                                    .ok_or_else(|| {
                                        UKError::MissingAampKey(format!(
                                            "AI program missing entry at {}",
                                            idx
                                        ))
                                    })?,
                                pio,
                            )?)
                        },
                    ))
                })
                .collect::<Result<IndexMap<_, _>>>()?,
            behaviors: plist
                .object("BehaviorIdx")
                .map(|pobj| -> Result<IndexMap<u32, ParameterList>> {
                    pobj.params()
                        .iter()
                        .map(|(k, v)| -> Result<(u32, ParameterList)> {
                            Ok((
                                *k,
                                pio.list("Behavior")
                                    .unwrap()
                                    .lists
                                    .0
                                    .values()
                                    .nth(v.as_int()? as usize)
                                    .ok_or_else(|| {
                                        UKError::MissingAampKey(format!(
                                            "AI program missing behavior at {:?}",
                                            v
                                        ))
                                    })?
                                    .clone(),
                            ))
                        })
                        .collect()
                })
                .transpose()?,
        })
    }

    fn plist_to_action(plist: &ParameterList, pio: &ParameterIO) -> Result<ActionEntry> {
        Ok(ActionEntry {
            def: plist
                .object("Def")
                .ok_or_else(|| {
                    UKError::MissingAampKey("Action entry missing Def object".to_owned())
                })?
                .clone(),
            params: plist.object("SInst").cloned(),
            behaviors: plist
                .object("BehaviorIdx")
                .map(|pobj| -> Result<IndexMap<u32, ParameterList>> {
                    pobj.params()
                        .iter()
                        .map(|(k, v)| -> Result<(u32, ParameterList)> {
                            Ok((
                                *k,
                                pio.list("Behavior")
                                    .unwrap()
                                    .lists
                                    .0
                                    .values()
                                    .nth(v.as_int()? as usize)
                                    .ok_or_else(|| {
                                        UKError::MissingAampKey(format!(
                                            "AI program missing behavior at {:?}",
                                            v
                                        ))
                                    })?
                                    .clone(),
                            ))
                        })
                        .collect()
                })
                .transpose()?,
        })
    }

    impl TryFrom<&ParameterIO> for AIProgram {
        type Error = UKError;

        fn try_from(pio: &ParameterIO) -> Result<Self> {
            let action_offset;
            Ok(Self {
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
                        action_offset = ai_list.lists.len();
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
                demos: pio
                    .object("DemoAIActionIdx")
                    .ok_or_else(|| {
                        UKError::MissingAampKey("AI program missing Demo action indexes".to_owned())
                    })?
                    .0
                    .iter()
                    .map(|(k, v)| -> Result<(u32, ActionEntry)> {
                        let idx = v.as_int()? as usize - action_offset;
                        Ok((
                            *k,
                            plist_to_action(
                                pio.list("Action")
                                    .unwrap()
                                    .lists
                                    .0
                                    .values()
                                    .nth(idx)
                                    .ok_or_else(|| {
                                        UKError::MissingAampKey(format!(
                                            "AI program missing entry at {}",
                                            idx
                                        ))
                                    })?,
                                pio,
                            )?,
                        ))
                    })
                    .collect::<Result<IndexMap<u32, ActionEntry>>>()?,
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
}

mod write {
    use std::collections::HashMap;

    use super::*;

    fn count_ais(ai: &AIEntry) -> usize {
        1 + ai
            .children
            .values()
            .filter_map(|c| match c {
                ChildEntry::AI(ai) => Some(count_ais(ai)),
                ChildEntry::Action(_) => None,
            })
            .sum::<usize>()
    }

    #[derive(Debug)]
    struct ParameterIOBuilder {
        aiprog: AIProgram,
        ais: Vec<ParameterList>,
        done_ais: HashMap<String, usize>,
        actions: Vec<ParameterList>,
        done_actions: HashMap<String, usize>,
        action_offset: usize,
        behaviors: Vec<ParameterList>,
    }

    impl ParameterIOBuilder {
        fn new(aiprog: AIProgram) -> Self {
            let action_offset = aiprog.tree.values().map(count_ais).sum();
            Self {
                action_offset,
                aiprog,
                ais: Vec::with_capacity(action_offset),
                done_ais: HashMap::with_capacity(action_offset),
                actions: vec![],
                done_actions: HashMap::new(),
                behaviors: vec![],
            }
        }

        fn ai_to_plist(&mut self, ai: AIEntry) -> usize {
            let name = ai.full_name();
            if let Some(idx) = self.done_ais.get(&name) {
                return *idx;
            }
            let mut plist = ParameterList::new();
            let idx = self.ais.len();
            plist.set_object("Def", ai.def);
            if let Some(params) = ai.params {
                plist.set_object("SInst", params);
            };
            if !ai.children.is_empty() {
                let mut children = ParameterObject::new();
                for (key, action) in ai.children {
                    let idx = match action {
                        ChildEntry::AI(child_ai) => self.ai_to_plist(child_ai),
                        ChildEntry::Action(child_action) => {
                            self.action_to_plist(child_action) + self.action_offset
                        }
                    };
                    children.0.insert(key, Parameter::Int(idx as i32));
                }
                plist.set_object("ChildIdx", children);
            }
            if let Some(behaviors) = ai.behaviors {
                let mut behavior_indexes = ParameterObject::new();
                for (key, behavior) in behaviors {
                    behavior_indexes.0.insert(
                        key,
                        Parameter::Int(if let Some(pos) =
                            self.behaviors.iter().position(|p| p == &behavior)
                        {
                            pos
                        } else {
                            let idx = self.behaviors.len();
                            self.behaviors.push(behavior.clone());
                            idx
                        } as i32),
                    );
                }
                plist.set_object("BehaviorIdx", behavior_indexes);
            };
            self.done_ais.insert(name, idx);
            self.ais.insert(idx, plist);
            idx
        }

        fn action_to_plist(&mut self, action: ActionEntry) -> usize {
            let name = action.full_name();
            if let Some(idx) = self.done_actions.get(&name) {
                return *idx;
            }
            let mut plist = ParameterList::new();
            plist.set_object("Def", action.def);
            if let Some(params) = action.params {
                plist.set_object("SInst", params);
            }
            if let Some(behaviors) = action.behaviors {
                let mut behavior_indexes = ParameterObject::new();
                for (key, behavior) in behaviors {
                    behavior_indexes.0.insert(
                        key,
                        Parameter::Int(if let Some(pos) =
                            self.behaviors.iter().position(|p| p == &behavior)
                        {
                            pos
                        } else {
                            let idx = self.behaviors.len();
                            self.behaviors.push(behavior.clone());
                            idx
                        } as i32),
                    );
                }
                plist.set_object("BehaviorIdx", behavior_indexes);
            };
            let idx = self.actions.len();
            self.done_actions.insert(name, idx);
            self.actions.push(plist);
            idx
        }

        fn build(mut self) -> ParameterIO {
            let mut pio = ParameterIO::new();
            pio.set_object("DemoAIActionIdx", ParameterObject::new());
            let mut tree: IndexMap<String, AIEntry> = IndexMap::new();
            std::mem::swap(&mut tree, &mut self.aiprog.tree);
            let roots: Vec<AIEntry> = tree.into_iter().map(|(_, root)| root).collect();
            for root in roots {
                self.ai_to_plist(root);
            }
            let mut demos: IndexMap<u32, ActionEntry> = IndexMap::new();
            std::mem::swap(&mut self.aiprog.demos, &mut demos);
            pio.object_mut("DemoAIActionIdx")
                .unwrap()
                .0
                .extend(demos.into_iter().map(|(k, action)| {
                    (k, {
                        Parameter::Int((self.action_to_plist(action) + self.action_offset) as i32)
                    })
                }));
            pio.set_list(
                "AI",
                ParameterList {
                    lists: ParameterListMap(
                        self.ais
                            .iter()
                            .enumerate()
                            .map(|(i, p)| (roead::aamp::hash_name(&format!("AI_{}", i)), p.clone()))
                            .collect(),
                    ),
                    objects: ParameterObjectMap::default(),
                },
            );
            pio.set_list(
                "Action",
                ParameterList {
                    lists: ParameterListMap(
                        self.actions
                            .iter()
                            .enumerate()
                            .map(|(i, p)| {
                                (roead::aamp::hash_name(&format!("Action_{}", i)), p.clone())
                            })
                            .collect(),
                    ),
                    objects: ParameterObjectMap::default(),
                },
            );
            pio.set_list(
                "Behavior",
                ParameterList {
                    lists: ParameterListMap(
                        self.behaviors
                            .iter()
                            .enumerate()
                            .map(|(i, p)| {
                                (
                                    roead::aamp::hash_name(&format!("Behavior_{}", i)),
                                    p.clone(),
                                )
                            })
                            .collect(),
                    ),
                    objects: ParameterObjectMap::default(),
                },
            );
            pio.set_list(
                "Query",
                ParameterList {
                    lists: ParameterListMap(
                        self.aiprog
                            .queries
                            .values()
                            .enumerate()
                            .map(|(i, p)| {
                                (roead::aamp::hash_name(&format!("Query_{}", i)), p.clone())
                            })
                            .collect(),
                    ),
                    objects: ParameterObjectMap::default(),
                },
            );
            pio
        }
    }

    impl AIProgram {
        pub fn into_pio(self) -> ParameterIO {
            ParameterIOBuilder::new(self).build()
        }
    }
}
