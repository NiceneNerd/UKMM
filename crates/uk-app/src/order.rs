//! Pure load-order manipulation, shared by all frontends.

/// Moves the `selected` items so they start at index `dest` in `items`,
/// preserving their relative order. `dest` is clamped to the list bounds.
///
/// No-op when nothing is selected or everything is selected (moving the whole
/// list is meaningless), and safe on an empty list.
pub fn move_selected<T: Clone + PartialEq>(items: &mut Vec<T>, selected: &[T], dest: usize) {
    if selected.is_empty() || selected.len() == items.len() {
        return;
    }
    let dest = dest.min(items.len().saturating_sub(1));
    items.retain(|item| !selected.contains(item));
    for (i, item) in selected.iter().enumerate() {
        items.insert((dest + i).min(items.len()), item.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn moved(mut items: Vec<i32>, selected: &[i32], dest: usize) -> Vec<i32> {
        move_selected(&mut items, selected, dest);
        items
    }

    #[test]
    fn empty_selection_is_noop() {
        assert_eq!(moved(vec![1, 2, 3], &[], 1), vec![1, 2, 3]);
    }

    #[test]
    fn all_selected_is_noop() {
        assert_eq!(moved(vec![1, 2, 3], &[1, 2, 3], 0), vec![1, 2, 3]);
    }

    #[test]
    fn empty_list_is_safe() {
        assert_eq!(moved(vec![], &[], 5), Vec::<i32>::new());
    }

    #[test]
    fn dest_is_clamped() {
        assert_eq!(moved(vec![1, 2, 3], &[1], 99), vec![2, 3, 1]);
        assert_eq!(moved(vec![1, 2, 3], &[3], 99), vec![1, 2, 3]);
    }

    #[test]
    fn relative_order_of_selection_is_preserved() {
        assert_eq!(moved(vec![1, 2, 3, 4], &[3, 1], 1), vec![2, 3, 1, 4]);
    }

    #[test]
    fn move_to_front_and_back() {
        assert_eq!(moved(vec![1, 2, 3, 4], &[4], 0), vec![4, 1, 2, 3]);
        assert_eq!(moved(vec![1, 2, 3, 4], &[1], 3), vec![2, 3, 4, 1]);
    }
}
