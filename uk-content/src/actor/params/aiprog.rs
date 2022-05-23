use crate::{prelude::*, util, Result, UKError};
use indexmap::IndexMap;
use join_str::jstr;
use roead::aamp::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub struct AIEntry {
    pub def: ParameterObject,
    pub params: Option<ParameterObject>,
    pub children: IndexMap<u32, ChildEntry>,
    pub behaviors: Option<IndexMap<u32, ParameterList>>,
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

impl Mergeable<()> for AIEntry {
    fn diff(&self, other: &Self) -> Self {
        let mut diff = AIEntry::default();
        if self.def != other.def {
            diff.def = self.def.clone();
            diff.def.0.extend(other.def.0.iter().filter_map(|(k, v)| {
                if !self.def.0.contains_key(k) || self.def.0[k] != *v {
                    Some((*k, v.clone()))
                } else {
                    None
                }
            }));
        } else {
            diff.def = self.def.clone();
        }
        if self.params != other.params {
            if let Some(self_params) = &self.params {
                diff.params = other
                    .params
                    .as_ref()
                    .map(|params| util::diff_pobj(self_params, params));
            } else {
                diff.params = other.params.clone();
            }
        }
        if self.behaviors != other.behaviors {
            if let Some(self_behaviors) = &self.behaviors {
                diff.behaviors = other.behaviors.as_ref().map(|behaviors| {
                    behaviors
                        .iter()
                        .filter_map(|(k, v)| {
                            if !self_behaviors.contains_key(k) {
                                Some((*k, v.clone()))
                            } else if self_behaviors[k] != *v {
                                Some((*k, util::diff_plist(&self_behaviors[k], v)))
                            } else {
                                None
                            }
                        })
                        .collect()
                });
            } else {
                diff.behaviors = other.behaviors.clone();
            }
        }
        diff.children = other
            .children
            .iter()
            .filter_map(|(k, v)| {
                if !self.children.contains_key(k) {
                    Some((*k, v.clone()))
                } else if &self.children[k] != v {
                    let self_child = &self.children[k];
                    match (self_child, v) {
                        (ChildEntry::AI(_), ChildEntry::Action(_))
                        | (ChildEntry::Action(_), ChildEntry::AI(_)) => Some((*k, v.clone())),
                        (ChildEntry::AI(self_ai), ChildEntry::AI(other_ai)) => {
                            Some((*k, ChildEntry::AI(self_ai.diff(other_ai))))
                        }
                        (ChildEntry::Action(self_action), ChildEntry::Action(other_action)) => {
                            Some((*k, ChildEntry::Action(self_action.diff(other_action))))
                        }
                    }
                } else {
                    None
                }
            })
            .collect();
        diff
    }

    fn merge(&self, diff: &Self) -> Self {
        let mut new = self.clone();
        new.def = diff.def.clone();
        if let Some(diff_params) = &diff.params {
            if let Some(new_params) = &new.params {
                new.params = Some(util::merge_pobj(new_params, diff_params));
            } else {
                new.params = diff.params.clone();
            }
        }
        if let Some(diff_behaviors) = &diff.behaviors {
            if let Some(base_behaviors) = &self.behaviors {
                new.behaviors = Some(
                    base_behaviors
                        .iter()
                        .chain(diff_behaviors.iter())
                        .map(|(k, v)| (*k, v.clone()))
                        .collect(),
                );
            } else {
                new.behaviors = diff.behaviors.clone();
            }
        }
        for (k, v) in &diff.children {
            if let Some(base_child) = self.children.get(k) {
                match (base_child, v) {
                    (ChildEntry::AI(_), ChildEntry::Action(_))
                    | (ChildEntry::Action(_), ChildEntry::AI(_)) => {
                        new.children.insert(*k, v.clone());
                    }
                    (ChildEntry::AI(base_ai), ChildEntry::AI(diff_ai)) => {
                        new.children
                            .insert(*k, ChildEntry::AI(AIEntry::merge(base_ai, diff_ai)));
                    }
                    (ChildEntry::Action(base_action), ChildEntry::Action(diff_action)) => {
                        new.children.insert(
                            *k,
                            ChildEntry::Action(ActionEntry::merge(base_action, diff_action)),
                        );
                    }
                }
            } else {
                new.children.insert(*k, v.clone());
            }
        }
        new
    }
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub struct ActionEntry {
    pub def: ParameterObject,
    pub params: Option<ParameterObject>,
    pub behaviors: Option<IndexMap<u32, ParameterList>>,
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

impl Mergeable<()> for ActionEntry {
    fn diff(&self, other: &Self) -> Self {
        let mut diff = ActionEntry::default();
        if self.def != other.def {
            diff.def = self.def.clone();
            diff.def.0.extend(other.def.0.iter().filter_map(|(k, v)| {
                if !self.def.0.contains_key(k) || self.def.0[k] != *v {
                    Some((*k, v.clone()))
                } else {
                    None
                }
            }));
        } else {
            diff.def = self.def.clone();
        }
        if self.params != other.params {
            if let Some(self_params) = &self.params {
                diff.params = other
                    .params
                    .as_ref()
                    .map(|params| util::diff_pobj(self_params, params));
            } else {
                diff.params = other.params.clone();
            }
        }
        if self.behaviors != other.behaviors {
            if let Some(self_behaviors) = &self.behaviors {
                diff.behaviors = other.behaviors.as_ref().map(|behaviors| {
                    behaviors
                        .iter()
                        .filter_map(|(k, v)| {
                            if !self_behaviors.contains_key(k) {
                                Some((*k, v.clone()))
                            } else if self_behaviors[k] != *v {
                                Some((*k, util::diff_plist(&self_behaviors[k], v)))
                            } else {
                                None
                            }
                        })
                        .collect()
                });
            } else {
                diff.behaviors = other.behaviors.clone();
            }
        }
        diff
    }

    fn merge(&self, diff: &Self) -> Self {
        let mut new = self.clone();
        new.def = diff.def.clone();
        if let Some(diff_params) = &diff.params {
            if let Some(new_params) = &new.params {
                new.params = Some(util::merge_pobj(new_params, diff_params));
            } else {
                new.params = diff.params.clone();
            }
        }
        if let Some(diff_behaviors) = &diff.behaviors {
            if let Some(base_behaviors) = &self.behaviors {
                new.behaviors = Some(
                    base_behaviors
                        .iter()
                        .chain(diff_behaviors.iter())
                        .map(|(k, v)| (*k, v.clone()))
                        .collect(),
                );
            } else {
                new.behaviors = diff.behaviors.clone();
            }
        }
        new
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

impl Mergeable<ParameterIO> for AIProgram {
    fn diff(&self, other: &Self) -> Self {
        Self {
            demos: other
                .demos
                .iter()
                .filter_map(|(k, v)| {
                    if !self.demos.contains_key(k) {
                        Some((*k, v.clone()))
                    } else if &self.demos[k] != v {
                        Some((*k, self.demos[k].diff(v)))
                    } else {
                        None
                    }
                })
                .collect(),
            queries: other
                .queries
                .iter()
                .filter_map(|(k, v)| {
                    if !self.queries.contains_key(k) {
                        Some((k.clone(), v.clone()))
                    } else if &self.queries[k] != v {
                        Some((k.clone(), util::diff_plist(&self.queries[k], v)))
                    } else {
                        None
                    }
                })
                .collect(),
            tree: other
                .tree
                .iter()
                .filter_map(|(k, v)| {
                    if !self.tree.contains_key(k) {
                        Some((k.clone(), v.clone()))
                    } else if &self.tree[k] != v {
                        Some((k.clone(), self.tree[k].diff(v)))
                    } else {
                        None
                    }
                })
                .collect(),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            demos: {
                let mut new = self.demos.clone();
                for (k, v) in &diff.demos {
                    let merged = if let Some(entry) = new.get_mut(k) {
                        ActionEntry::merge(entry, v)
                    } else {
                        v.clone()
                    };
                    new.insert(*k, merged);
                }
                new
            },
            queries: {
                let mut new = self.queries.clone();
                for (k, v) in &diff.queries {
                    let merged = if let Some(entry) = new.get_mut(k) {
                        util::merge_plist(entry, v)
                    } else {
                        v.clone()
                    };
                    new.insert(k.clone(), merged);
                }
                new
            },
            tree: {
                let mut new = self.tree.clone();
                for (k, v) in &diff.tree {
                    let merged = if let Some(entry) = new.get_mut(k) {
                        AIEntry::merge(entry, v)
                    } else {
                        v.clone()
                    };
                    new.insert(k.clone(), merged);
                }
                new
            },
        }
    }
}

mod parse {
    use super::*;

    fn plist_to_ai(
        list: &ParameterList,
        pio: &ParameterIO,
        action_offset: usize,
    ) -> Result<AIEntry> {
        Ok(AIEntry {
            def: list
                .object("Def")
                .ok_or(UKError::MissingAampKey("AI entry missing Def object"))?
                .clone(),
            params: list.object("SInst").cloned(),
            children: list
                .object("ChildIdx")
                .ok_or(UKError::MissingAampKey("AI entry missing ChildIdx object"))?
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
                                        UKError::MissingAampKeyD(jstr!(
                                            "AI program missing entry at {&lexical::to_string(idx)}"
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
                                        UKError::MissingAampKeyD(jstr!(
                                            "AI program missing entry at {&lexical::to_string(idx)}"
                                        ))
                                    })?,
                                pio,
                            )?)
                        },
                    ))
                })
                .collect::<Result<IndexMap<_, _>>>()?,
            behaviors: list
                .object("BehaviorIdx")
                .map(|obj| -> Result<IndexMap<u32, ParameterList>> {
                    obj.params()
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
                                        UKError::MissingAampKeyD(format!(
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

    fn plist_to_action(list: &ParameterList, pio: &ParameterIO) -> Result<ActionEntry> {
        Ok(ActionEntry {
            def: list
                .object("Def")
                .ok_or(UKError::MissingAampKey("Action entry missing Def object"))?
                .clone(),
            params: list.object("SInst").cloned(),
            behaviors: list
                .object("BehaviorIdx")
                .map(|obj| -> Result<IndexMap<u32, ParameterList>> {
                    obj.params()
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
                                        UKError::MissingAampKeyD(format!(
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
                                        .ok_or(UKError::MissingAampKey(
                                            "AI entry missing Def object",
                                        ))?
                                        .param("ClassName")
                                        .ok_or(UKError::MissingAampKey("AI def missing ClassName"))?
                                        .as_string()?
                                        .to_owned(),
                                    plist_to_ai(root, pio, action_offset)?,
                                ))
                            })
                            .collect::<Result<IndexMap<_, _>>>()?
                    } else {
                        return Err(UKError::MissingAampKey("AI program missing AI list"));
                    }
                },
                demos: pio
                    .object("DemoAIActionIdx")
                    .ok_or(UKError::MissingAampKey(
                        "AI program missing Demo action indexes",
                    ))?
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
                                        UKError::MissingAampKeyD(jstr!(
                                            "AI program missing entry at {&lexical::to_string(idx)}"
                                        ))
                                    })?,
                                pio,
                            )?,
                        ))
                    })
                    .collect::<Result<IndexMap<u32, ActionEntry>>>()?,
                queries: pio
                    .list("Query")
                    .ok_or(UKError::MissingAampKey("AI program missing Queries list"))?
                    .lists
                    .0
                    .values()
                    .cloned()
                    .map(|query| -> Result<(String, ParameterList)> {
                        Ok((
                            query
                                .object("Def")
                                .ok_or(UKError::MissingAampKey("Query missing Def object"))?
                                .param("ClassName")
                                .ok_or(UKError::MissingAampKey("AI def missing ClassName"))?
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
    use super::*;
    use std::collections::HashMap;

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
            let mut list = ParameterList::new();
            let idx = self.ais.len();
            self.ais.insert(idx, ParameterList::new());
            list.set_object("Def", ai.def);
            if let Some(params) = ai.params {
                list.set_object("SInst", params);
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
                list.set_object("ChildIdx", children);
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
                list.set_object("BehaviorIdx", behavior_indexes);
            };
            self.done_ais.insert(name, idx);
            std::mem::swap(&mut list, self.ais.get_mut(idx).unwrap());
            idx
        }

        fn action_to_plist(&mut self, action: ActionEntry) -> usize {
            let name = action.full_name();
            if let Some(idx) = self.done_actions.get(&name) {
                return *idx;
            }
            let mut list = ParameterList::new();
            list.set_object("Def", action.def);
            if let Some(params) = action.params {
                list.set_object("SInst", params);
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
                list.set_object("BehaviorIdx", behavior_indexes);
            };
            let idx = self.actions.len();
            self.done_actions.insert(name, idx);
            self.actions.push(list);
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
                    lists: self
                        .ais
                        .iter()
                        .enumerate()
                        .map(|(i, p)| (jstr!("AI_{&lexical::to_string(i)}"), p.clone()))
                        .collect(),
                    objects: ParameterObjectMap::default(),
                },
            );
            pio.set_list(
                "Action",
                ParameterList {
                    lists: self
                        .actions
                        .iter()
                        .enumerate()
                        .map(|(i, p)| (jstr!("Action_{&lexical::to_string(i)}"), p.clone()))
                        .collect(),
                    objects: ParameterObjectMap::default(),
                },
            );
            pio.set_list(
                "Behavior",
                ParameterList {
                    lists: self
                        .behaviors
                        .iter()
                        .enumerate()
                        .map(|(i, p)| (jstr!("Behavior_{&lexical::to_string(i)}"), p.clone()))
                        .collect(),
                    objects: ParameterObjectMap::default(),
                },
            );
            pio.set_list(
                "Query",
                ParameterList {
                    lists: self
                        .aiprog
                        .queries
                        .values()
                        .enumerate()
                        .map(|(i, p)| (jstr!("Query_{&lexical::to_string(i)}"), p.clone()))
                        .collect(),
                    objects: ParameterObjectMap::default(),
                },
            );
            pio
        }
    }

    impl From<AIProgram> for ParameterIO {
        fn from(val: AIProgram) -> Self {
            ParameterIOBuilder::new(val).build()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use roead::aamp::*;

    #[test]
    fn serde() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = ParameterIO::from_binary(
            actor
                .get_file_data("Actor/AIProgram/Guardian_A.baiprog")
                .unwrap(),
        )
        .unwrap();
        let aiprog = super::AIProgram::try_from(&pio).unwrap();
        let data = aiprog.clone().into_pio().to_binary();
        let pio2 = ParameterIO::from_binary(&data).unwrap();
        let aiprog2 = super::AIProgram::try_from(&pio2).unwrap();
        assert_eq!(aiprog, aiprog2);
    }

    #[test]
    fn diff() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = ParameterIO::from_binary(
            actor
                .get_file_data("Actor/AIProgram/Guardian_A.baiprog")
                .unwrap(),
        )
        .unwrap();
        let aiprog = super::AIProgram::try_from(&pio).unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Guardian_A");
        let pio2 = ParameterIO::from_binary(
            actor2
                .get_file_data("Actor/AIProgram/Guardian_A.baiprog")
                .unwrap(),
        )
        .unwrap();
        let aiprog2 = super::AIProgram::try_from(&pio2).unwrap();
        let diff = aiprog.diff(&aiprog2);
        println!("{}", serde_json::to_string_pretty(&diff).unwrap());
    }

    #[test]
    fn merge() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = ParameterIO::from_binary(
            actor
                .get_file_data("Actor/AIProgram/Guardian_A.baiprog")
                .unwrap(),
        )
        .unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Guardian_A");
        let aiprog = super::AIProgram::try_from(&pio).unwrap();
        let pio2 = ParameterIO::from_binary(
            actor2
                .get_file_data("Actor/AIProgram/Guardian_A.baiprog")
                .unwrap(),
        )
        .unwrap();
        let aiprog2 = super::AIProgram::try_from(&pio2).unwrap();
        let diff = aiprog.diff(&aiprog2);
        let merged = aiprog.merge(&diff);
        assert_eq!(aiprog2, merged);
    }
}
