//! Pure view helpers: deriving the visible (sorted/filtered) mod list from a
//! [`Snapshot`](crate::session::Snapshot).
//!
//! Snapshot entries are always in load order. Frontends turn them into rows
//! with [`sort_entries`] + [`filter_entries`] and map rows back to [`ModId`]s,
//! so display indices never leak into load-order operations (the old GUI's
//! index-space mismatch).

use std::{cmp::Ordering, collections::HashMap};

use serde::{Deserialize, Serialize};
use uk_manager::mods::Mod;

use crate::session::{ModEntry, ModId};

/// How the mod list is sorted. Persisted in the frontend's UI state
/// (`ui.json`), so variant names and order are part of that format.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Sort {
    Enabled,
    Name,
    Category,
    Version,
    Priority,
}

pub type Orderer = dyn Fn(&(usize, Mod), &(usize, Mod)) -> Ordering;

impl Sort {
    /// Comparator over `(load index, mod)` pairs, kept for frontends that
    /// still hold `Mod` clones directly (the egui frontend today).
    pub fn orderer(&self) -> Box<Orderer> {
        match self {
            Sort::Enabled => {
                Box::new(|(_, a): &(_, Mod), (_, b): &(_, Mod)| a.enabled.cmp(&b.enabled))
            }
            Sort::Name => {
                Box::new(|(_, a): &(_, Mod), (_, b): &(_, Mod)| a.meta.name.cmp(&b.meta.name))
            }
            Sort::Category => {
                Box::new(|(_, a): &(_, Mod), (_, b): &(_, Mod)| {
                    a.meta.category.u8().cmp(&b.meta.category.u8())
                })
            }
            Sort::Version => {
                Box::new(|(_, a): &(_, Mod), (_, b): &(_, Mod)| {
                    a.meta
                        .version
                        .partial_cmp(&b.meta.version)
                        .unwrap_or(Ordering::Equal)
                })
            }
            Sort::Priority => Box::new(|&(a, _), &(b, _)| a.cmp(&b)),
        }
    }
}

/// The visible row order as mod ids: `entries` sorted by `sort` (stable,
/// so ties keep load order), reversed when `descending`. Since `entries` is
/// in load order, `Sort::Priority` preserves it.
pub fn sort_entries(entries: &[ModEntry], sort: Sort, descending: bool) -> Vec<ModId> {
    let mut idx: Vec<usize> = (0..entries.len()).collect();
    idx.sort_by(|&a, &b| {
        let ord = match sort {
            Sort::Enabled => entries[a].enabled.cmp(&entries[b].enabled),
            Sort::Name => entries[a].meta.name.cmp(&entries[b].meta.name),
            Sort::Category => {
                entries[a]
                    .meta
                    .category
                    .u8()
                    .cmp(&entries[b].meta.category.u8())
            }
            Sort::Version => {
                entries[a]
                    .meta
                    .version
                    .partial_cmp(&entries[b].meta.version)
                    .unwrap_or(Ordering::Equal)
            }
            Sort::Priority => a.cmp(&b),
        };
        if descending { ord.reverse() } else { ord }
    });
    idx.into_iter().map(|i| entries[i].id).collect()
}

/// Filters a visible id list by case-insensitive name substring, preserving
/// order. An empty filter returns the input unchanged.
pub fn filter_entries(visible: &[ModId], entries: &[ModEntry], filter: &str) -> Vec<ModId> {
    if filter.is_empty() {
        return visible.to_vec();
    }
    let filter = filter.to_lowercase();
    let by_id: HashMap<ModId, &ModEntry> = entries.iter().map(|e| (e.id, e)).collect();
    visible
        .iter()
        .copied()
        .filter(|id| {
            by_id
                .get(id)
                .map(|e| e.meta.name.to_lowercase().contains(&filter))
                .unwrap_or(false)
        })
        .collect()
}

/// The load-order index of `id` in `entries`. Drag destinations and
/// `MoveTo` use this — never a row index.
pub fn load_index_of(entries: &[ModEntry], id: ModId) -> Option<usize> {
    entries.iter().position(|e| e.id == id)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use uk_mod::{Meta, ModPlatform};

    use super::*;

    fn entry(id: usize, name: &str, enabled: bool) -> ModEntry {
        let mut meta = Meta::blank(ModPlatform::Universal);
        meta.name = name.into();
        ModEntry {
            id: ModId(id),
            meta,
            enabled,
            options: vec![],
            path: PathBuf::from(format!("/mods/{name}.zip")),
        }
    }

    fn entries() -> Vec<ModEntry> {
        vec![
            entry(10, "charlie", true),
            entry(20, "alpha", false),
            entry(30, "bravo", true),
        ]
    }

    #[test]
    fn priority_sort_keeps_load_order() {
        let e = entries();
        assert_eq!(sort_entries(&e, Sort::Priority, false), vec![
            ModId(10),
            ModId(20),
            ModId(30)
        ]);
        assert_eq!(sort_entries(&e, Sort::Priority, true), vec![
            ModId(30),
            ModId(20),
            ModId(10)
        ]);
    }

    #[test]
    fn name_sort_is_stable_and_reversible() {
        let e = entries();
        assert_eq!(sort_entries(&e, Sort::Name, false), vec![
            ModId(20),
            ModId(30),
            ModId(10)
        ]);
        assert_eq!(sort_entries(&e, Sort::Name, true), vec![
            ModId(10),
            ModId(30),
            ModId(20)
        ]);
    }

    #[test]
    fn enabled_sort_groups_disabled_first() {
        let e = entries();
        assert_eq!(sort_entries(&e, Sort::Enabled, false), vec![
            ModId(20),
            ModId(10),
            ModId(30)
        ]);
    }

    #[test]
    fn filter_is_case_insensitive_substring_on_names() {
        let e = entries();
        let visible = sort_entries(&e, Sort::Priority, false);
        assert_eq!(filter_entries(&visible, &e, "ALP"), vec![ModId(20)]);
        assert_eq!(filter_entries(&visible, &e, "a"), visible.clone());
        assert_eq!(filter_entries(&visible, &e, "zzz"), Vec::<ModId>::new());
        assert_eq!(filter_entries(&visible, &e, ""), visible);
    }

    #[test]
    fn load_index_of_maps_ids_to_load_order() {
        let e = entries();
        assert_eq!(load_index_of(&e, ModId(30)), Some(2));
        assert_eq!(load_index_of(&e, ModId(20)), Some(1));
        assert_eq!(load_index_of(&e, ModId(999)), None);
    }
}
