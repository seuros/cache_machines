extern crate std;

use super::*;

#[test]
fn hits_change_eviction_order_and_frequency_ties_keep_the_incumbent() {
    let mut cache = Lru::<u64, 2>::new();
    assert_eq!(cache.admit(1, 0, 0), Admission::Inserted);
    assert_eq!(cache.admit(2, 0, 0), Admission::Inserted);
    assert_eq!(cache.victim(), Some(1));
    assert!(cache.access(1));
    assert_eq!(cache.victim(), Some(2));
    assert_eq!(cache.admit(3, 4, 4), Admission::Rejected);
    assert_eq!(cache.victim(), Some(2));
    assert_eq!(cache.admit(3, 5, 4), Admission::Evicted(2));
    assert!(!cache.access(2));
    assert_eq!(cache.admit(1, 0, 0), Admission::Hit);
    assert_eq!(cache.victim(), Some(3));
}

#[test]
fn zero_capacity_and_misses_are_well_defined() {
    let mut cache = Lru::<u8, 0>::new();
    assert_eq!(cache.admit(1, u64::MAX, 0), Admission::Rejected);
    assert!(!cache.access(1));
    assert_eq!(cache.victim(), None);
    assert_eq!(cache.capacity(), 0);
    assert!(cache.is_empty());
    assert!(!cache.remove(1));
}

#[test]
fn removal_and_clear_preserve_policy_invariants() {
    let mut cache = Lru::<u64, 3>::new();
    for key in 1..=3 {
        assert_eq!(cache.admit(key, 0, 0), Admission::Inserted);
    }
    assert_eq!(cache.keys().collect::<std::vec::Vec<_>>(), [3, 2, 1]);
    assert!(cache.remove(2));
    assert_eq!(cache.keys().collect::<std::vec::Vec<_>>(), [3, 1]);
    assert_eq!(cache.len(), 2);
    assert_eq!(cache.victim(), None);
    assert!(!cache.remove(2));
    assert_eq!(cache.admit(u64::MAX, 0, u64::MAX), Admission::Inserted);
    assert_eq!(cache.victim(), Some(1));
    cache.clear();
    assert!(cache.is_empty());
    assert_eq!(cache.keys().next(), None);
    assert_eq!(cache.admit(0, 0, 0), Admission::Inserted);
}
