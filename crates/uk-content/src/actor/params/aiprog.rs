use std::{
    collections::{BTreeMap, HashSet},
    hash::Hash,
};

use anyhow::Context;
use itertools::Itertools;
use join_str::jstr;
use roead::{aamp::*, h};
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};
use uk_content_derive::ParamData;
use uk_util::OptionResultExt;

use crate::{
    actor::ParameterResource,
    prelude::*,
    util::{self, HashMap, IndexMap, IndexSet},
    Result, UKError,
};

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub struct BehaviorMap(pub IndexMap<u32, String32>);

#[derive(Debug, Default, Clone, PartialEq, Hash, Eq, Deserialize, Serialize, ParamData)]

pub struct AIDef {
    #[name = "Name"]
    pub name: Option<String>,
    #[name = "ClassName"]
    pub class_name: String32,
    #[name = "GroupName"]
    pub group_name: Option<String>,
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Hash, Eq, Deserialize, Serialize)]

pub enum Category {
    #[default]
    AI,
    Action,
    Behavior,
    Query,
}

impl std::fmt::Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Category::AI => "AI",
            Category::Action => "Action",
            Category::Behavior => "Behavior",
            Category::Query => "Query",
        })
    }
}

#[derive(Debug, Default, PartialEq, Clone, Deserialize, Serialize)]

pub struct AIEntry {
    pub category: Category,
    pub def: AIDef,
    pub params: Option<ParameterObject>,
    pub behaviors: Option<IndexMap<Name, usize>>,
    pub children: Option<IndexMap<Name, AIEntry>>,
}

impl Mergeable for AIEntry {
    fn diff(&self, other: &Self) -> Self {
        Self {
            category: self.category,
            def: other.def.clone(),
            params: other.params.as_ref().map(|other_params| {
                self.params
                    .as_ref()
                    .map(|self_params| util::diff_pobj(self_params, other_params))
                    .unwrap_or_else(|| other_params.clone())
            }),
            behaviors: other.behaviors.as_ref().map(|other_behaviors| {
                self.behaviors
                    .as_ref()
                    .map(|self_behaviors| {
                        other_behaviors
                            .iter()
                            .filter_map(|(k, v)| {
                                (self_behaviors.get(k) != Some(v)).then_some((*k, *v))
                            })
                            .collect()
                    })
                    .unwrap_or_else(|| other_behaviors.clone())
            }),
            children: other.children.as_ref().map(|other_children| {
                self.children
                    .as_ref()
                    .map(|self_children| {
                        other_children
                            .iter()
                            .filter_map(|(k, other_entry)| {
                                if let Some(self_entry) = self_children.get(k) {
                                    (self_entry != other_entry)
                                        .then(|| (*k, self_entry.diff(other_entry)))
                                } else {
                                    Some((*k, other_entry.clone()))
                                }
                            })
                            .collect()
                    })
                    .unwrap_or_else(|| other_children.clone())
            }),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            category: self.category,
            def: diff.def.clone(),
            params: diff
                .params
                .as_ref()
                .map(|diff_params| {
                    self.params
                        .as_ref()
                        .map(|self_params| util::merge_pobj(self_params, diff_params))
                        .unwrap_or_else(|| diff_params.clone())
                })
                .or_else(|| self.params.clone()),
            behaviors: diff
                .behaviors
                .as_ref()
                .map(|diff_behaviors| {
                    self.behaviors
                        .as_ref()
                        .map(|self_behaviors| {
                            self_behaviors
                                .iter()
                                .chain(diff_behaviors.iter())
                                .map(|(k, v)| (*k, *v))
                                .collect()
                        })
                        .unwrap_or_else(|| diff_behaviors.clone())
                })
                .or_else(|| self.behaviors.clone()),
            children: diff
                .children
                .as_ref()
                .map(|diff_children| {
                    self.children
                        .as_ref()
                        .map(|self_children| {
                            let all_keys = self_children
                                .keys()
                                .chain(diff_children.keys())
                                .collect::<IndexSet<_>>();
                            all_keys
                                .into_iter()
                                .map(|key| {
                                    if let (Some(self_child), Some(diff_child)) =
                                        (self_children.get(key), diff_children.get(key))
                                    {
                                        (*key, self_child.merge(diff_child))
                                    } else {
                                        (
                                            *key,
                                            self_children
                                                .get(key)
                                                .or_else(|| diff_children.get(key))
                                                .cloned()
                                                .expect("This key has to exist, nutcase"),
                                        )
                                    }
                                })
                                .collect()
                        })
                        .unwrap_or_else(|| diff_children.clone())
                })
                .or_else(|| self.children.clone()),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]

pub struct AIProgram {
    pub demos:     IndexMap<Name, AIEntry>,
    pub behaviors: BTreeMap<usize, AIEntry>,
    pub queries:   BTreeMap<usize, AIEntry>,
    pub roots:     IndexMap<String, AIEntry>,
}

struct Parser<'a> {
    demos: &'a ParameterObject,
    ais: &'a ParameterList,
    action_offset: usize,
    actions: &'a ParameterList,
    behavior_offset: usize,
    behaviors: &'a ParameterList,
    query_offset: usize,
    queries: &'a ParameterList,
}

impl<'a> Parser<'a> {
    fn new(pio: &'a ParameterIO) -> Result<Parser<'a>> {
        let demos = pio
            .object("DemoAIActionIdx")
            .ok_or(UKError::MissingAampKey(
                "AI program missing DemoAIActionIdx",
                None,
            ))?;
        let ais = pio
            .list("AI")
            .ok_or(UKError::MissingAampKey("AI program missing AI list", None))?;
        let action_offset = ais.lists.len();
        let actions = pio.list("Action").ok_or(UKError::MissingAampKey(
            "AI program missing Action list",
            None,
        ))?;
        let behavior_offset = action_offset + actions.lists.len();
        let behaviors = pio.list("Behavior").ok_or(UKError::MissingAampKey(
            "AI program missing Behavior list",
            None,
        ))?;
        let query_offset = behavior_offset + behaviors.lists.len();
        let queries = pio.list("Query").ok_or(UKError::MissingAampKey(
            "AI program missing Query list",
            None,
        ))?;
        Ok(Self {
            demos,
            ais,
            action_offset,
            actions,
            behavior_offset,
            behaviors,
            query_offset,
            queries,
        })
    }

    fn find_demos(&self) -> Result<IndexMap<Name, AIEntry>> {
        self.demos
            .iter()
            .map(|(k, v)| -> Result<(Name, AIEntry)> {
                let index = v.as_int::<usize>().context("Demo index not an integer")?;
                let (index, parent, category) = if index >= self.query_offset {
                    (index - self.query_offset, self.queries, Category::Query)
                } else if index >= self.behavior_offset {
                    (
                        index - self.behavior_offset,
                        self.behaviors,
                        Category::Behavior,
                    )
                } else if index >= self.action_offset {
                    (index - self.action_offset, self.actions, Category::Action)
                } else {
                    (index, self.ais, Category::AI)
                };
                let entry = parent
                    .lists
                    .0
                    .values()
                    .nth(index)
                    .ok_or_else(|| {
                        UKError::MissingAampKeyD(format!(
                            "AI program missing demo at index {}",
                            index
                        ))
                    })
                    .and_then(|list| self.entry_from_list(list, category))?;
                Ok((*k, entry))
            })
            .collect()
    }

    fn entry_from_list(&self, list: &ParameterList, category: Category) -> Result<AIEntry> {
        let def = list
            .object("Def")
            .ok_or_else(|| {
                UKError::MissingAampKey("AI entry missing Def object", Some(list.into()))
            })
            .and_then(AIDef::try_from)
            .context("Failed to parse AI def")?;
        let params = list.object("SInst").cloned();
        let behaviors = list
            .object("BehaviorIdx")
            .map(|obj| -> Result<IndexMap<_, _>> {
                obj.iter()
                    .map(|(k, v)| -> Result<(Name, usize)> {
                        Ok((
                            *k,
                            v.as_int().with_context(|| {
                                format!(
                                    "Bad behavior index for {}",
                                    def.name
                                        .as_ref()
                                        .map(|n| n.as_str())
                                        .unwrap_or_else(|| def.class_name.as_str())
                                )
                            })?,
                        ))
                    })
                    .collect()
            })
            .transpose()?;
        let children = list
            .object("ChildIdx")
            .map(|obj| -> Result<IndexMap<Name, AIEntry>> {
                obj.iter()
                    .map(|(k, idx)| -> Result<(Name, AIEntry)> {
                        let index: usize = idx.as_int().with_context(|| {
                            format!(
                                "Bad child index for {}",
                                def.name
                                    .as_ref()
                                    .map(|n| n.as_str())
                                    .unwrap_or_else(|| def.class_name.as_str())
                            )
                        })?;
                        let (index, parent, category) = if index >= self.query_offset {
                            (index - self.query_offset, self.queries, Category::Query)
                        } else if index >= self.behavior_offset {
                            (
                                index - self.behavior_offset,
                                self.behaviors,
                                Category::Behavior,
                            )
                        } else if index >= self.action_offset {
                            (index - self.action_offset, self.actions, Category::Action)
                        } else {
                            (index, self.ais, Category::AI)
                        };
                        let entry = self
                            .entry_from_list(
                                parent.lists.0.values().nth(index).ok_or_else(|| {
                                    UKError::MissingAampKeyD(format!(
                                        "AI program missing {}_{} {}",
                                        category,
                                        index,
                                        def.name
                                            .as_ref()
                                            .map(|n| jstr!("as child of {&n}"))
                                            .unwrap_or_default()
                                    ))
                                })?,
                                category,
                            )
                            .with_context(|| {
                                format!(
                                    "Failed to parse child {} of AI entry {}",
                                    k.hash(),
                                    def.name
                                        .as_ref()
                                        .map(|n| n.as_str())
                                        .unwrap_or_else(|| def.class_name.as_str())
                                )
                            })?;
                        Ok((*k, entry))
                    })
                    .collect()
            })
            .transpose()?;
        Ok(AIEntry {
            category,
            def,
            params,
            behaviors,
            children,
        })
    }

    fn parse(self) -> Result<AIProgram> {
        let demos = self
            .find_demos()
            .context("Failed to parse AI program demos")?;
        let behaviors = self
            .behaviors
            .lists
            .0
            .values()
            .enumerate()
            .map(|(i, list)| {
                let entry = self.entry_from_list(list, Category::Behavior)?;
                Ok((i, entry))
            })
            .collect::<Result<_>>()
            .context("Failed to collect AI program behaviors")?;
        let queries = self
            .queries
            .lists
            .0
            .values()
            .enumerate()
            .map(|(i, list)| {
                let entry = self.entry_from_list(list, Category::Query)?;
                Ok((i, entry))
            })
            .collect::<Result<_>>()
            .context("Failed to collect AI program queries")?;
        let children: FxHashSet<usize> = self
            .ais
            .lists
            .0
            .values()
            .filter_map(|list| {
                let children = list.object("ChildIdx")?;
                Some(children.0.values().filter_map(|v| v.as_int().ok()))
            })
            .flatten()
            .collect();
        let roots = self
            .ais
            .lists
            .0
            .values()
            .enumerate()
            .map(|(i, v)| (i, v, Category::AI))
            .chain(
                self.actions
                    .lists
                    .0
                    .values()
                    .enumerate()
                    .map(|(i, v)| (i + self.action_offset, v, Category::Action)),
            )
            .filter(|&(i, ..)| !children.contains(&i))
            .map(|(_, list, category)| {
                let entry = self.entry_from_list(list, category)?;
                Ok((
                    entry
                        .def
                        .name
                        .as_ref()
                        .ok_or_else(|| {
                            UKError::MissingAampKey("AI entry def missing name", Some(list.into()))
                        })?
                        .clone(),
                    entry,
                ))
            })
            .collect::<Result<_>>()
            .context("Failed to collect AI program tree roots")?;
        Ok(AIProgram {
            demos,
            behaviors,
            queries,
            roots,
        })
    }
}

#[derive(Default)]
struct Writer {
    action_offset: usize,
    aiprog: AIProgram,
    ais: ParameterListMap,
    actions: ParameterListMap,
    behaviors: ParameterListMap,
    queries: ParameterListMap,
    finished: HashMap<AIDef, usize>,
}

impl Writer {
    fn count_ais<'list, 'entry: 'list>(
        entry: &'entry AIEntry,
        checked: &'list mut FxHashSet<&'entry AIDef>,
    ) -> usize {
        let self_count = if checked.contains(&entry.def) {
            0
        } else {
            checked.insert(&entry.def);
            1
        };
        match entry.category {
            Category::AI => {
                self_count
                    + entry
                        .children
                        .as_ref()
                        .map(|children| {
                            children
                                .values()
                                .map(|entry| Self::count_ais(entry, checked))
                                .sum()
                        })
                        .unwrap_or(0)
            }
            _ => 0,
        }
    }

    fn new(aiprog: AIProgram) -> Self {
        let mut checked: FxHashSet<&AIDef> = Default::default();
        Self {
            action_offset: aiprog
                .roots
                .values()
                .map(|root| Self::count_ais(root, &mut checked))
                .sum(),
            aiprog,
            ..Default::default()
        }
    }

    #[allow(clippy::unwrap_used)]
    fn entry_to_list(&mut self, entry: AIEntry) -> usize {
        if let Some(index) = self
            .finished
            .get(&entry.def)
            .filter(|_| matches!(entry.category, Category::AI | Category::Action))
        {
            *index
        } else {
            let AIEntry {
                category,
                def,
                params,
                behaviors,
                children,
            } = entry;
            let mut list = ParameterList::new();
            if let Some(n) = def.name.as_ref() {
                get_default_name_table().add_name(n.to_string())
            }
            list.set_object("Def", def.clone().into());
            if children.is_some() {
                list.set_object("ChildIdx", Default::default());
            }
            if let Some(behaviors) = behaviors {
                list.set_object(
                    "BehaviorIdx",
                    behaviors
                        .into_iter()
                        .map(|(k, idx)| (k, Parameter::I32(idx as i32)))
                        .collect(),
                )
            }
            if let Some(params) = params {
                list.set_object("SInst", params);
            }
            match category {
                Category::AI => {
                    let index = self.ais.len();
                    let name = format!("AI_{}", index);
                    self.ais.insert(name.clone(), list);
                    if let Some(children) = children {
                        let children: ParameterObject = children
                            .into_iter()
                            .sorted_by_cached_key(|c| {
                                !c.1.def
                                    .name
                                    .as_ref()
                                    .map(|n| n.starts_with("Demo_"))
                                    .unwrap_or(false)
                            })
                            .map(|(k, entry)| (k, Parameter::I32(self.entry_to_list(entry) as i32)))
                            .collect();
                        *self
                            .ais
                            .get_mut(&name)
                            .unwrap()
                            .object_mut("ChildIdx")
                            .unwrap() = children;
                    }
                    self.finished.insert(def, index);
                    index
                }
                Category::Action => {
                    let index = self.action_offset + self.actions.len();
                    let name = format!("Action_{}", index);
                    self.actions
                        .insert(format!("Action_{}", self.actions.len()), list);
                    if let Some(children) = children {
                        let children: ParameterObject = children
                            .into_iter()
                            .map(|(k, entry)| (k, Parameter::I32(self.entry_to_list(entry) as i32)))
                            .collect();
                        *self
                            .ais
                            .get_mut(&name)
                            .unwrap()
                            .object_mut("ChildIdx")
                            .unwrap() = children;
                    }
                    self.finished.insert(def, index);
                    index
                }
                Category::Behavior => {
                    self.behaviors
                        .insert(format!("Behavior_{}", self.behaviors.len()), list);
                    0
                }
                Category::Query => {
                    self.queries
                        .insert(format!("Query_{}", self.queries.len()), list);
                    0
                }
            }
        }
    }

    fn write(mut self) -> ParameterIO {
        let AIProgram {
            behaviors,
            demos,
            queries,
            roots,
        } = std::mem::take(&mut self.aiprog);
        for behavior in behaviors.into_values() {
            self.entry_to_list(behavior);
        }
        for query in queries.into_values() {
            self.entry_to_list(query);
        }
        for root in roots.into_values() {
            self.entry_to_list(root);
        }
        let name_table = get_default_name_table();
        let demos = demos
            .into_iter()
            .map(|(k, entry)| (k, Parameter::I32(self.entry_to_list(entry) as i32)))
            .sorted_by_cached_key(|(k, _)| name_table.get_name(k.hash(), 0, h!("DemoAIActionIdx")))
            .collect();
        let Self {
            ais,
            actions,
            behaviors,
            queries,
            ..
        } = self;
        ParameterIO::new()
            .with_object("DemoAIActionIdx", demos)
            .with_list("AI", ParameterList {
                lists: ais,
                ..Default::default()
            })
            .with_list("Action", ParameterList {
                lists: actions,
                ..Default::default()
            })
            .with_list("Behavior", ParameterList {
                lists: behaviors,
                ..Default::default()
            })
            .with_list("Query", ParameterList {
                lists: queries,
                ..Default::default()
            })
    }
}

impl Mergeable for AIProgram {
    fn diff(&self, other: &Self) -> Self {
        AIProgram {
            demos:     other
                .demos
                .iter()
                .filter_map(|(k, v)| {
                    if let Some(self_demo) = self.demos.get(k) {
                        (self_demo != v).then(|| (*k, self_demo.diff(v)))
                    } else {
                        Some((*k, v.clone()))
                    }
                })
                .collect(),
            behaviors: other
                .behaviors
                .iter()
                .filter(|&(k, v)| Some(v) != self.behaviors.get(k))
                .map(|(k, v)| (*k, v.clone()))
                .collect(),
            queries:   other
                .queries
                .iter()
                .filter_map(|(k, v)| {
                    if let Some(self_demo) = self.queries.get(k) {
                        (self_demo != v).then(|| (*k, self_demo.diff(v)))
                    } else {
                        Some((*k, v.clone()))
                    }
                })
                .collect(),
            roots:     other
                .roots
                .iter()
                .filter_map(|(k, v)| {
                    if let Some(self_demo) = self.roots.get(k) {
                        (self_demo != v).then(|| (k.clone(), self_demo.diff(v)))
                    } else {
                        Some((k.clone(), v.clone()))
                    }
                })
                .collect(),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            demos:     {
                let all_keys: HashSet<_> = self.demos.keys().chain(diff.demos.keys()).collect();
                all_keys
                    .into_iter()
                    .map(|key| {
                        if let (Some(self_demo), Some(diff_demo)) =
                            (self.demos.get(key), diff.demos.get(key))
                        {
                            (*key, self_demo.merge(diff_demo))
                        } else {
                            (
                                *key,
                                self.demos
                                    .get(key)
                                    .or_else(|| diff.demos.get(key))
                                    .cloned()
                                    .expect("This key has to exist, nutcase"),
                            )
                        }
                    })
                    .collect()
            },
            behaviors: {
                let all_keys: HashSet<_> =
                    self.behaviors.keys().chain(diff.behaviors.keys()).collect();
                all_keys
                    .into_iter()
                    .map(|key| {
                        if let (Some(self_behavior), Some(diff_behavior)) =
                            (self.behaviors.get(key), diff.behaviors.get(key))
                        {
                            (*key, self_behavior.merge(diff_behavior))
                        } else {
                            (
                                *key,
                                self.behaviors
                                    .get(key)
                                    .or_else(|| diff.behaviors.get(key))
                                    .cloned()
                                    .expect("This key has to exist, nutcase"),
                            )
                        }
                    })
                    .collect()
            },
            queries:   {
                let all_keys: HashSet<_> = self.queries.keys().chain(diff.queries.keys()).collect();
                all_keys
                    .into_iter()
                    .map(|key| {
                        if let (Some(self_query), Some(diff_query)) =
                            (self.queries.get(key), diff.queries.get(key))
                        {
                            (*key, self_query.merge(diff_query))
                        } else {
                            (
                                *key,
                                self.queries
                                    .get(key)
                                    .or_else(|| diff.queries.get(key))
                                    .cloned()
                                    .expect("This key has to exist, nutcase"),
                            )
                        }
                    })
                    .collect()
            },
            roots:     {
                let all_keys: HashSet<_> = self.roots.keys().chain(diff.roots.keys()).collect();
                all_keys
                    .into_iter()
                    .map(|key| {
                        if let (Some(self_root), Some(diff_root)) =
                            (self.roots.get(key), diff.roots.get(key))
                        {
                            (key.clone(), self_root.merge(diff_root))
                        } else {
                            (
                                key.clone(),
                                self.roots
                                    .get(key)
                                    .or_else(|| diff.roots.get(key))
                                    .cloned()
                                    .expect("This key has to exist, nutcase"),
                            )
                        }
                    })
                    .collect()
            },
        }
    }
}

impl TryFrom<&ParameterIO> for AIProgram {
    type Error = UKError;

    fn try_from(pio: &ParameterIO) -> Result<Self> {
        Parser::new(pio).and_then(|p| p.parse())
    }
}

impl From<AIProgram> for ParameterIO {
    fn from(aiprog: AIProgram) -> Self {
        Writer::new(aiprog).write()
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
        path.as_ref()
            .extension()
            .and_then(|ext| ext.to_str())
            .contains(&"baiprog")
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
                .get_data("Actor/AIProgram/Guardian_A.baiprog")
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
    fn diff() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = ParameterIO::from_binary(
            actor
                .get_data("Actor/AIProgram/Guardian_A.baiprog")
                .unwrap(),
        )
        .unwrap();
        let aiprog = super::AIProgram::try_from(&pio).unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Guardian_A");
        let pio2 = ParameterIO::from_binary(
            actor2
                .get_data("Actor/AIProgram/Guardian_A.baiprog")
                .unwrap(),
        )
        .unwrap();
        let aiprog2 = super::AIProgram::try_from(&pio2).unwrap();
        let _diff = aiprog.diff(&aiprog2);
        dbg!(_diff);
    }

    #[test]
    fn merge() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = ParameterIO::from_binary(
            actor
                .get_data("Actor/AIProgram/Guardian_A.baiprog")
                .unwrap(),
        )
        .unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Guardian_A");
        let aiprog = super::AIProgram::try_from(&pio).unwrap();
        let pio2 = ParameterIO::from_binary(
            actor2
                .get_data("Actor/AIProgram/Guardian_A.baiprog")
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
