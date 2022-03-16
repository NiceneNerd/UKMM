use roead::aamp::*;

pub fn diff_plist(base: &ParameterList, other: &ParameterList) -> ParameterList {
    ParameterList {
        lists: ParameterListMap(
            other
                .lists
                .0
                .iter()
                .filter_map(|(k, v)| {
                    if !base.lists.0.contains_key(k) {
                        Some((*k, v.clone()))
                    } else if base.lists.0[k] != *v {
                        Some((*k, diff_plist(&base.lists.0[k], v)))
                    } else {
                        None
                    }
                })
                .collect(),
        ),
        objects: ParameterObjectMap(
            other
                .objects
                .0
                .iter()
                .filter_map(|(k, v)| {
                    if !base.objects.0.contains_key(k) {
                        Some((*k, v.clone()))
                    } else if base.objects.0[k] != *v {
                        Some((*k, diff_pobj(&base.objects.0[k], v)))
                    } else {
                        None
                    }
                })
                .collect(),
        ),
    }
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

pub fn merge_plist(base: &ParameterList, diff: &ParameterList) -> ParameterList {
    ParameterList {
        objects: {
            let mut new = base.objects.clone();
            for (k, v) in &diff.objects.0 {
                if !new.0.contains_key(k) {
                    new.0.insert(*k, v.clone());
                } else {
                    new.0[k] = merge_pobj(&new.0[k], v);
                }
            }
            new
        },
        lists: {
            let mut new = base.lists.clone();
            for (k, v) in &diff.lists.0 {
                if !new.0.contains_key(k) {
                    new.0.insert(*k, v.clone());
                } else {
                    new.0[k] = merge_plist(&new.0[k], v);
                }
            }
            new
        }
    }
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
