extern crate std;

use super::*;
use core::fmt::Debug;
use std::vec::Vec;

mod model;

/// The call answers `answer` (a refusal included) and leaves `cache` exactly as it was.
fn unchanged<const N: usize, O: Copy + Debug + PartialEq>(
    cache: &mut Lru<u64, N>,
    answer: O,
    call: impl FnOnce(&mut Lru<u64, N>) -> O,
) {
    let before = cache.clone();
    assert_eq!(call(cache), answer);
    assert_eq!(*cache, before, "{answer:?} changed the cache");
}

fn keys<const N: usize>(cache: &Lru<u64, N>) -> Vec<u64> {
    cache.keys().collect()
}

#[test]
fn hits_change_eviction_order_and_frequency_ties_keep_the_incumbent() {
    let mut cache = Lru::<u64, 2>::new();
    assert_eq!(cache.admit(1, 0, 0), Ok(Admission::Inserted));
    assert_eq!(cache.admit(2, 0, 0), Ok(Admission::Inserted));
    assert_eq!(cache.victim(), Some(1));
    assert_eq!(cache.access(1), Access::Hit);
    assert_eq!(cache.victim(), Some(2));
    unchanged(&mut cache, Err(Refusal::Outranked), |c| c.admit(3, 4, 4));
    unchanged(&mut cache, Err(Refusal::Outranked), |c| c.admit(3, 3, 4));
    assert_eq!(cache.victim(), Some(2));
    assert_eq!(cache.admit(3, 5, 4), Ok(Admission::Evicted(2)));
    unchanged(&mut cache, Access::Miss, |c| c.access(2));
    assert_eq!(cache.admit(1, 0, 0), Ok(Admission::Hit));
    assert_eq!(cache.victim(), Some(3));
}

#[test]
fn a_hit_on_the_front_key_is_a_repeat_that_changes_nothing() {
    let mut cache = Lru::<u64, 2>::new();
    assert_eq!(cache.admit(1, 0, 0), Ok(Admission::Inserted));
    unchanged(&mut cache, Ok(Admission::Hit), |c| c.admit(1, 0, 0));
    unchanged(&mut cache, Access::Hit, |c| c.access(1));
}

#[test]
fn zero_capacity_and_misses_are_well_defined() {
    let mut cache = Lru::<u64, 0>::new();
    unchanged(&mut cache, Err(Refusal::ZeroCapacity), |c| {
        c.admit(1, u64::MAX, 0)
    });
    unchanged(&mut cache, Access::Miss, |c| c.access(1));
    unchanged(&mut cache, Removal::Absent, |c| c.remove(1));
    assert_eq!(cache.victim(), None);
    assert_eq!(cache.capacity(), 0);
    assert!(cache.is_empty());
}

#[test]
fn removal_and_clear_preserve_policy_invariants() {
    let mut cache = Lru::<u64, 3>::new();
    for key in 1..=3 {
        assert_eq!(cache.admit(key, 0, 0), Ok(Admission::Inserted));
    }
    assert_eq!(keys(&cache), [3, 2, 1]);
    assert_eq!(cache.remove(2), Removal::Removed);
    assert_eq!(keys(&cache), [3, 1]);
    assert_eq!(cache.len(), 2);
    assert_eq!(cache.victim(), None);
    unchanged(&mut cache, Removal::Absent, |c| c.remove(2));
    assert_eq!(cache.admit(u64::MAX, 0, u64::MAX), Ok(Admission::Inserted));
    assert_eq!(cache.victim(), Some(1));
    cache.clear();
    assert!(cache.is_empty());
    assert_eq!(cache.keys().next(), None);
    assert_eq!(cache, Lru::new(), "a cleared cache equals a new one");
    assert_eq!(cache.admit(0, 0, 0), Ok(Admission::Inserted));
}

#[test]
fn equal_contents_compare_equal_whatever_the_history() {
    let mut removed = Lru::<u64, 3>::new();
    for key in [7, 8, 9] {
        assert_eq!(removed.admit(key, 0, 0), Ok(Admission::Inserted));
    }
    assert_eq!(removed.remove(9), Removal::Removed);
    let mut direct = Lru::<u64, 3>::new();
    for key in [7, 8] {
        assert_eq!(direct.admit(key, 0, 0), Ok(Admission::Inserted));
    }
    assert_eq!(removed, direct);
    assert_eq!(Lru::<u64, 3>::default(), Lru::new());
}

#[test]
fn the_default_key_is_an_ordinary_key() {
    let mut cache = Lru::<u64, 2>::new();
    unchanged(&mut cache, Access::Miss, |c| c.access(0));
    unchanged(&mut cache, Removal::Absent, |c| c.remove(0));
    assert_eq!(cache.admit(0, 0, 0), Ok(Admission::Inserted));
    assert_eq!(cache.admit(1, 0, 0), Ok(Admission::Inserted));
    assert_eq!(cache.victim(), Some(0));
    assert_eq!(cache.admit(2, 1, 0), Ok(Admission::Evicted(0)));
    unchanged(&mut cache, Access::Miss, |c| c.access(0));
}

#[test]
fn refusals_describe_themselves() {
    use std::string::ToString;
    let all = [Refusal::Outranked, Refusal::ZeroCapacity];
    for (index, refusal) in all.iter().enumerate() {
        let text = refusal.to_string();
        assert_ne!(text, "");
        assert!(all[..index].iter().all(|other| other.to_string() != text));
        let _: &dyn core::error::Error = refusal;
    }
}
