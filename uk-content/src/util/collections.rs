use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::hash::Hash;

use crate::prelude::Mergeable;

pub trait DeleteKey: Hash + Eq + Clone {}
impl<T> DeleteKey for T where T: Hash + Eq + Clone {}

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeleteVec<T: Clone + PartialEq>(Vec<T>, Vec<bool>);

impl<T: Clone + PartialEq> FromIterator<(T, bool)> for DeleteVec<T> {
    fn from_iter<I: IntoIterator<Item = (T, bool)>>(iter: I) -> Self {
        let (vec1, vec2) = iter.into_iter().unzip();
        Self(vec1, vec2)
    }
}

impl<T: Clone + PartialEq> FromIterator<T> for DeleteVec<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let (items, dels) = iter.into_iter().map(|item| (item, false)).unzip();
        Self(items, dels)
    }
}

impl<T: Clone + PartialEq> PartialEq for DeleteVec<T> {
    fn eq(&self, other: &Self) -> bool {
        (self.0.len() == other.0.len() && self.1.len() == other.1.len())
            && self.0.iter().zip(self.1.iter()).all(|(item, del)| {
                other.0.iter().position(|it| item == it).map(|i| other.1[i]) == Some(*del)
            })
            && other.0.iter().zip(other.1.iter()).all(|(item, del)| {
                self.0.iter().position(|it| item == it).map(|i| self.1[i]) == Some(*del)
            })
    }
}

impl<T: Clone + PartialEq> IntoIterator for DeleteVec<T> {
    type Item = T;
    type IntoIter = impl Iterator<Item = T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0
            .into_iter()
            .zip(self.1.into_iter())
            .filter_map(|(k, del)| (!del).then(|| k))
    }
}

impl<T: Clone + PartialEq> DeleteVec<T> {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.len() == 0
    }

    #[inline]
    pub fn and_delete(mut self) -> Self {
        self.delete();
        self
    }

    pub fn delete(&mut self) {
        self.0 = self
            .0
            .iter()
            .zip(self.1.iter())
            .filter_map(|(k, del)| (!*del).then(|| k.clone()))
            .collect()
    }

    pub fn deleted(&self) -> Vec<&T> {
        self.0
            .iter()
            .zip(self.1.iter())
            .filter_map(|(k, del)| (!*del).then(|| k))
            .collect()
    }

    pub fn set_delete(&mut self, item: impl Borrow<T>) {
        if let Some(i) = self.0.iter().position(|it| item.borrow() == it) {
            self.1[i] = true;
        }
    }

    pub fn is_delete(&self, item: impl Borrow<T>) -> Option<bool> {
        self.0
            .iter()
            .position(|it| item.borrow() == it)
            .map(|i| self.1[i])
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.0
            .iter()
            .zip(self.1.iter())
            .filter_map(|(k, del)| (!*del).then(|| k))
    }

    #[inline]
    pub fn contains(&self, item: impl Borrow<T>) -> bool {
        self.0.contains(item.borrow())
    }

    pub fn push(&mut self, item: T) {
        self.0.push(item);
        self.1.push(false);
    }

    pub fn push_del(&mut self, item: T) {
        self.0.push(item);
        self.1.push(true);
    }
}

impl<T: Clone + PartialEq> Mergeable for DeleteVec<T> {
    fn diff(&self, other: &Self) -> Self {
        other
            .iter()
            .filter(|it| !self.contains(*it))
            .map(|it| (it.clone(), false))
            .chain(
                self.iter()
                    .filter_map(|it| (!other.contains(it)).then(|| (it.clone(), true))),
            )
            .collect()
    }

    fn merge(&self, diff: &Self) -> Self {
        let mut all_items: Vec<T> = self
            .iter()
            .filter(|item| diff.is_delete(*item) != Some(true))
            .cloned()
            .collect();
        for (idx, item) in diff.iter().enumerate() {
            if !all_items.contains(item) {
                all_items.insert(idx, item.clone());
            }
        }
        let dels = vec![false; all_items.len()];
        Self(all_items, dels).and_delete()
    }
}

#[derive(Debug, Default, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DeleteSet<T: DeleteKey>(IndexMap<T, bool>);

impl<T: DeleteKey> IntoIterator for DeleteSet<T> {
    type Item = T;
    type IntoIter = impl Iterator<Item = T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter().filter_map(|(k, del)| (!del).then(|| k))
    }
}

impl<T: DeleteKey> From<IndexMap<T, bool>> for DeleteSet<T> {
    fn from(val: IndexMap<T, bool>) -> Self {
        Self(val)
    }
}

impl<T: DeleteKey> FromIterator<(T, bool)> for DeleteSet<T> {
    fn from_iter<I: IntoIterator<Item = (T, bool)>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<T: DeleteKey> FromIterator<T> for DeleteSet<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self(iter.into_iter().map(|item| (item, false)).collect())
    }
}

impl<T: DeleteKey> DeleteSet<T> {
    #[inline]
    pub fn and_delete(mut self) -> Self {
        self.delete();
        self
    }

    pub fn delete(&mut self) {
        self.0.retain(|_, del| !*del);
    }

    pub fn deleted(&self) -> Vec<&T> {
        self.0
            .iter()
            .filter_map(|(k, del)| (!*del).then(|| k))
            .collect()
    }

    pub fn set_delete(&mut self, item: impl Borrow<T>) {
        if let Some(del) = self.0.get_mut(item.borrow()) {
            *del = true
        }
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.0.iter().filter_map(|(k, del)| (!*del).then(|| k))
    }

    #[inline]
    pub fn contains(&self, item: impl Borrow<T>) -> bool {
        self.0.contains_key(item.borrow())
    }

    pub fn diff(&self, other: &Self) -> Self {
        other
            .iter()
            .filter(|it| !self.contains(*it))
            .map(|it| (it.clone(), false))
            .chain(
                self.iter()
                    .filter_map(|it| (!other.contains(it)).then(|| (it.clone(), true))),
            )
            .collect()
    }

    pub fn merge(&self, other: &Self) -> Self {
        Self(
            self.0
                .keys()
                .chain(other.0.keys())
                .map(|k| (k.clone(), other.0.get(k).copied().unwrap_or(false)))
                .collect(),
        )
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<T: DeleteKey> Mergeable for DeleteSet<T> {
    fn diff(&self, other: &Self) -> Self {
        other
            .iter()
            .filter(|it| !self.contains(*it))
            .map(|it| (it.clone(), false))
            .chain(
                self.iter()
                    .filter_map(|it| (!other.contains(it)).then(|| (it.clone(), true))),
            )
            .collect()
    }

    fn merge(&self, diff: &Self) -> Self {
        self.iter()
            .interleave(diff.iter())
            .cloned()
            .collect::<Self>()
            .and_delete()
    }
}

#[derive(Debug, Default, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SortedDeleteSet<T: DeleteKey + Ord>(BTreeMap<T, bool>);

impl<T: DeleteKey + Ord> IntoIterator for SortedDeleteSet<T> {
    type Item = T;
    type IntoIter = impl Iterator<Item = T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter().filter_map(|(k, del)| (!del).then(|| k))
    }
}

impl<T: DeleteKey + Ord> From<BTreeMap<T, bool>> for SortedDeleteSet<T> {
    fn from(val: BTreeMap<T, bool>) -> Self {
        Self(val)
    }
}

impl<T: DeleteKey + Ord> FromIterator<(T, bool)> for SortedDeleteSet<T> {
    fn from_iter<I: IntoIterator<Item = (T, bool)>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<T: DeleteKey + Ord> FromIterator<T> for SortedDeleteSet<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self(iter.into_iter().map(|v| (v, false)).collect())
    }
}

impl<T: DeleteKey + Ord> SortedDeleteSet<T> {
    #[inline]
    pub fn and_delete(mut self) -> Self {
        self.delete();
        self
    }

    pub fn delete(&mut self) {
        self.0.retain(|_, del| !*del);
    }

    pub fn deleted(&self) -> Vec<&T> {
        self.0
            .iter()
            .filter_map(|(k, del)| (!*del).then(|| k))
            .collect()
    }

    pub fn set_delete(&mut self, item: impl Borrow<T>) {
        if let Some(del) = self.0.get_mut(item.borrow()) {
            *del = true
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.0.iter().filter_map(|(k, del)| (!*del).then(|| k))
    }

    #[inline]
    pub fn contains(&self, item: impl Borrow<T>) -> bool {
        self.0.contains_key(item.borrow())
    }

    pub fn diff(&self, other: &Self) -> Self {
        other
            .iter()
            .filter(|it| !self.contains(*it))
            .map(|it| (it.clone(), false))
            .chain(
                self.iter()
                    .filter_map(|it| (!other.contains(it)).then(|| (it.clone(), true))),
            )
            .collect()
    }

    pub fn merge(&self, other: &Self) -> Self {
        Self(
            self.0
                .keys()
                .chain(other.0.keys())
                .map(|k| (k.clone(), other.0.get(k).copied().unwrap_or(false)))
                .collect(),
        )
        .and_delete()
    }
}

macro_rules! impl_delete_map {
    ($type:tt, $($key:tt)+) => {
        impl<T: $($key)*, U: PartialEq + Clone> IntoIterator for $type<T, U> {
            type Item = (T, U);
            type IntoIter = impl Iterator<Item = (T, U)>;

            fn into_iter(self) -> Self::IntoIter {
                let Self(items, dels) = self;
                items.into_iter().filter(move |(k, _)| !dels[k])
            }
        }

        impl<T: $($key)*, U: PartialEq + Clone> FromIterator<(T, U, bool)> for $type<T, U> {
            fn from_iter<I: IntoIterator<Item = (T, U, bool)>>(iter: I) -> Self {
                let (map1, map2) = iter
                    .into_iter()
                    .map(|(k, v, del)| ((k.clone(), v), (k, del)))
                    .unzip();
                Self(map1, map2)
            }
        }

        impl<T: $($key)*, U: PartialEq + Clone> FromIterator<(T, U)> for $type<T, U> {
            fn from_iter<I: IntoIterator<Item = (T, U)>>(iter: I) -> Self {
                let (items, dels) = iter
                    .into_iter()
                    .map(|(k, v)| ((k.clone(), v), (k, false)))
                    .unzip();
                Self(items, dels)
            }
        }

        impl<T: $($key)*, U: PartialEq + Clone> PartialEq for $type<T, U> {
            fn eq(&self, other: &Self) -> bool {
                self.0.len() == other.0.len() && self.iter().all(|(k, v)| other.get(k) == Some(v))
            }
        }

        impl<T: $($key)*, U: PartialEq + Clone> $type<T, U> {
            pub fn new() -> Self {
                Self(Default::default(), Default::default())
            }

            #[inline]
            pub fn len(&self) -> usize {
                self.0.len()
            }

            #[inline]
            pub fn is_empty(&self) -> bool {
                self.0.is_empty()
            }

            #[inline]
            pub fn and_delete(mut self) -> Self {
                self.delete();
                self
            }

            pub fn delete(&mut self) {
                self.0.retain(|k, _| !self.1[k]);
                self.1.retain(|_, v| !*v);
            }

            pub fn deleted(&self) -> IndexMap<T, U> {
                self.0
                    .iter()
                    .filter_map(|(k, v)| (!self.1[k]).then(|| (k.clone(), v.clone())))
                    .collect()
            }

            pub fn set_delete(&mut self, key: impl Borrow<T>) {
                if let Some(del_val) = self.1.get_mut(key.borrow()) {
                    *del_val = true;
                }
            }

            pub fn is_delete(&self, key: impl Borrow<T>) -> Option<bool> {
                self.1.get(key.borrow()).copied()
            }

            #[inline]
            pub fn iter(&self) -> impl Iterator<Item = (&T, &U)> {
                self.0.iter().filter(|(k, _)| !self.1[*k])
            }

            #[inline]
            pub fn keys(&self) -> impl Iterator<Item = &T> {
                self.0.keys().filter(|k| !self.1[*k])
            }

            #[inline]
            pub fn values(&self) -> impl Iterator<Item = &U> {
                self.0
                    .iter()
                    .filter_map(|(k, v)| (!self.1.get(k).unwrap()).then(|| v))
            }

            #[inline]
            pub fn contains_key(&self, key: impl Borrow<T>) -> bool {
                self.0.contains_key(key.borrow())
            }

            #[inline]
            pub fn get(&self, key: impl Borrow<T>) -> Option<&U> {
                self.0.get(key.borrow())
            }

            #[inline]
            pub fn get_mut(&mut self, key: impl Borrow<T>) -> Option<&mut U> {
                self.0.get_mut(key.borrow())
            }

            pub fn insert(&mut self, key: impl Borrow<T>, value: U) {
                self.0.insert(key.borrow().clone(), value);
                self.1.insert(key.borrow().clone(), false);
            }

            pub fn insert_del(&mut self, key: impl Borrow<T>, value: U) {
                self.0.insert(key.borrow().clone(), value);
                self.1.insert(key.borrow().clone(), true);
            }
        }

        impl<T: $($key)*, U: PartialEq + Clone> crate::prelude::Mergeable for $type<T, U> {
            fn diff(&self, other: &Self) -> Self {
                other
                    .0
                    .iter()
                    .filter_map(|(k, v)| (self.get(k) != Some(v)).then(|| (k.clone(), v.clone(), false)))
                    .chain(self.0.iter().filter_map(|(k, v)| {
                        (!other.contains_key(k)).then(|| (k.clone(), v.clone(), true))
                    }))
                    .collect()
            }

            fn merge(&self, other: &Self) -> Self {
                Self(
                    self.0
                        .iter()
                        .chain(other.0.iter())
                        .collect::<IndexMap<_, _>>()
                        .into_iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect(),
                    self.1
                        .iter()
                        .chain(other.1.iter())
                        .collect::<IndexMap<_, _>>()
                        .into_iter()
                        .map(|(k, v)| (k.clone(), *v))
                        .collect(),
                )
                .and_delete()
            }
        }

        impl<T: $($key)*, U: crate::prelude::Mergeable + Clone + PartialEq + Default> $type<T, U> {
            pub fn deep_diff(&self, other: &Self) -> Self {
                other
                    .0
                    .iter()
                    .filter_map(|(k, other_map)| {
                        if let Some(self_map) = self.get(k) {
                            if self_map != other_map {
                                Some((k.clone(), self_map.diff(other_map), false))
                            } else {
                                None
                            }
                        } else {
                            Some((k.clone(), other_map.clone(), false))
                        }
                    })
                    .chain(self.iter().filter_map(|(k, _)| {
                        (!other.contains_key(k)).then(|| (k.clone(), U::default(), true))
                    }))
                    .collect()
            }

            pub fn deep_merge(&self, diff: &Self) -> Self {
                let keys: IndexSet<_> = self.keys().chain(diff.keys()).cloned().collect();
                keys.into_iter()
                    .map(|key| {
                        let (self_map, diff_map) = (self.get(&key), diff.get(&key));
                        if let Some(self_map) = self_map && let Some(diff_map) = diff_map {
                            (key.clone(), self_map.merge(diff_map), diff.is_delete(&key).unwrap())
                        } else {
                            (key.clone(), diff_map.or(self_map).cloned().unwrap(), diff.is_delete(&key).unwrap_or_default())
                        }
                    })
                    .collect::<$type<_, _>>()
                    .and_delete()
            }
        }
    };
}

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeleteMap<T: DeleteKey, U: PartialEq + Clone>(IndexMap<T, U>, IndexMap<T, bool>);

impl_delete_map!(DeleteMap, DeleteKey);

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct SortedDeleteMap<T: DeleteKey + Ord, U: PartialEq + Clone>(
    BTreeMap<T, U>,
    BTreeMap<T, bool>,
);

impl_delete_map!(SortedDeleteMap, DeleteKey + Ord);
