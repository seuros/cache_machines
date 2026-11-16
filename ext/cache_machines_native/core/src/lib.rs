#![no_std]
#![doc = include_str!("../README.md")]
#![cfg_attr(not(test), deny(clippy::indexing_slicing))]

use core::fmt;

/// Honoured answer to [`Lru::admit`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Admission<K> {
    /// The key was already retained and is now the most recent.
    Hit,
    /// The key took a free slot and is now the most recent.
    Inserted,
    /// The key replaced the least-recently-used key, carried here so the
    /// caller drops its payload.
    Evicted(K),
}

/// Answer to [`Lru::access`]. Reading is never admitting: a miss inserts
/// nothing and changes nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    /// The key is retained and is now the most recent.
    Hit,
    /// The key is not retained.
    Miss,
}

/// Honoured answer to [`Lru::remove`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Removal {
    /// The key was retained and is gone; the others keep their order.
    Removed,
    /// The key was not retained: the effect already holds, nothing changed.
    Absent,
}

/// Why [`Lru::admit`] did not admit a key. A refusal changes nothing: no key
/// moves, none is inserted or evicted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// The cache is full and the candidate's frequency does not exceed the
    /// victim's: ties keep the incumbent.
    Outranked,
    /// The cache has no slot at all (`N == 0`).
    ZeroCapacity,
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Outranked => "the candidate does not outrank the victim",
            Self::ZeroCapacity => "the cache has no capacity",
        })
    }
}

impl core::error::Error for Refusal {}

/// LRU keys in most-recent-first order. Slots before `len` hold `Some`,
/// slots at and past it `None`, so two caches retaining the same keys in the
/// same order compare equal and no placeholder key is needed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lru<K, const N: usize> {
    keys: [Option<K>; N],
    len: usize,
}

impl<K: Copy + Eq, const N: usize> Default for Lru<K, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Copy + Eq, const N: usize> Lru<K, N> {
    /// Start empty.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            keys: [None; N],
            len: 0,
        }
    }

    /// Maximum number of keys this policy can retain.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        N
    }

    /// Number of retained keys.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Whether there are no retained keys.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Retained keys in most-recent-first order.
    pub fn keys(&self) -> impl Iterator<Item = K> + '_ {
        self.live().iter().flatten().copied()
    }

    /// Forget every key.
    pub fn clear(&mut self) {
        self.keys.fill(None);
        self.len = 0;
    }

    /// The retained slots, `keys[..len]`. `len <= N` always holds (it only
    /// grows while `len < N`); were it ever broken the live run reads as
    /// empty, so every operation answers as for a key that is not retained.
    fn live(&self) -> &[Option<K>] {
        self.keys.get(..self.len).unwrap_or_default()
    }

    fn live_mut(&mut self) -> &mut [Option<K>] {
        self.keys.get_mut(..self.len).unwrap_or_default()
    }

    /// The retained run up to and including `key`, so its last slot holds
    /// `key`; `None` when `key` is not retained.
    fn through(&mut self, key: K) -> Option<&mut [Option<K>]> {
        let live = self.live_mut();
        let index = live.iter().position(|value| *value == Some(key))?;
        live.get_mut(..=index)
    }

    /// Remove a key without changing the relative order of the others.
    /// Removing a key that is not retained is an honoured repeat,
    /// [`Removal::Absent`], and changes nothing.
    pub fn remove(&mut self, key: K) -> Removal {
        let live = self.live_mut();
        let Some(index) = live.iter().position(|value| *value == Some(key)) else {
            return Removal::Absent;
        };
        if let Some(tail) = live.get_mut(index..) {
            tail.rotate_left(1);
            if let Some(last) = tail.last_mut() {
                *last = None;
            }
        }
        // `index < len`, so `len >= 1`.
        self.len = self.len.saturating_sub(1);
        Removal::Removed
    }

    /// Promote a hit to most recent; a miss answers [`Access::Miss`] and
    /// leaves the ordering unchanged.
    pub fn access(&mut self, key: K) -> Access {
        let Some(head) = self.through(key) else {
            return Access::Miss;
        };
        head.rotate_right(1);
        Access::Hit
    }

    /// Admit a key; `victim_frequency` refers to the current [`Lru::victim`]
    /// key and is only consulted when the cache is full.
    ///
    /// # Errors
    ///
    /// [`Refusal::Outranked`] when the cache is full and
    /// `candidate_frequency <= victim_frequency`; [`Refusal::ZeroCapacity`]
    /// when `N == 0`. Nothing changes.
    pub fn admit(
        &mut self,
        key: K,
        candidate_frequency: u64,
        victim_frequency: u64,
    ) -> Result<Admission<K>, Refusal> {
        if self.access(key) == Access::Hit {
            return Ok(Admission::Hit);
        }
        let (outcome, len) = match self.victim() {
            Some(_) if candidate_frequency <= victim_frequency => {
                return Err(Refusal::Outranked);
            }
            Some(victim) => (Admission::Evicted(victim), self.len),
            // `len < N <= usize::MAX`, so this cannot overflow.
            None if self.len < N => (Admission::Inserted, self.len.saturating_add(1)),
            None => return Err(Refusal::ZeroCapacity),
        };
        // `1 <= len <= N`: a victim exists only when the cache is full and
        // nonempty, an insertion only below capacity.
        let slots = self.keys.get_mut(..len).unwrap_or_default();
        // Shift every retained key down one place, the last falling off (it
        // is the evicted victim, or the empty slot just claimed). On an empty
        // run the rotation is a no-op, so the arm below changes nothing.
        slots.rotate_right(1);
        let Some(first) = slots.first_mut() else {
            debug_assert!(false, "an admission always has a slot below N");
            return Err(Refusal::ZeroCapacity);
        };
        *first = Some(key);
        self.len = len;
        Ok(outcome)
    }

    /// Current eviction candidate, when the cache is full and nonempty.
    #[must_use]
    pub const fn victim(&self) -> Option<K> {
        match self.keys.last() {
            Some(slot) if self.len == N => *slot,
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests;
