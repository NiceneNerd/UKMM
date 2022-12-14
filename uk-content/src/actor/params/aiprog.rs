mod ui;
use std::{
    collections::{BTreeMap, HashSet},
    hash::Hash,
};

use anyhow::Context;
use join_str::jstr;
use roead::aamp::*;
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};
use uk_content_derive::ParamData;
use uk_ui_derive::Editable;

use crate::{
    actor::ParameterResource,
    prelude::*,
    util::{self, DeleteMap, IndexMap},
    Result, UKError,
};

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub struct BehaviorMap(pub IndexMap<u32, String32>);

#[derive(
    Debug, Default, Clone, PartialEq, Hash, Eq, Deserialize, Serialize, Editable, ParamData,
)]
pub struct AIDef {
    #[name = "Name"]
    pub name: Option<String>,
    #[name = "ClassName"]
    pub class_name: String32,
    #[name = "GroupName"]
    pub group_name: Option<String>,
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Hash, Eq, Deserialize, Serialize, Editable)]
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

#[derive(Debug, Default, PartialEq, Clone, Deserialize, Serialize, Editable)]
pub struct AIEntry {
    pub category: Category,
    pub def: AIDef,
    pub params: Option<ParameterObject>,
    pub behaviors: Option<IndexMap<Name, usize>>,
    pub children: Option<IndexMap<Name, AIEntry>>,
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize, Editable)]
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
                let index = v.as_int().context("Demo index not an integer")? as usize;
                let (index, parent, category) = if index > self.query_offset {
                    (index - self.query_offset, self.queries, Category::Query)
                } else if index > self.behavior_offset {
                    (
                        index - self.behavior_offset,
                        self.behaviors,
                        Category::Behavior,
                    )
                } else if index > self.action_offset {
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
                            })? as usize,
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
                        let index = idx.as_int().with_context(|| {
                            format!(
                                "Bad child index for {}",
                                def.name
                                    .as_ref()
                                    .map(|n| n.as_str())
                                    .unwrap_or_else(|| def.class_name.as_str())
                            )
                        })? as usize;
                        let (index, parent, category) = if index > self.query_offset {
                            (index - self.query_offset, self.queries, Category::Query)
                        } else if index > self.behavior_offset {
                            (
                                index - self.behavior_offset,
                                self.behaviors,
                                Category::Behavior,
                            )
                        } else if index > self.action_offset {
                            (index - self.action_offset, self.actions, Category::Action)
                        } else {
                            (index, self.ais, Category::AI)
                        };
                        let entry = self
                            .entry_from_list(
                                parent.lists.0.values().nth(index).ok_or_else(|| {
                                    UKError::MissingAampKeyD(format!(
                                        "AI program missing {}_{}",
                                        category, index
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
                Some(
                    children
                        .0
                        .values()
                        .filter_map(|v| v.as_int().ok().map(|i| i as usize)),
                )
            })
            .flatten()
            .collect();
        let roots = self
            .ais
            .lists
            .0
            .values()
            .enumerate()
            .filter_map(|(i, list)| {
                (!children.contains(&i)).then(|| -> Result<(String, AIEntry)> {
                    let entry = self.entry_from_list(list, Category::AI)?;
                    Ok((
                        entry
                            .def
                            .name
                            .as_ref()
                            .ok_or_else(|| {
                                UKError::MissingAampKey(
                                    "AI entry def missing name",
                                    Some(list.into()),
                                )
                            })?
                            .clone(),
                        entry,
                    ))
                })
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

impl Mergeable for AIProgram {
    fn diff(&self, other: &Self) -> Self {
        todo!()
    }

    fn merge(&self, diff: &Self) -> Self {
        todo!()
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
        todo!()
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
        dbg!(_diff);
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
