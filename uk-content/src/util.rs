use indexmap::IndexMap;
use roead::aamp::*;
use roead::byml::Byml;
use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::hash::Hash;

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
        let mut all_items: Vec<T> = self
            .iter()
            .filter(|item| other.is_delete(*item) != Some(true))
            .cloned()
            .collect();
        for (idx, item) in other.iter().enumerate() {
            if !all_items.contains(item) {
                all_items.insert(idx, item.clone());
            }
        }
        let dels = vec![false; all_items.len()];
        Self(all_items, dels)
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
}

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeleteMap<T: DeleteKey, U: PartialEq + Clone>(IndexMap<T, U>, IndexMap<T, bool>);

impl<T: DeleteKey, U: PartialEq + Clone> IntoIterator for DeleteMap<T, U> {
    type Item = (T, U);
    type IntoIter = impl Iterator<Item = (T, U)>;

    fn into_iter(self) -> Self::IntoIter {
        let Self(items, dels) = self;
        items.into_iter().filter(move |(k, _)| !dels[k])
    }
}

impl<T: DeleteKey, U: PartialEq + Clone> FromIterator<(T, U, bool)> for DeleteMap<T, U> {
    fn from_iter<I: IntoIterator<Item = (T, U, bool)>>(iter: I) -> Self {
        let (map1, map2) = iter
            .into_iter()
            .map(|(k, v, del)| ((k.clone(), v), (k, del)))
            .unzip();
        Self(map1, map2)
    }
}

impl<T: DeleteKey, U: PartialEq + Clone> FromIterator<(T, U)> for DeleteMap<T, U> {
    fn from_iter<I: IntoIterator<Item = (T, U)>>(iter: I) -> Self {
        let (items, dels) = iter
            .into_iter()
            .map(|(k, v)| ((k.clone(), v), (k, false)))
            .unzip();
        Self(items, dels)
    }
}

impl<T: DeleteKey, U: PartialEq + Clone> PartialEq for DeleteMap<T, U> {
    fn eq(&self, other: &Self) -> bool {
        self.0.len() == other.0.len() && self.iter().all(|(k, v)| other.get(k) == Some(v))
    }
}

impl<T: DeleteKey, U: PartialEq + Clone> DeleteMap<T, U> {
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
        self.0.retain(|k, _| !self.1[k])
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

    pub fn diff(&self, other: &Self) -> Self {
        other
            .0
            .iter()
            .filter_map(|(k, v)| (self.get(k) != Some(v)).then(|| (k.clone(), v.clone(), false)))
            .chain(self.0.iter().filter_map(|(k, v)| {
                (!other.contains_key(k)).then(|| (k.clone(), v.clone(), true))
            }))
            .collect()
    }

    pub fn merge(&self, other: &Self) -> Self {
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

#[derive(Debug, Default, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SortedDeleteMap<T: DeleteKey + Ord, U: PartialEq + Clone>(
    BTreeMap<T, U>,
    BTreeMap<T, bool>,
);

impl<T: DeleteKey + Ord, U: PartialEq + Clone> IntoIterator for SortedDeleteMap<T, U> {
    type Item = (T, U);
    type IntoIter = impl Iterator<Item = (T, U)>;

    fn into_iter(self) -> Self::IntoIter {
        let Self(items, dels) = self;
        items.into_iter().filter(move |(k, _)| !dels[k])
    }
}

impl<T: DeleteKey + Ord, U: PartialEq + Clone> FromIterator<(T, U, bool)>
    for SortedDeleteMap<T, U>
{
    fn from_iter<I: IntoIterator<Item = (T, U, bool)>>(iter: I) -> Self {
        let (map1, map2) = iter
            .into_iter()
            .map(|(k, v, del)| ((k.clone(), v), (k, del)))
            .unzip();
        Self(map1, map2)
    }
}

impl<T: DeleteKey + Ord, U: PartialEq + Clone> FromIterator<(T, U)> for SortedDeleteMap<T, U> {
    fn from_iter<I: IntoIterator<Item = (T, U)>>(iter: I) -> Self {
        let (items, dels) = iter
            .into_iter()
            .map(|(k, v)| ((k.clone(), v), (k, false)))
            .unzip();
        Self(items, dels)
    }
}

impl<T: DeleteKey + Ord, U: PartialEq + Clone> SortedDeleteMap<T, U> {
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
        self.0.retain(|k, _| !self.1[k])
    }

    pub fn deleted(&self) -> BTreeMap<T, U> {
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

    pub fn diff(&self, other: &Self) -> Self {
        other
            .0
            .iter()
            .filter_map(|(k, v)| (self.get(k) != Some(v)).then(|| (k.clone(), v.clone(), false)))
            .chain(self.0.iter().filter_map(|(k, v)| {
                (!other.contains_key(k)).then(|| (k.clone(), v.clone(), true))
            }))
            .collect()
    }

    pub fn merge(&self, other: &Self) -> Self {
        Self(
            self.0
                .iter()
                .chain(other.0.iter())
                .collect::<BTreeMap<_, _>>()
                .into_iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            self.1
                .iter()
                .chain(other.1.iter())
                .collect::<BTreeMap<_, _>>()
                .into_iter()
                .map(|(k, v)| (k.clone(), *v))
                .collect(),
        )
    }
}

pub fn diff_plist<P: ParamList + From<ParameterList>>(base: &P, other: &P) -> P {
    ParameterList {
        lists: other
            .lists()
            .0
            .iter()
            .filter_map(|(k, v)| {
                if !base.lists().0.contains_key(k) {
                    Some((*k, v.clone()))
                } else if base.lists().0[k] != *v {
                    Some((*k, diff_plist(&base.lists().0[k], v)))
                } else {
                    None
                }
            })
            .collect(),
        objects: other
            .objects()
            .0
            .iter()
            .filter_map(|(k, v)| {
                if !base.objects().0.contains_key(k) {
                    Some((*k, v.clone()))
                } else if base.objects().0[k] != *v {
                    Some((*k, diff_pobj(&base.objects().0[k], v)))
                } else {
                    None
                }
            })
            .collect(),
    }
    .into()
}

pub fn diff_pobj(base: &ParameterObject, other: &ParameterObject) -> ParameterObject {
    other
        .0
        .iter()
        .filter_map(|(k, v)| {
            if !base.0.contains_key(k) || base.0[k] != *v {
                Some((*k, v.clone()))
            } else {
                None
            }
        })
        .collect()
}

pub fn merge_plist<P: ParamList + From<ParameterList>>(base: &P, diff: &P) -> P {
    ParameterList {
        objects: {
            let mut new = base.objects().clone();
            for (k, v) in &diff.objects().0 {
                if !new.0.contains_key(k) {
                    new.0.insert(*k, v.clone());
                } else {
                    new.0[k] = merge_pobj(&new.0[k], v);
                }
            }
            new
        },
        lists: {
            let mut new = base.lists().clone();
            for (k, v) in &diff.lists().0 {
                if !new.0.contains_key(k) {
                    new.0.insert(*k, v.clone());
                } else {
                    new.0[k] = merge_plist(&new.0[k], v);
                }
            }
            new
        },
    }
    .into()
}

pub fn merge_pobj(base: &ParameterObject, diff: &ParameterObject) -> ParameterObject {
    base.0
        .iter()
        .chain(diff.0.iter())
        .map(|(k, v)| (*k, v.clone()))
        .collect()
}

pub fn diff_byml_shallow(base: &Byml, other: &Byml) -> Byml {
    if let Byml::Hash(base) = &base && let &Byml::Hash(other) = &other {
        Byml::Hash(other.iter().filter_map(|(key, value)| {
            if base.get(key) != Some(value) {
                Some((key.clone(), value.clone()))
            } else {
                None
            }
        }).chain(
            base.keys().filter_map(|key| (!other.contains_key(key)).then(|| (key.clone(), Byml::Null)))
        ).collect())
    } else {
        panic!("Can only shallow diff BYML hashes")
    }
}

pub fn merge_byml_shallow(base: &Byml, diff: &Byml) -> Byml {
    if let Byml::Hash(base) = &base && let &Byml::Hash(diff) = &diff {
        Byml::Hash(base.iter().chain(diff.iter()).map(|(k, v)| (k.to_owned(), v.clone())).collect())
    } else {
        panic!("Can only shallow merge BYML hashes")
    }
}

pub fn simple_index_diff<T: Clone + PartialEq>(
    base: &BTreeMap<usize, T>,
    other: &BTreeMap<usize, T>,
) -> BTreeMap<usize, T> {
    other
        .iter()
        .filter_map(|(i, other_item)| {
            (base.get(i) != Some(other_item)).then(|| (*i, other_item.clone()))
        })
        .collect()
}

pub fn simple_index_merge<T: Clone + PartialEq>(
    base: &BTreeMap<usize, T>,
    diff: &BTreeMap<usize, T>,
) -> BTreeMap<usize, T> {
    base.iter()
        .chain(diff.iter())
        .map(|(i, body)| (*i, body.clone()))
        .collect()
}
