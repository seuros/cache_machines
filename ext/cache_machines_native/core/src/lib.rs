#![no_std]
#![doc = include_str!("../README.md")]

/// Outcome for a key admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Admission<K> {
    /// Existing key was promoted.
    Hit,
    /// New key was inserted into an empty slot.
    Inserted,
    /// The least-recently-used key was replaced.
    Evicted(K),
    /// Frequency policy or zero capacity rejected the candidate.
    Rejected,
}

/// LRU keys in most-recent-first order.
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
        self.keys[..self.len].iter().flatten().copied()
    }

    /// Forget every key.
    pub fn clear(&mut self) {
        self.keys.fill(None);
        self.len = 0;
    }

    fn index(&self, key: K) -> Option<usize> {
        self.keys[..self.len]
            .iter()
            .position(|value| *value == Some(key))
    }

    /// Remove a key without changing the relative order of the others.
    #[must_use]
    pub fn remove(&mut self, key: K) -> bool {
        let Some(index) = self.index(key) else {
            return false;
        };
        self.keys[index..self.len].rotate_left(1);
        self.len -= 1;
        self.keys[self.len] = None;
        true
    }

    /// Promote a hit; misses leave the ordering unchanged.
    #[must_use]
    pub fn access(&mut self, key: K) -> bool {
        let Some(index) = self.index(key) else {
            return false;
        };
        self.keys[..=index].rotate_right(1);
        true
    }

    /// Admit a key; victim frequency refers to the current `victim()` key.
    #[must_use]
    pub fn admit(
        &mut self,
        key: K,
        candidate_frequency: u64,
        victim_frequency: u64,
    ) -> Admission<K> {
        if self.access(key) {
            return Admission::Hit;
        }
        if N == 0 {
            return Admission::Rejected;
        }
        let outcome = if self.len == N {
            if candidate_frequency <= victim_frequency {
                return Admission::Rejected;
            }
            let Some(victim) = self.keys[N - 1] else {
                return Admission::Rejected;
            };
            Admission::Evicted(victim)
        } else {
            self.len += 1;
            Admission::Inserted
        };
        self.keys[..self.len].rotate_right(1);
        self.keys[0] = Some(key);
        outcome
    }

    /// Current eviction candidate, when the cache is full and nonempty.
    #[must_use]
    pub const fn victim(&self) -> Option<K> {
        if N > 0 && self.len == N {
            self.keys[N - 1]
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests;
