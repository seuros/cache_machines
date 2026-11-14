#![no_std]
#![doc = include_str!("../README.md")]

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

/// LRU keys in most-recent-first order. Slots at and past `len` hold
/// `K::default()`, so two caches retaining the same keys in the same order
/// compare equal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lru<K, const N: usize> {
    keys: [K; N],
    len: usize,
}

impl<K: Copy + Eq + Default, const N: usize> Default for Lru<K, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Copy + Eq + Default, const N: usize> Lru<K, N> {
    /// Start empty.
    #[must_use]
    pub fn new() -> Self {
        Self {
            keys: [K::default(); N],
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
        self.keys[..self.len].iter().copied()
    }

    /// Forget every key.
    pub fn clear(&mut self) {
        self.keys.fill(K::default());
        self.len = 0;
    }

    fn index(&self, key: K) -> Option<usize> {
        self.keys[..self.len].iter().position(|value| *value == key)
    }

    /// Remove a key without changing the relative order of the others.
    /// Removing a key that is not retained is an honoured repeat,
    /// [`Removal::Absent`], and changes nothing.
    pub fn remove(&mut self, key: K) -> Removal {
        let Some(index) = self.index(key) else {
            return Removal::Absent;
        };
        self.keys[index..self.len].rotate_left(1);
        self.len -= 1;
        self.keys[self.len] = K::default();
        Removal::Removed
    }

    /// Promote a hit to most recent; a miss answers [`Access::Miss`] and
    /// leaves the ordering unchanged.
    pub fn access(&mut self, key: K) -> Access {
        let Some(index) = self.index(key) else {
            return Access::Miss;
        };
        self.keys[..=index].rotate_right(1);
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
        let outcome = match self.victim() {
            Some(_) if candidate_frequency <= victim_frequency => {
                return Err(Refusal::Outranked);
            }
            Some(victim) => Admission::Evicted(victim),
            None if self.len < N => {
                self.len += 1;
                Admission::Inserted
            }
            None => return Err(Refusal::ZeroCapacity),
        };
        self.keys[..self.len].rotate_right(1);
        self.keys[0] = key;
        Ok(outcome)
    }

    /// Current eviction candidate, when the cache is full and nonempty.
    #[must_use]
    pub const fn victim(&self) -> Option<K> {
        if self.len == N {
            self.keys.last().copied()
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests;
