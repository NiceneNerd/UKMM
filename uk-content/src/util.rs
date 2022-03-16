use roead::aamp::*;

pub fn diff_plist(this: &ParameterList, other: &ParameterList) -> ParameterList {
    ParameterList {
        lists: ParameterListMap(
            other
                .lists
                .0
                .iter()
                .filter_map(|(k, v)| {
                    if !this.lists.0.contains_key(k) {
                        Some((*k, v.clone()))
                    } else if this.lists.0[k] != *v {
                        Some((*k, diff_plist(&this.lists.0[k], v)))
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
                    if !this.objects.0.contains_key(k) {
                        Some((*k, v.clone()))
                    } else if this.objects.0[k] != *v {
                        Some((*k, diff_pobj(&this.objects.0[k], v)))
                    } else {
                        None
                    }
                })
                .collect(),
        ),
    }
}

pub fn diff_pobj(this: &ParameterObject, other: &ParameterObject) -> ParameterObject {
    ParameterObject(
        other
            .0
            .iter()
            .filter_map(|(k, v)| {
                if !this.0.contains_key(k) || this.0[k] != *v {
                    Some((*k, v.clone()))
                } else {
                    None
                }
            })
            .collect(),
    )
}
