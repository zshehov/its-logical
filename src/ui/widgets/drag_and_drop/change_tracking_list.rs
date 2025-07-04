use std::hash::Hash;

#[derive(PartialEq, Debug, Clone)]
pub(crate) enum Change<T> {
    Pushed(T),
    Moved(Vec<usize>),
    Removed(usize),
}

pub(crate) struct ChangeTrackingVec<T: Clone + Eq + Hash> {
    current_changes: Vec<Change<T>>,
    items: Vec<T>,
    order_changes: Vec<usize>,
    order_has_changed: bool,
}

impl<T: Clone + Eq + Hash> ChangeTrackingVec<T> {
    pub(crate) fn new(items: Vec<T>) -> Self {
        let mut order_changes = Vec::<usize>::with_capacity(items.len());

        for idx in 0..items.len() {
            order_changes.push(idx);
        }

        Self {
            current_changes: vec![],
            order_changes,
            items,
            order_has_changed: false,
        }
    }

    pub(crate) fn push(&mut self, item: T) {
        if let Some(order_changes) = self.flush_order_changes() {
            self.current_changes.push(Change::Moved(order_changes));
        }
        self.items.push(item.clone());
        self.order_changes.push(self.items.len() - 1);
        self.current_changes.push(Change::Pushed(item));
    }

    pub(crate) fn move_item(&mut self, from: usize, to: usize) {
        let item = self.items.remove(from);
        self.items.insert(to, item);
        let order_idx = self.order_changes.remove(from);
        self.order_changes.insert(to, order_idx);
        self.order_has_changed = true;
    }

    pub(crate) fn remove(&mut self, idx: usize) -> T {
        if let Some(order_changes) = self.flush_order_changes() {
            self.current_changes.push(Change::Moved(order_changes));
        }
        let removed = self.items.remove(idx);
        self.order_changes.remove(idx);
        // order changes before the removal have already been persisted
        self.reset_order();
        self.current_changes.push(Change::Removed(idx));
        removed
    }

    pub(crate) fn iter(&self) -> std::slice::Iter<'_, T> {
        self.items.iter()
    }

    pub(crate) fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
        self.items.iter_mut()
    }

    pub(crate) fn len(&self) -> usize {
        self.items.len()
    }

    pub(crate) fn get_current_changes(&mut self) -> Vec<Change<T>> {
        if let Some(order_changes) = self.flush_order_changes() {
            self.current_changes.push(Change::Moved(order_changes));
        }
        let mut empty_changes: Vec<Change<T>> = vec![];
        std::mem::swap(&mut self.current_changes, &mut empty_changes);
        empty_changes
    }

    fn flush_order_changes(&mut self) -> Option<Vec<usize>> {
        if self.order_has_changed {
            let mut order_changes = Vec::<usize>::with_capacity(self.items.len());

            for pre_change_idx in &self.order_changes {
                order_changes.push(*pre_change_idx);
            }

            self.reset_order();
            self.order_has_changed = false;
            return Some(order_changes);
        }
        None
    }

    fn reset_order(&mut self) {
        for (idx, order_idx) in &mut self.order_changes.iter_mut().enumerate() {
            *order_idx = idx;
        }
    }
}

#[test]
fn test_new() {
    let v = ChangeTrackingVec::new(vec![1, 2, 3]);
    assert_eq!(v.items, vec![1, 2, 3]);
    assert_eq!(v.order_changes.len(), 3);
    assert_eq!(v.current_changes.len(), 0);
}

#[test]
fn test_push() {
    let mut v = ChangeTrackingVec::new(vec![1, 2, 3]);
    v.push(4);
    assert_eq!(v.items, vec![1, 2, 3, 4]);
    assert_eq!(v.order_changes.len(), 4);
    assert_eq!(v.current_changes.len(), 1);
    assert_eq!(v.current_changes[0], Change::Pushed(4));
}

#[test]
fn test_move_item() {
    let mut v = ChangeTrackingVec::new(vec![1, 2, 3]);
    v.move_item(0, 2);
    assert_eq!(v.items, vec![2, 3, 1]);
}

#[test]
fn test_remove() {
    let mut v = ChangeTrackingVec::new(vec![1, 2, 3]);
    v.remove(1);
    assert_eq!(v.items, vec![1, 3]);
    assert_eq!(v.order_changes.len(), 2);
    assert_eq!(v.current_changes.len(), 1);
    assert_eq!(v.current_changes[0], Change::Removed(1));
}

#[test]
fn test_flush_order_changes() {
    let mut v = ChangeTrackingVec::new(vec![1, 2, 3]);
    v.move_item(0, 2);
    assert_eq!(v.items, vec![2, 3, 1]);
    assert!(v.order_has_changed);
    let order_changes = v.flush_order_changes().unwrap();
    assert_eq!(order_changes, vec![1, 2, 0]);
    assert!(!v.order_has_changed);
}

#[test]
fn test_push_after_move() {
    let mut v = ChangeTrackingVec::new(vec![1, 2, 3]);
    v.move_item(0, 2);
    assert_eq!(v.items, vec![2, 3, 1]);
    v.push(69);
    assert_eq!(v.items, vec![2, 3, 1, 69]);
    assert_eq!(v.order_changes.len(), 4);
    let current_changes = v.get_current_changes();
    assert_eq!(current_changes.len(), 2);
    assert_eq!(current_changes[0], Change::Moved(vec![1, 2, 0]));
    assert_eq!(current_changes[1], Change::Pushed(69));
}

#[test]
fn test_remove_after_move() {
    let mut v = ChangeTrackingVec::new(vec![1, 2, 3]);
    v.move_item(0, 2);
    assert_eq!(v.items, vec![2, 3, 1]);
    v.remove(1);
    assert_eq!(v.items, vec![2, 1]);
    assert_eq!(v.order_changes.len(), 2);
    let current_changes = v.get_current_changes();
    assert_eq!(current_changes.len(), 2);
    assert_eq!(current_changes[0], Change::Moved(vec![1, 2, 0]));
    assert_eq!(current_changes[1], Change::Removed(1));
}

#[test]
fn test_move_after_move_when_from_is_after_to() {
    let mut v = ChangeTrackingVec::new(vec![1, 2]);
    v.move_item(1, 0);
    assert_eq!(v.items, vec![2, 1]);
    v.move_item(1, 0);
    assert_eq!(v.items, vec![1, 2]);
    assert_eq!(v.order_changes.len(), 2);
    let current_changes = v.get_current_changes();
    assert_eq!(current_changes, vec![Change::Moved(vec![0, 1])]);
}

#[test]
fn test_move_after_move_when_to_is_after_from() {
    let mut v = ChangeTrackingVec::new(vec![1, 2]);
    v.move_item(0, 1);
    assert_eq!(v.items, vec![2, 1]);
    v.move_item(0, 1);
    assert_eq!(v.items, vec![1, 2]);
    assert_eq!(v.order_changes.len(), 2);
    let current_changes = v.get_current_changes();
    assert_eq!(current_changes, vec![Change::Moved(vec![0, 1])]);
}

#[test]
fn test_iter() {
    let v = ChangeTrackingVec::new(vec![1, 2, 3]);
    let new_vec: Vec<&i32> = v.iter().collect();
    assert_eq!(new_vec, vec![&1, &2, &3]);
}

#[test]
fn test_iter_mut() {
    let mut v = ChangeTrackingVec::new(vec![1, 2, 3]);
    let mut new_vec: Vec<&mut i32> = v.iter_mut().collect();
    *new_vec[1] = 69;
    assert_eq!(v.items, vec![1, 69, 3]);
}
