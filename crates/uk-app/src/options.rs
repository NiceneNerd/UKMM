//! Pure option-gating logic, shared by all frontends.

use uk_mod::{ModOption, ModOptionGroup, OptionGroup};

/// True when every required option group has at least one of its options in
/// `enabled`. Non-required groups never block installation.
pub fn required_satisfied(groups: &[OptionGroup], enabled: &[ModOption]) -> bool {
    groups
        .iter()
        .all(|group| !group.required() || group.options().iter().any(|opt| enabled.contains(opt)))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use uk_mod::{ExclusiveOptionGroup, ModOption, MultipleOptionGroup, OptionGroup};

    use super::*;

    fn opt(name: &str) -> ModOption {
        ModOption {
            name: name.into(),
            description: Default::default(),
            path: PathBuf::from(name),
            requires: vec![],
        }
    }

    fn exclusive(required: bool, options: Vec<ModOption>) -> OptionGroup {
        OptionGroup::Exclusive(ExclusiveOptionGroup {
            name: "g".into(),
            description: Default::default(),
            required,
            default: None,
            options,
        })
    }

    fn multiple(required: bool, options: Vec<ModOption>) -> OptionGroup {
        OptionGroup::Multiple(MultipleOptionGroup {
            name: "g".into(),
            description: Default::default(),
            required,
            defaults: Default::default(),
            options,
        })
    }

    #[test]
    fn no_groups_is_satisfied() {
        assert!(required_satisfied(&[], &[]));
    }

    #[test]
    fn optional_group_never_blocks() {
        let groups = vec![
            exclusive(false, vec![opt("a")]),
            multiple(false, vec![opt("b")]),
        ];
        assert!(required_satisfied(&groups, &[]));
    }

    #[test]
    fn required_exclusive_group_needs_a_pick() {
        let groups = vec![exclusive(true, vec![opt("a"), opt("b")])];
        assert!(!required_satisfied(&groups, &[]));
        assert!(required_satisfied(&groups, &[opt("b")]));
        // a pick from another group does not count
        assert!(!required_satisfied(&groups, &[opt("c")]));
    }

    #[test]
    fn required_multiple_group_needs_a_pick() {
        let groups = vec![multiple(true, vec![opt("a")])];
        assert!(!required_satisfied(&groups, &[]));
        assert!(required_satisfied(&groups, &[opt("a")]));
    }

    #[test]
    fn all_required_groups_must_be_satisfied() {
        let groups = vec![
            exclusive(true, vec![opt("a")]),
            multiple(true, vec![opt("b")]),
        ];
        assert!(!required_satisfied(&groups, &[opt("a")]));
        assert!(required_satisfied(&groups, &[opt("a"), opt("b")]));
    }
}
