//! Randomized comparison against a plain reference model.
use super::*;
use matryona_machines::testing::XorShift64;

const CAPACITY: usize = 4;

/// Keys most recent first, by definition.
struct Model(Vec<u64>);

impl Model {
    fn access(&mut self, key: u64) -> Access {
        let Some(index) = self.0.iter().position(|&k| k == key) else {
            return Access::Miss;
        };
        let key = self.0.remove(index);
        self.0.insert(0, key);
        Access::Hit
    }

    fn remove(&mut self, key: u64) -> Removal {
        let Some(index) = self.0.iter().position(|&k| k == key) else {
            return Removal::Absent;
        };
        self.0.remove(index);
        Removal::Removed
    }

    fn admit(&mut self, key: u64, candidate: u64, victim: u64) -> Result<Admission<u64>, Refusal> {
        if self.access(key) == Access::Hit {
            return Ok(Admission::Hit);
        }
        if self.0.len() < CAPACITY {
            self.0.insert(0, key);
            return Ok(Admission::Inserted);
        }
        if candidate <= victim {
            return Err(Refusal::Outranked);
        }
        let evicted = self.0.pop();
        self.0.insert(0, key);
        evicted.map(Admission::Evicted).ok_or(Refusal::ZeroCapacity)
    }
}

/// Honoured changes, honoured answers that changed nothing, refusals.
fn run(seed: u64) -> [usize; 3] {
    let mut random = XorShift64::new(seed);
    let mut cache = Lru::<u64, CAPACITY>::new();
    let mut model = Model(Vec::new());
    let mut tally = [0; 3];
    for _ in 0..400 {
        let before = cache.clone();
        let key = random.below(8);
        let changed = match random.below(4) {
            0 | 1 => {
                let (candidate, victim) = (random.below(4), random.below(4));
                let answer = cache.admit(key, candidate, victim);
                assert_eq!(answer, model.admit(key, candidate, victim));
                match answer {
                    Ok(Admission::Hit) => Some(before.keys().next() != Some(key)),
                    Ok(Admission::Inserted | Admission::Evicted(_)) => Some(true),
                    Err(_) => None,
                }
            }
            2 => {
                let answer = cache.access(key);
                assert_eq!(answer, model.access(key));
                Some(answer == Access::Hit && before.keys().next() != Some(key))
            }
            _ => {
                let answer = cache.remove(key);
                assert_eq!(answer, model.remove(key));
                Some(answer == Removal::Removed)
            }
        };
        match changed {
            Some(true) => tally[0] += 1,
            Some(false) => {
                tally[1] += 1;
                assert_eq!(
                    cache, before,
                    "an answer that moved nothing changed the cache"
                );
            }
            None => {
                tally[2] += 1;
                assert_eq!(cache, before, "a refusal changed the cache");
            }
        }
        assert_eq!(cache.keys().collect::<Vec<_>>(), model.0);
        let full = model.0.len() == CAPACITY;
        assert_eq!(cache.victim(), model.0.last().copied().filter(|_| full));
    }
    tally
}

#[test]
fn random_operations_match_the_reference_model() {
    let mut tally = [0; 3];
    for seed in 1..=200_u64 {
        for (total, count) in tally
            .iter_mut()
            .zip(run(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15)))
        {
            *total += count;
        }
    }
    assert!(tally.iter().all(|&count| count > 1000), "{tally:?}");
}
