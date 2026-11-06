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
updates. A zero-capacity cache rejects every insertion.
