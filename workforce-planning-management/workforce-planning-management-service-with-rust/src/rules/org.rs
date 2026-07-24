//! Org-chart rules (WPM-R7), DB-free: the manager chain must stay a
//! forest — assigning a manager may not create a cycle.

use std::collections::HashMap;
use std::hash::BuildHasher;
use uuid::Uuid;

/// Whether setting `employee`'s manager to `manager` would create a
/// cycle, given the current `manager_of` map (employee pid → manager
/// pid). Walks up from the proposed manager; hitting `employee` means
/// a cycle. Self-management is a cycle of length one. The walk is
/// bounded by the map size, so a (corrupt) pre-existing cycle
/// elsewhere terminates rather than spinning.
#[must_use]
pub fn would_create_cycle<S: BuildHasher>(
    employee: Uuid,
    manager: Uuid,
    manager_of: &HashMap<Uuid, Uuid, S>,
) -> bool {
    if employee == manager {
        return true;
    }
    let mut current = manager;
    for _ in 0..=manager_of.len() {
        match manager_of.get(&current) {
            Some(&next) if next == employee => return true,
            Some(&next) => current = next,
            None => return false,
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }

    /// Self, direct, and transitive cycles are caught; legal chains and
    /// re-parenting pass.
    #[test]
    fn cycle_detection() {
        // chain: 3 -> 2 -> 1 (1 is the root)
        let mut map = HashMap::new();
        map.insert(u(3), u(2));
        map.insert(u(2), u(1));
        assert!(would_create_cycle(u(5), u(5), &map)); // self
        assert!(would_create_cycle(u(1), u(3), &map)); // root under leaf
        assert!(would_create_cycle(u(1), u(2), &map)); // root under middle
        assert!(!would_create_cycle(u(4), u(3), &map)); // new leaf
        assert!(!would_create_cycle(u(3), u(1), &map)); // re-parent up
    }

    /// A corrupt pre-existing cycle elsewhere terminates (bounded walk)
    /// and does not implicate an unrelated assignment.
    #[test]
    fn bounded_walk_survives_corrupt_data() {
        let mut map = HashMap::new();
        map.insert(u(1), u(2));
        map.insert(u(2), u(1)); // pre-existing corruption
        assert!(!would_create_cycle(u(9), u(1), &map));
    }
}
