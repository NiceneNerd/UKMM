use std::collections::HashSet;

use join_str::jstr;
use roead::aamp::*;
use serde::{Deserialize, Serialize};
use uk_content_derive::ParamData;
use uk_ui_derive::Editable;

use crate::{
    actor::ParameterResource,
    prelude::*,
    util::{self, IndexMap},
    Result, UKError,
};

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize, Editable, ParamData)]
pub struct AIDef {
    #[name = "Name"]
    pub name: String,
    #[name = "ClassName"]
    pub class_name: String32,
    #[name = "GroupName"]
    pub group_name: String,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize, Editable)]
pub struct AIEntry {
    pub def: AIDef,
    pub params: Option<ParameterObject>,
    pub children: IndexMap<Name, ChildEntry>,
    pub behaviors: Option<IndexMap<Name, ParameterList>>,
}

impl AIEntry {
    fn full_name(&self) -> String {
        self.def.name.clone() + self.def.class_name.as_str() + self.def.group_name.as_str()
    }
}

impl Mergeable for AIEntry {
    fn diff(&self, other: &Self) -> Self {
        let mut diff = AIEntry {
            def: other.def.clone(),
            ..Default::default()
        };
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
                    Some((*k, self_child.diff(v)))
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
                new.children.insert(*k, base_child.merge(v));
            } else {
                new.children.insert(*k, v.clone());
            }
        }
        new
    }
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize, Editable)]
pub struct ActionEntry {
    pub def: AIDef,
    pub params: Option<ParameterObject>,
    pub behaviors: Option<IndexMap<Name, ParameterList>>,
}

impl ActionEntry {
    fn full_name(&self) -> String {
        self.def.name.clone() + self.def.class_name.as_str() + self.def.group_name.as_str()
    }
}

impl Mergeable for ActionEntry {
    fn diff(&self, other: &Self) -> Self {
        let mut diff = ActionEntry {
            def: other.def.clone(),
            ..Default::default()
        };
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

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Editable)]
pub enum ChildEntry {
    AI(AIEntry),
    Action(ActionEntry),
}

impl Default for ChildEntry {
    fn default() -> Self {
        Self::AI(Default::default())
    }
}

impl Mergeable for ChildEntry {
    fn diff(&self, other: &Self) -> Self {
        match (self, other) {
            (ChildEntry::AI(_), ChildEntry::Action(_))
            | (ChildEntry::Action(_), ChildEntry::AI(_)) => other.clone(),
            (ChildEntry::AI(base_ai), ChildEntry::AI(diff_ai)) => {
                ChildEntry::AI(AIEntry::diff(base_ai, diff_ai))
            }
            (ChildEntry::Action(base_action), ChildEntry::Action(diff_action)) => {
                ChildEntry::Action(ActionEntry::diff(base_action, diff_action))
            }
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        match (self, diff) {
            (ChildEntry::AI(_), ChildEntry::Action(_))
            | (ChildEntry::Action(_), ChildEntry::AI(_)) => diff.clone(),
            (ChildEntry::AI(base_ai), ChildEntry::AI(diff_ai)) => {
                ChildEntry::AI(AIEntry::merge(base_ai, diff_ai))
            }
            (ChildEntry::Action(base_action), ChildEntry::Action(diff_action)) => {
                ChildEntry::Action(ActionEntry::merge(base_action, diff_action))
            }
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize, Editable)]
pub struct AIProgram {
    pub demos:   IndexMap<Name, ChildEntry>,
    pub tree:    IndexMap<String, AIEntry>,
    pub queries: IndexMap<String, ParameterList>,
}

impl Mergeable for AIProgram {
    fn diff(&self, other: &Self) -> Self {
        Self {
            demos:   other
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
            tree:    other
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
            demos:   {
                let mut new = self.demos.clone();
                for (k, v) in &diff.demos {
                    let merged = if let Some(entry) = new.get_mut(k) {
                        entry.merge(v)
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
            tree:    {
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
    use anyhow::Context;

    use super::*;

    fn plist_to_ai(
        list: &ParameterList,
        pio: &ParameterIO,
        action_offset: usize,
    ) -> Result<AIEntry> {
        // These are all sound because this function is never called until these all
        // have been verified to exist (and of course the ParameterIO is immutable).
        let ai_list = unsafe { pio.list("AI").unwrap_unchecked() };
        let action_list = unsafe { pio.list("Action").unwrap_unchecked() };
        let behavior_list = unsafe { pio.list("Behavior").unwrap_unchecked() };
        Ok(AIEntry {
            def: list
                .object("Def")
                .ok_or_else(|| {
                    UKError::MissingAampKey("AI entry missing Def object", Some(list.into()))
                })?
                .try_into()?,
            params: list.object("SInst").cloned(),
            children: list
                .object("ChildIdx")
                .ok_or_else(|| {
                    UKError::MissingAampKey("AI entry missing ChildIdx object", Some(list.into()))
                })?
                .0
                .iter()
                .map(|(k, v)| -> Result<(Name, ChildEntry)> {
                    let idx = v.as_int()? as usize;
                    Ok((
                        *k,
                        if idx < action_offset {
                            ChildEntry::AI(plist_to_ai(
                                ai_list.lists.0.values().nth(idx).ok_or_else(|| {
                                    UKError::MissingAampKeyD(jstr!(
                                        "AI program missing AI entry at index \
                                         {&lexical::to_string(idx)}"
                                    ))
                                })?,
                                pio,
                                action_offset,
                            )?)
                        } else {
                            ChildEntry::Action(plist_to_action(
                                action_list
                                    .lists
                                    .0
                                    .values()
                                    .nth(idx - action_offset)
                                    .ok_or_else(|| {
                                        UKError::MissingAampKeyD(jstr!(
                                            "AI program missing action entry at index \
                                             {&lexical::to_string(idx)}"
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
                .map(|obj| -> Result<IndexMap<Name, ParameterList>> {
                    obj.iter()
                        .map(|(k, v)| -> Result<(Name, ParameterList)> {
                            Ok((
                                *k,
                                behavior_list
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
        // This is sound because this function is never called until the behavior list
        // has been verified to exist (and of course the ParameterIO is immutable).
        let behavior_list = unsafe { pio.list("Behavior").unwrap_unchecked() };
        Ok(ActionEntry {
            def: list
                .object("Def")
                .ok_or(UKError::MissingAampKey(
                    "Action entry missing Def object",
                    None,
                ))?
                .try_into()?,
            params: list.object("SInst").cloned(),
            behaviors: list
                .object("BehaviorIdx")
                .map(|obj| -> Result<IndexMap<Name, ParameterList>> {
                    obj.iter()
                        .map(|(k, v)| -> Result<(Name, ParameterList)> {
                            Ok((
                                *k,
                                behavior_list
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
            let ai_list = pio
                .list("AI")
                .ok_or(UKError::MissingAampKey("AI program missing AI list", None))?;
            let action_list = pio.list("Action").ok_or(UKError::MissingAampKey(
                "AI program missing Action list",
                None,
            ))?;
            if pio.list("Behavior").is_none() {
                return Err(UKError::MissingAampKey(
                    "AI program missing Behavior list",
                    None,
                ));
            }
            let query_list = pio.list("Query").ok_or(UKError::MissingAampKey(
                "AI program missing Query list",
                None,
            ))?;
            Ok(Self {
                tree:    {
                    let child_indexes: HashSet<usize> = ai_list
                        .lists
                        .0
                        .values()
                        .filter_map(|ai| {
                            ai.object("ChildIdx").map(|ci| {
                                ci.0.values()
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
                            let def = root.object("Def").ok_or_else(|| {
                                UKError::MissingAampKey(
                                    "AI entry missing Def object",
                                    Some(root.into()),
                                )
                            })?;
                            Ok((
                                def.get("ClassName")
                                    .ok_or_else(|| {
                                        UKError::MissingAampKey(
                                            "AI def missing ClassName",
                                            Some(def.into()),
                                        )
                                    })?
                                    .as_str()?
                                    .into(),
                                plist_to_ai(root, pio, action_offset)
                                    .context("Failed to parse AI entry from parameter list")?,
                            ))
                        })
                        .collect::<Result<IndexMap<_, _>>>()?
                },
                demos:   pio
                    .object("DemoAIActionIdx")
                    .ok_or(UKError::MissingAampKey(
                        "AI program missing Demo action indexes",
                        None,
                    ))?
                    .0
                    .iter()
                    .map(|(k, v)| -> Result<(Name, ChildEntry)> {
                        let idx = v.as_int()? as usize;
                        Ok((
                            *k,
                            if idx >= action_offset {
                                ChildEntry::Action(
                                    plist_to_action(
                                        action_list
                                            .lists
                                            .0
                                            .values()
                                            .nth(idx - action_offset)
                                            .ok_or_else(|| {
                                                UKError::MissingAampKeyD(jstr!(
                                                    "AI program missing demo action at index \
                                                     {&lexical::to_string(idx - action_offset)}"
                                                ))
                                            })?,
                                        pio,
                                    )
                                    .context("Failed to parse action entry from parameter list")?,
                                )
                            } else {
                                ChildEntry::AI(
                                    plist_to_ai(
                                        ai_list.lists.0.values().nth(idx).ok_or_else(|| {
                                            UKError::MissingAampKeyD(jstr!(
                                                "AI program missing demo AI at index \
                                                 {&lexical::to_string(idx)}"
                                            ))
                                        })?,
                                        pio,
                                        action_offset,
                                    )
                                    .context("Failed to parse AI entry from parameter list")?,
                                )
                            },
                        ))
                    })
                    .collect::<Result<IndexMap<Name, ChildEntry>>>()?,
                queries: query_list
                    .lists
                    .0
                    .values()
                    .cloned()
                    .map(|query| -> Result<(String, ParameterList)> {
                        let def = query.object("Def").ok_or_else(|| {
                            UKError::MissingAampKey(
                                "Query missing Def object",
                                Some((&query).into()),
                            )
                        })?;
                        Ok((
                            def.get("ClassName")
                                .ok_or_else(|| {
                                    UKError::MissingAampKey(
                                        "AI def missing ClassName",
                                        Some(def.into()),
                                    )
                                })?
                                .as_str()?
                                .into(),
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
    use crate::util::HashMap;

    fn count_ais(ai: &AIEntry) -> usize {
        1 + ai
            .children
            .values()
            .filter_map(|c| {
                match c {
                    ChildEntry::AI(ai) => Some(count_ais(ai)),
                    ChildEntry::Action(_) => None,
                }
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
                done_ais: {
                    let mut map = HashMap::default();
                    map.reserve(action_offset);
                    map
                },
                actions: vec![],
                done_actions: HashMap::default(),
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
            list.objects_mut().insert("Def", ai.def.into());
            if let Some(params) = ai.params {
                list.objects_mut().insert("SInst", params);
            };
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
            list.objects_mut().insert("ChildIdx", children);
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
                list.objects_mut().insert("BehaviorIdx", behavior_indexes);
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
            list.objects_mut().insert("Def", action.def.into());
            if let Some(params) = action.params {
                list.objects_mut().insert("SInst", params);
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
                list.objects_mut().insert("BehaviorIdx", behavior_indexes);
            };
            let idx = self.actions.len();
            self.done_actions.insert(name, idx);
            self.actions.push(list);
            idx
        }

        fn build(mut self) -> ParameterIO {
            let mut pio = ParameterIO::new();
            pio.objects_mut()
                .insert("DemoAIActionIdx", ParameterObject::new());
            let mut tree: IndexMap<String, AIEntry> = IndexMap::default();
            std::mem::swap(&mut tree, &mut self.aiprog.tree);
            let roots: Vec<AIEntry> = tree.into_iter().map(|(_, root)| root).collect();
            for root in roots {
                self.ai_to_plist(root);
            }
            let mut demos: IndexMap<Name, ChildEntry> = IndexMap::default();
            std::mem::swap(&mut self.aiprog.demos, &mut demos);
            pio.object_mut("DemoAIActionIdx")
                .unwrap()
                .0
                .extend(demos.into_iter().map(|(k, demo_child)| {
                    (k, match demo_child {
                        ChildEntry::AI(ai) => {
                            let idx = self.ai_to_plist(ai);
                            Parameter::Int(idx as i32)
                        }
                        ChildEntry::Action(action) => {
                            let idx = self.action_to_plist(action);
                            Parameter::Int((idx + self.action_offset) as i32)
                        }
                    })
                }));
            pio.set_list("AI", ParameterList {
                lists:   self
                    .ais
                    .iter()
                    .enumerate()
                    .map(|(i, p)| (jstr!("AI_{&lexical::to_string(i)}"), p.clone()))
                    .collect(),
                objects: ParameterObjectMap::default(),
            });
            pio.set_list("Action", ParameterList {
                lists:   self
                    .actions
                    .iter()
                    .enumerate()
                    .map(|(i, p)| (jstr!("Action_{&lexical::to_string(i)}"), p.clone()))
                    .collect(),
                objects: ParameterObjectMap::default(),
            });
            pio.set_list("Behavior", ParameterList {
                lists:   self
                    .behaviors
                    .iter()
                    .enumerate()
                    .map(|(i, p)| (jstr!("Behavior_{&lexical::to_string(i)}"), p.clone()))
                    .collect(),
                objects: ParameterObjectMap::default(),
            });
            pio.set_list("Query", ParameterList {
                lists:   self
                    .aiprog
                    .queries
                    .values()
                    .enumerate()
                    .map(|(i, p)| (jstr!("Query_{&lexical::to_string(i)}"), p.clone()))
                    .collect(),
                objects: ParameterObjectMap::default(),
            });
            pio
        }
    }

    impl From<AIProgram> for ParameterIO {
        fn from(val: AIProgram) -> Self {
            ParameterIOBuilder::new(val).build()
        }
    }
}

impl ParameterResource for AIProgram {
    fn path(name: &str) -> std::string::String {
        jstr!("Actor/AIProgram/{name}.baiprog")
    }
}

impl Resource for AIProgram {
    fn from_binary(data: impl AsRef<[u8]>) -> Result<Self> {
        (&ParameterIO::from_binary(data.as_ref())?).try_into()
    }

    fn into_binary(self, _endian: Endian) -> Vec<u8> {
        ParameterIO::from(self).to_binary()
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref().extension().and_then(|ext| ext.to_str()) == Some("baiprog")
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
                .get_data("Actor/AIProgram/Guardian_A.baiprog")
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let aiprog = super::AIProgram::try_from(&pio).unwrap();
        let data = roead::aamp::ParameterIO::from(aiprog.clone()).to_binary();
        let pio2 = ParameterIO::from_binary(data).unwrap();
        let aiprog2 = super::AIProgram::try_from(&pio2).unwrap();
        assert_eq!(aiprog, aiprog2);
    }

    #[test]
    fn serde_woodball() {
        let pio = ParameterIO::from_text(
            std::fs::read_to_string("test/Actor/AIProgram/WoodBall_Golf.aiprog.yml").unwrap(),
        )
        .unwrap();
        let aiprog = super::AIProgram::try_from(&pio).unwrap();
        let data = roead::aamp::ParameterIO::from(aiprog.clone()).to_binary();
        let pio2 = ParameterIO::from_binary(data).unwrap();
        let aiprog2 = super::AIProgram::try_from(&pio2).unwrap();
        assert_eq!(aiprog, aiprog2);
    }

    #[test]
    fn diff() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = ParameterIO::from_binary(
            actor
                .get_data("Actor/AIProgram/Guardian_A.baiprog")
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let aiprog = super::AIProgram::try_from(&pio).unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Guardian_A");
        let pio2 = ParameterIO::from_binary(
            actor2
                .get_data("Actor/AIProgram/Guardian_A.baiprog")
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let aiprog2 = super::AIProgram::try_from(&pio2).unwrap();
        let _diff = aiprog.diff(&aiprog2);
    }

    #[test]
    fn merge() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = ParameterIO::from_binary(
            actor
                .get_data("Actor/AIProgram/Guardian_A.baiprog")
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Guardian_A");
        let aiprog = super::AIProgram::try_from(&pio).unwrap();
        let pio2 = ParameterIO::from_binary(
            actor2
                .get_data("Actor/AIProgram/Guardian_A.baiprog")
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let aiprog2 = super::AIProgram::try_from(&pio2).unwrap();
        let diff = aiprog.diff(&aiprog2);
        dbg!(&diff);
        let merged = aiprog.merge(&diff);
        assert_eq!(aiprog2, merged);
    }

    #[test]
    fn identify() {
        let path = std::path::Path::new(
            "content/Actor/Pack/Enemy_Guardian_A.sbactorpack//Actor/AIProgram/Guardian_A.baiprog",
        );
        assert!(super::AIProgram::path_matches(path));
    }
}
