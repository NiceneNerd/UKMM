use indexmap::IndexMap;
use roead::aamp::*;
use roead::byml::Byml;
use std::borrow::Borrow;
use std::hash::Hash;

pub trait DeleteKey: Hash + Eq + Clone {}
impl<T> DeleteKey for T where T: Hash + Eq + Clone {}

#[derive(Debug, Default, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DeleteList<T: Hash + Eq>(IndexMap<T, bool>);

impl<T: DeleteKey> From<IndexMap<T, bool>> for DeleteList<T> {
    fn from(val: IndexMap<T, bool>) -> Self {
        Self(val)
    }
}

impl<T: DeleteKey> FromIterator<(T, bool)> for DeleteList<T> {
    fn from_iter<I: IntoIterator<Item = (T, bool)>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<T: DeleteKey> DeleteList<T> {
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
        self.0.get_mut(item.borrow()).map(|del| *del = true);
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.0.iter().filter_map(|(k, del)| (!*del).then(|| k))
    }

    #[inline]
    pub fn into_iter(self) -> impl Iterator<Item = T> {
        self.0.into_iter().filter_map(|(k, del)| (!del).then(|| k))
    }

    #[inline]
    pub fn contains(&self, item: impl Borrow<T>) -> bool {
        self.0.contains_key(item.borrow())
    }

    pub fn merge(&self, other: &Self) -> Self {
        Self(
            self.0
                .keys()
                .chain(other.0.keys())
                .map(|k| (k.clone(), other.0.get(k).map(|k| *k).unwrap_or(false)))
                .collect(),
        )
    }
}

pub fn diff_plist<P: ParamList + From<ParameterList>>(base: &P, other: &P) -> P {
    ParameterList {
        lists: ParameterListMap(
            other
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
        ),
        objects: ParameterObjectMap(
            other
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
        ),
    }
    .into()
}

pub fn diff_pobj(base: &ParameterObject, other: &ParameterObject) -> ParameterObject {
    ParameterObject(
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
            .collect(),
    )
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
    ParameterObject(
        base.0
            .iter()
            .chain(diff.0.iter())
            .map(|(k, v)| (*k, v.clone()))
            .collect(),
    )
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
