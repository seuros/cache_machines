# CacheMachines

> Sixty-four berths, and a hatch that asks what you've done to deserve one. 📦

An admission-controlled LRU for Ruby. It keeps the most recently used keys in order, names the one it would throw out next, and refuses to throw it out for a newcomer that isn't worth more. You store the payloads and count the hits. It decides what stays.

## Quick Start

```bash
gem 'cache_machines'
```

```ruby
bay = CacheMachines::Lru.new

# key, candidate frequency, victim frequency: positional, like the Rust core.
bay.admit(42, 1, 0)   # => :inserted
bay.admit(7, 1, 0)    # => :inserted
bay.access(42)        # => :hit
bay.keys              # => [42, 7]
bay.remove(7)         # => :removed
```

Keys are Integers. Pair them with your own storage, hand the decisions to the bay, and the bay tells you what to keep.

## A Message to the Resistance

So you built an LRU cache. Hot rows stay, cold rows fall off the back, dashboards go green. Then the nightly export job runs. It reads every row in the table exactly once, and every one of those reads is a cache miss that inserts a key you will never ask for again.

By the time the export finishes, your working set is gone. The 64 keys that got hit ten thousand times each have been shoved out by 64 keys that got hit once. Monday morning is a cold start, and the LRU did exactly what you told it to do.

LRU answers "what was used least recently?" It never asks "does the newcomer deserve the slot?" This gem asks. When the bay is full, a candidate has to show up with a higher frequency than the key it would evict. Ties lose. The export job's keys never make it past the hatch.

## Chapter 1: Stardate 2025.186, the AMS Denning

> **AMS Denning**, NCC-4203, *Denning*-class cache tender.
> The hold is finite. The universe of possible consignments is not.

The tender has 64 berths and no more. Every consignment that arrives is logged by key, and the manifest is kept in order of last use. When the bay is full and something new arrives, the quartermaster names the berth at the back, asks how often each has been requisitioned, and only swaps if the newcomer wins outright.

Somewhere in the fleet, a supply officer proposes:

```ruby
def fetch(key)
  @cache[key] ||= load(key)     # Hash with 64 entries, oldest shifted off
end
```

"That," says the quartermaster, "is how one inventory scan empties the bay."

## The Flight Plan

What happens to a key offered to `admit`:

```mermaid
stateDiagram-v2
    [*] --> offered
    offered --> hit: key already aboard, promoted to front
    offered --> inserted: bay has a free berth
    offered --> outranked: bay full, candidate frequency <= victim frequency (refused)
    offered --> evicted: bay full, candidate frequency > victim frequency
    evicted --> [*]: Evicted carries the victim key
    hit --> [*]
    inserted --> [*]
    outranked --> [*]: nothing changed
```

And the night of the export job:

```mermaid
sequenceDiagram
    participant App
    participant Bay as Lru (full)
    participant Store as Your storage

    App->>Bay: victim
    Bay-->>App: 17 (seen 4000 times)
    App->>Bay: admit(export row 9001, 1, 4000)
    Bay-->>App: :outranked
    Note over App: don't cache the row, serve it and move on

    App->>Bay: access(17)
    Bay-->>App: :hit, promoted

    App->>Bay: victim
    Bay-->>App: 23 (seen 2 times)
    App->>Bay: admit(user 555, 9, 2)
    Bay-->>App: Evicted(key 23)
    App->>Store: delete 23, store 555
```

## How the Bay Works

### Admit

`admit(key, candidate_frequency, victim_frequency)` takes the key, how often the candidate has been requested, and how often the current victim has. It answers `:inserted`, `:hit`, or, only when something was displaced, an `Evicted` carrying the evicted key. A key that didn't get in is a refusal symbol.

```ruby
bay = CacheMachines::Lru.new
bay.admit(1, 5, 0)   # => :inserted
bay.admit(1, 5, 0)   # => :hit
```

While there's a free berth, frequencies don't matter. The candidate gets in. A key already aboard is a `:hit` and moves to the front.

### When the bay is full

Fill all 64 berths and the frequencies start to count.

```ruby
bay = CacheMachines::Lru.new
64.times { |key| bay.admit(key, 0, 0) }

bay.victim              # => 0
bay.admit(100, 9, 9)    # => :outranked
bay.admit(100, 8, 9)    # => :outranked
bay.admit(100, 10, 9)   # => #<data CacheMachines::Evicted key=0>
```

Equal loses. Lower loses. A refused candidate changes nothing: the victim is still the victim. Only strictly higher gets a berth, and then you are told exactly which key just left so you can drop its payload.

### Access

A hit promotes the key to the front and answers `:hit`. A miss answers `:miss`, inserts nothing, and changes nothing. Reading is never admitting. Both are answers, not refusals, and both are truthy: compare with `==`, don't test with `if`.

```ruby
bay = CacheMachines::Lru.new
bay.admit(1, 0, 0)
bay.admit(2, 0, 0)

bay.keys        # => [2, 1]
bay.access(1)   # => :hit
bay.keys        # => [1, 2]
bay.access(9)   # => :miss
bay.size        # => 2
```

### Victim

`victim` is the key at the back of the line, the one `admit` would evict. It is `nil` until the bay is full. Ask for it right before you admit, look up its frequency, and pass that in.

```ruby
bay = CacheMachines::Lru.new
bay.victim   # => nil
64.times { |key| bay.admit(key, 0, 0) }
bay.victim   # => 0
bay.access(0)
bay.victim                                                   # => 1
```

### Remove and clear

```ruby
bay = CacheMachines::Lru.new
bay.admit(1, 0, 0)
bay.remove(1)   # => :removed
bay.remove(1)   # => :absent, already gone: nothing changes
bay.clear       # => nil
bay.empty?      # => true
```

`keys` is a snapshot, most recent first. Mutate the array all you like; the bay doesn't notice.

## The Rules of the Tender

- **Ruby mirrors the Rust core.** `admit(key, candidate_frequency, victim_frequency)` is positional, in the order of `Lru::admit`. `access`, `remove`, `clear`, `keys`, `victim` and `capacity` are the core's methods under the core's names; `size` and `empty?` are `len` and `is_empty`.
- **Capacity is 64**, exposed as `CacheMachines::Lru::CAPACITY` and `#capacity`. The constructor takes no arguments.
- **Keys and frequencies are Integers** in `0..2**64-1`. Floats, strings, nil, and objects with `to_int` raise `TypeError`. Negative or too large raises `RangeError`. Bad input is rejected before anything changes. `ArgumentError` is only ever Ruby's own, for the wrong number of arguments.
- **Refusals don't raise and don't change anything.** `admit` is the only call that refuses: `:outranked` when the bay is full and the candidate's frequency doesn't exceed the victim's, `:zero_capacity` for a policy with no berths at all (the Rust core can be built that way; this gem's bay never is). Every reason has its own symbol, the snake case of the core's `Refusal` variant; there is no catch-all.
- **Repeats are honoured.** `remove` of a key that isn't aboard is `:absent`: the effect you asked for already holds, and nothing changes. `admit` or `access` of the key already at the front is `:hit` and moves nothing.
- **`Evicted` is a value, and values are valid or don't exist.** `CacheMachines::Evicted.new(key)` is checked by the native extension whether the bay built it or you did: a non-Integer raises `TypeError`, a key outside `0..2**64-1` `RangeError`. `==` compares it by value. It is sealed: subclassing it raises `TypeError` and `allocate` is gone. Sealing stops accidents, not reflection (a named `class X < Evicted` still binds `X` before raising, and `Data#initialize` can be bound directly), but the bay reads no `Evicted` from you, so a forged one reaches nothing and never equals a real answer.
- **An eviction is never lost.** `admit` decides on a copy of the bay and keeps it only once the answer exists: if building the `Evicted` raises, nothing was admitted or evicted. If building it changed the bay (a redefined `Evicted.new` calling back in), `admit` raises `RuntimeError` and is not applied; the inner change stands. If building it froze the bay, `admit` raises `FrozenError` and is not applied. The `Evicted` class is resolved once, when the extension loads: reassigning the constant later changes nothing the bay builds.
- **No scopes, no restarts to plan for.** The bay issues no ids or handles; keys are yours. A new bay is empty.
- **No clock.** Frequencies are counts you measured, not times. The bay reads no clock and records none.
- **Every answer, by method.**

  | Method | Honoured | Refused | Records the clock |
  | --- | --- | --- | --- |
  | `admit(key, candidate_frequency, victim_frequency)` | `:hit` (promoted), `:inserted`, an `Evicted` | `:outranked`, `:zero_capacity` | no clock, nothing recorded |
  | `access(key)` | `:hit` (promoted), `:miss` (nothing changes) | never refuses | no clock, nothing recorded |
  | `remove(key)` | `:removed`, `:absent` (a repeat: nothing changes) | never refuses | no clock, nothing recorded |
  | `clear` | `nil` | never refuses | no clock, nothing recorded |
  | `keys`, `size`, `empty?`, `victim`, `capacity` | a fresh Array, Integer, `true`/`false`, the victim key or `nil`, Integer | never refuses | no clock, nothing recorded |
  | `Evicted.new(key)` | the value | never refuses; an invalid key raises `RangeError` (`TypeError` for a non-Integer) | no clock, nothing recorded |

- **Frequencies are yours.** The bay never counts hits. It compares the two numbers you give it and nothing else.
- **Frozen means frozen.** After `freeze`, every mutator raises `FrozenError`, including a miss on `access` and a `remove` of a key that isn't there.

## What You Think You're Doing vs Reality

### You Think: "A bigger cache would have survived the export."
### Reality: Any finite cache loses to a scan larger than itself.

The export touches more rows than you have berths. Doubling the bay means it takes twice as long to flush, then flushes anyway. The fix is not more room. The fix is a hatch that asks whether the newcomer has earned a berth, and the answer for a row read once is no.

## What's Yours

The bay holds keys and order, nothing else. Payloads live in your store, and when `admit` answers an `Evicted` you delete that payload yourself. Frequency counting is yours too, whether that's a counter per key, a sketch, or a column you already have. Treat `victim` then `admit` as one step in your code; the bay won't stop another thread from slipping in between.

## License

MIT License. See [LICENSE.txt](LICENSE.txt) for details.

**Remember: The export job will read every row once. Make sure once is not enough.**
