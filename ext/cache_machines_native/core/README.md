# cache-machines

> **AMS Denning**, NCC-4203, *Denning*-class cache tender.

Engine-room flight manual: choose which supplies deserve a berth in limited storage.

Allocation-free `no_std` fixed-capacity LRU key policy. Hits move a key to the
front; accepted new keys evict the least-recently-used key. When full, admission
requires candidate frequency to exceed the victim's frequency (ties retain the
incumbent). Frequencies are supplied by the caller, potentially using an existing
probabilistic sketch; this is not a complete TinyLFU/W-TinyLFU implementation.

Values, loading, eviction callbacks, TTLs, byte weights, synchronization, and sketch
aging are not owned by this policy. Coordinate key-policy mutations with storage
updates. A zero-capacity cache refuses every insertion.

## Contract

- **Refusals.** `admit` returns `Result<Admission<K>, Refusal>`; `Refusal` is
  the crate's one refusal type: `Outranked` (full, and
  `candidate_frequency <= victim_frequency`: ties keep the incumbent) and
  `ZeroCapacity` (`N == 0`). A refusal changes nothing: no key moves, none is
  inserted or evicted.
- **Honoured answers.** `Admission::{Hit, Inserted, Evicted(K)}`;
  `access -> Access::{Hit, Miss}`; `remove -> Removal::{Removed, Absent}`.
  `access` and `remove` never refuse. A miss is an answer ("not retained"), not
  a refusal: reading is never admitting, and a miss changes nothing.
- **Repeats.** `remove` of a key that is not retained is `Removal::Absent`: the
  effect already holds, provably, and nothing changes. `admit` or `access` of
  the most recent key is `Hit` and moves nothing.
- **Keys.** `K: Copy + Eq`, no `Default`: free slots hold no key, so caches
  retaining the same keys in the same order compare equal, and `Lru::new` is a
  `const fn` (an empty cache can be a `const` or a `static`).
- **Exhaustive.** `Admission`, `Access`, `Removal` and `Refusal` carry no
  `#[non_exhaustive]`; a new variant is a breaking change.
- **Scopes and restarts.** The policy issues no ids; keys are the caller's. A
  new cache is empty.
- **Time.** None. Frequencies are caller-measured counts (`u64`), not clocks;
  the policy reads and records no time.
