use std::{borrow::Borrow, collections::BTreeMap, hash::Hash, vec};

use itertools::Itertools;

use crate::prelude::Mergeable;

pub type HashMap<K, V> = rustc_hash::FxHashMap<K, V>;
pub type HashSet<K> = rustc_hash::FxHashSet<K>;
pub type IndexMap<K, V> =
    indexmap::IndexMap<K, V, std::hash::BuildHasherDefault<rustc_hash::FxHasher>>;
pub type IndexSet<V> = indexmap::IndexSet<V, std::hash::BuildHasherDefault<rustc_hash::FxHasher>>;
pub trait DeleteKey: Hash + Eq + Clone {}
impl<T> DeleteKey for T where T: Hash + Eq + Clone {}

pub struct DeleteIterator<I, T>
where
    I: Iterator<Item = (T, bool)>,
{
    inner: I,
}

impl<I, T> Iterator for DeleteIterator<I, T>
where
    I: Iterator<Item = (T, bool)>,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .and_then(|(item, del)| (!del).then_some(item))
    }
}

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeleteVec<T: Clone + PartialEq>(Vec<(T, bool)>);

impl<T: Clone + PartialEq> FromIterator<(T, bool)> for DeleteVec<T> {
    fn from_iter<I: IntoIterator<Item = (T, bool)>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<T: Clone + PartialEq> FromIterator<T> for DeleteVec<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self(iter.into_iter().map(|item| (item, false)).collect())
    }
}

impl<T: Clone + PartialEq> PartialEq for DeleteVec<T> {
    fn eq(&self, other: &Self) -> bool {
        (self.0.len() == other.0.len())
            && self.0.iter().all(|(item, del)| {
                other
                    .0
                    .iter()
                    .position(|(it, _)| item == it)
                    .map(|i| other.0[i].1)
                    == Some(*del)
            })
            && other.0.iter().all(|(item, del)| {
                self.0
                    .iter()
                    .position(|(it, _)| item == it)
                    .map(|i| self.0[i].1)
                    == Some(*del)
            })
    }
}

impl<T: Clone + PartialEq> IntoIterator for DeleteVec<T> {
    type IntoIter = DeleteIterator<vec::IntoIter<(T, bool)>, T>;
    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        DeleteIterator {
            inner: self.0.into_iter(),
        }
    }
}

impl<T: Clone + PartialEq> DeleteVec<T> {
    pub fn len(&self) -> usize {
        self.iter().count()
    }

    pub fn is_empty(&self) -> bool {
        self.0.len() == 0
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        if let Some((val, false)) = self.0.get(index) {
            Some(val)
        } else {
            None
        }
    }

    #[inline]
    pub fn and_delete(mut self) -> Self {
        self.delete();
        self
    }

    pub fn delete(&mut self) {
        self.0.retain(|(_, del)| !*del);
    }

    pub fn deleted(&self) -> Vec<&T> {
        self.0
            .iter()
            .filter_map(|(k, del)| (!*del).then_some(k))
            .collect()
    }

    pub fn set_delete(&mut self, item: impl Borrow<T>) {
        if let Some(i) = self.0.iter().position(|(it, _)| item.borrow() == it) {
            self.0[i].1 = true;
        }
    }

    pub fn is_delete(&self, item: impl Borrow<T>) -> Option<bool> {
        self.0
            .iter()
            .find_map(|(it, del)| (item.borrow() == it).then_some(*del))
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.0.iter().filter_map(|(k, del)| (!*del).then_some(k))
    }

    #[inline]
    pub fn contains(&self, item: impl Borrow<T>) -> bool {
        self.0
            .iter()
            .any(|(it, del)| (item.borrow() == it) && !*del)
    }

    pub fn push(&mut self, item: T) {
        self.0.push((item, false));
    }

    pub fn push_del(&mut self, item: T) {
        self.0.push((item, true));
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
                    .filter(|&it| (!other.contains(it)))
                    .map(|it| (it.clone(), true)),
            )
            .collect()
    }

    fn merge(&self, diff: &Self) -> Self {
        let mut all_items: Vec<(T, bool)> = self
            .iter()
            .filter(|item| diff.is_delete(*item) != Some(true))
            .cloned()
            .map(|item| (item, false))
            .collect();
        for (idx, item) in diff.iter().enumerate() {
            if !all_items.iter().any(|(it, _)| it == item) {
                all_items.insert(idx, (item.clone(), false));
            }
        }
        Self(all_items).and_delete()
    }
}

#[derive(Debug, Default, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DeleteSet<T: DeleteKey>(IndexMap<T, bool>);

impl<T: DeleteKey> IntoIterator for DeleteSet<T> {
    type IntoIter = DeleteIterator<indexmap::map::IntoIter<T, bool>, T>;
    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        DeleteIterator {
            inner: self.0.into_iter(),
        }
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
            .filter_map(|(k, del)| (!*del).then_some(k))
            .collect()
    }

    pub fn set_delete(&mut self, item: impl Borrow<T>) {
        if let Some(del) = self.0.get_mut(item.borrow()) {
            *del = true
        }
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.0.iter().filter_map(|(k, del)| (!*del).then_some(k))
    }

    #[inline]
    pub fn insert(&mut self, item: T) {
        self.0.insert(item, false);
    }

    #[inline]
    pub fn contains(&self, item: impl Borrow<T>) -> bool {
        self.0.contains_key(item.borrow())
    }

    /*
    pub fn diff(&self, other: &Self) -> Self {
        other
            .iter()
            .filter(|it| !self.contains(*it))
            .map(|it| (it.clone(), false))
            .chain(
                self.iter()
                    .filter(|&it| (!other.contains(it)))
                    .map(|it| (it.clone(), true)),
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
    */

    pub fn len(&self) -> usize {
        self.iter().count()
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
                    .filter(|&it| (!other.contains(it)))
                    .map(|it| (it.clone(), true)),
            )
            .collect()
    }

    fn merge(&self, diff: &Self) -> Self {
        self.iter()
            .chain(diff.iter())
            .cloned()
            .collect::<Self>()
            .and_delete()
    }
}

#[derive(Debug, Default, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SortedDeleteSet<T: DeleteKey + Ord>(BTreeMap<T, bool>);

impl<T: DeleteKey + Ord> IntoIterator for SortedDeleteSet<T> {
    type IntoIter = DeleteIterator<std::collections::btree_map::IntoIter<T, bool>, T>;
    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        DeleteIterator {
            inner: self.0.into_iter(),
        }
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
            .filter_map(|(k, del)| (!*del).then_some(k))
            .collect()
    }

    pub fn set_delete(&mut self, item: impl Borrow<T>) {
        if let Some(del) = self.0.get_mut(item.borrow()) {
            *del = true
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.iter().count()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.0.iter().filter_map(|(k, del)| (!*del).then_some(k))
    }

    #[inline]
    pub fn extend(&mut self, iter: impl IntoIterator<Item = T>) {
        self.0.extend(iter.into_iter().map(|t| (t, false)));
    }

    #[inline]
    pub fn iter_full(&self) -> impl Iterator<Item = (&T, &bool)> {
        self.0.iter()
    }

    #[inline]
    pub fn iter_full_mut(&mut self) -> impl Iterator<Item = (&T, &mut bool)> {
        self.0.iter_mut()
    }

    #[inline]
    pub fn contains(&self, item: impl Borrow<T>) -> bool {
        self.0.contains_key(item.borrow())
    }

    #[inline]
    pub fn insert(&mut self, item: T) {
        self.0.insert(item, false);
    }
}

impl<T: DeleteKey + Ord> Mergeable for SortedDeleteSet<T> {
    fn diff(&self, other: &Self) -> Self {
        other
            .iter()
            .filter(|it| !self.contains(*it))
            .map(|it| (it.clone(), false))
            .chain(
                self.iter()
                    .filter(|&it| (!other.contains(it)))
                    .map(|it| (it.clone(), true)),
            )
            .collect()
    }

    fn merge(&self, other: &Self) -> Self {
        Self(
            self.0
                .keys()
                .chain(other.0.keys())
                .map(|k| (k.clone(), other.0.get(k).copied().unwrap_or(false)))
                .collect(),
        ).and_delete()
    }
}

pub struct DeleteMapIterator<I, T, U>
where
    I: Iterator<Item = (T, (U, bool))>,
{
    inner: I,
}

impl<I, T, U> Iterator for DeleteMapIterator<I, T, U>
where
    I: Iterator<Item = (T, (U, bool))>,
{
    type Item = (T, U);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .and_then(|(key, (val, del))| (!del).then_some((key, val)))
    }
}

macro_rules! impl_delete_map {
    ($type:tt, $inner:ty, $($key:tt)+) => {
        impl<T: $($key)*, U: PartialEq + Clone> IntoIterator for $type<T, U> {
            type IntoIter = DeleteMapIterator<<$inner as std::iter::IntoIterator>::IntoIter, T, U>;
            type Item = (T, U);

            fn into_iter(self) -> Self::IntoIter {
                DeleteMapIterator {
                    inner: self.0.into_iter(),
                }
            }
        }

        impl<T: $($key)*, U: PartialEq + Clone> FromIterator<(T, U, bool)> for $type<T, U> {
            fn from_iter<I: IntoIterator<Item = (T, U, bool)>>(iter: I) -> Self {
                Self(iter.into_iter().map(|(k, v, del)| (k, (v, del))).collect())
            }
        }

        impl<T: $($key)*, U: PartialEq + Clone> FromIterator<(T, U)> for $type<T, U> {
            fn from_iter<I: IntoIterator<Item = (T, U)>>(iter: I) -> Self {
                Self(iter.into_iter().map(|(k, v)| (k, (v, false))).collect())
            }
        }

        impl<T: $($key)*, U: PartialEq + Clone> PartialEq for $type<T, U> {
            fn eq(&self, other: &Self) -> bool {
                self.0.len() == other.0.len() && self.iter().all(|(k, v)| other.get(k) == Some(v))
            }
        }

        impl<T: $($key)*, U: PartialEq + Clone> $type<T, U> {
            #[inline]
            pub fn new() -> Self {
                Self(Default::default())
            }

            #[inline]
            pub fn len(&self) -> usize {
                self.iter().count()
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

            #[inline]
            pub fn delete(&mut self) {
                self.0.retain(|_, (_, del)| !*del);
            }

            // pub fn deleted(&self) -> IndexMap<T, U> {
            //     self.0
            //         .iter()
            //         .filter_map(|(k, v)| (!self.1[k]).then(|| (k.clone(), v.clone())))
            //         .collect()
            // }

            #[inline]
            pub fn set_delete(&mut self, key: impl Borrow<T>) {
                if let Some((_, del)) = self.0.get_mut(key.borrow()) {
                    *del = true;
                }
            }

            #[inline]
            pub fn is_delete(&self, key: impl Borrow<T>) -> Option<bool> {
                self.0.get(key.borrow()).map(|(_, del)| del).copied()
            }

            #[inline]
            pub fn iter(&self) -> impl Iterator<Item = (&T, &U)> {
                self.0.iter().filter_map(|(k, (v, del))| (!*del).then(|| (k, v)))
            }

            #[inline]
            pub fn iter_full(&self) -> impl Iterator<Item = (&T, &(U, bool))> {
                self.0.iter()
            }

            #[inline]
            pub fn iter_mut(&mut self) -> impl Iterator<Item = (&T, &mut U)> {
                self.0.iter_mut().filter_map(|(k, (v, del))| (!*del).then(|| (k, v)))
            }

            #[inline]
            pub fn iter_full_mut(&mut self) -> impl Iterator<Item = (&T, &mut (U, bool))> {
                self.0.iter_mut()
            }

            #[inline]
            pub fn keys(&self) -> impl Iterator<Item = &T> {
                self.0.iter().filter_map(|(k, (_, del))| (!*del).then(|| k))
            }

            #[inline]
            pub fn values(&self) -> impl Iterator<Item = &U> {
                self.0
                    .values()
                    .filter_map(|(v, del)| (!*del).then(|| v))
            }

            #[inline]
            pub fn contains_key(&self, key: impl Borrow<T>) -> bool {
                self.0.contains_key(key.borrow())
            }

            #[inline]
            pub fn get(&self, key: impl Borrow<T>) -> Option<&U> {
                self.0.get(key.borrow()).and_then(|(v, del)| (!*del).then(|| v))
            }

            #[inline]
            pub fn get_mut(&mut self, key: impl Borrow<T>) -> Option<&mut U> {
                self.0.get_mut(key.borrow()).and_then(|(v, del)| (!*del).then(|| v))
            }

            #[inline]
            pub fn get_or_insert_default(&mut self, key: impl Into<T>) -> &mut U where U: Default {
                &mut self.0.entry(key.into()).or_insert_with(|| (Default::default(), false)).0
            }

            #[inline]
            pub fn get_or_insert_with(&mut self, key: impl Into<T>, with: impl Fn() -> U) -> &mut U {
                &mut self.0.entry(key.into()).or_insert_with(|| (with(), false)).0
            }

            #[inline]
            pub fn insert(&mut self, key: impl Into<T>, value: U) {
                self.0.insert(key.into(), (value, false));
            }

            #[inline]
            pub fn insert_del(&mut self, key: impl Borrow<T>, value: U) {
                self.0.insert(key.borrow().clone(), (value, true));
            }

            #[inline]
            pub fn extend(&mut self, other: impl IntoIterator<Item = (T, U)>) {
                for (k, v) in other {
                    self.0.insert(k.clone(), (v, false));
                }
            }
        }

        impl<T: $($key)*, U: PartialEq + Clone> crate::prelude::Mergeable for $type<T, U> {
            fn diff(&self, other: &Self) -> Self {
                other
                    .0
                    .iter()
                    .filter_map(|(k, (v, del))| (self.get(k) != Some(v) && !*del).then(|| (k.clone(), v.clone(), false)))
                    .chain(self.0.iter().filter_map(|(k, (v, _))| {
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
                        .filter_map(|(k, (v, del))| (!*del).then(|| (k.clone(), (v.clone(), false))))
                        .collect(),
                )
            }
        }

        impl<T: $($key)*, U: crate::prelude::Mergeable + Clone + PartialEq + Default> $type<T, U> {
            pub fn deep_diff(&self, other: &Self) -> Self {
                other
                    // .0
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
                        if let (Some(self_map), Some(diff_map)) = (self_map, diff_map) {
                            (key.clone(), self_map.merge(diff_map), unsafe {
                                // We know this is sound because we just checked that `key`
                                // is in `diff`.
                                diff.is_delete(&key).unwrap_unchecked()
                            })
                        } else {
                            (key.clone(), unsafe {
                                // We know this is sound because the key had to come from
                                // one of these two maps.
                                diff_map.or(self_map).cloned().unwrap_unchecked()
                            }, diff.is_delete(&key).unwrap_or_default())
                        }
                    })
                    .collect::<$type<_, _>>()
                    .and_delete()
            }
        }
    };
}

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeleteMap<T: DeleteKey, U: PartialEq + Clone>(IndexMap<T, (U, bool)>);

impl_delete_map!(DeleteMap, IndexMap<T, (U, bool)>, DeleteKey);

impl<T: DeleteKey, U: PartialEq + Clone> DeleteMap<T, U> {
    pub fn with_capacity(capacity: usize) -> Self {
        let mut map = IndexMap::default();
        map.reserve(capacity);
        Self(map)
    }
}

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct SortedDeleteMap<T: DeleteKey + Ord, U: PartialEq + Clone>(BTreeMap<T, (U, bool)>);

impl_delete_map!(SortedDeleteMap, BTreeMap<T, (U, bool)>, DeleteKey + Ord);
