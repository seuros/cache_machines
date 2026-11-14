//! Thin native binding; all ordering and admission decisions belong to the core.

mod init;

use std::cell::RefCell;

use cache_machines::{Access, Admission, Lru, Refusal, Removal};
use magnus::typed_data::Obj;
use magnus::{Error, Ruby, Value, prelude::*};
use matryona_machines::ruby::{Class, Registry, Symbols, integer, keep, symbol};

const CAPACITY: usize = 64;

matryona_machines::symbols! {
    /// The core's refusals as Symbols: the snake case of each variant.
    Refusals for Refusal as "a refusal" {
        Outranked => "outranked",
        ZeroCapacity => "zero_capacity",
    }
}

matryona_machines::symbols! {
    /// The core's `access` answers as Symbols.
    Accesses for Access as "an access" {
        Hit => "hit",
        Miss => "miss",
    }
}

matryona_machines::symbols! {
    /// The core's `remove` answers as Symbols.
    Removals for Removal as "a removal" {
        Removed => "removed",
        Absent => "absent",
    }
}

/// Ruby symbol of a refusal: the snake case of the core variant.
fn refusal(ruby: &Ruby, refused: Refusal) -> Value {
    Refusals::symbol(ruby, refused).as_value()
}

/// `CacheMachines::Evicted`, resolved once at init: reassigning the constant
/// later changes nothing the cache builds.
struct Classes {
    evicted: Class,
}

static CLASSES: Registry<Classes> = Registry::new("cache_machines");

/// What `Evicted.new` stores: the key, validated as the core's `u64`.
fn checked_evicted(key: Value) -> Result<u64, Error> {
    integer(key, "key")
}

#[magnus::wrap(class = "CacheMachines::Lru", free_immediately, size)]
struct Wrapper(RefCell<Lru<u64, CAPACITY>>);

impl Wrapper {
    fn new() -> Self {
        Self(RefCell::new(Lru::new()))
    }

    fn admit(
        ruby: &Ruby,
        rb_self: Obj<Self>,
        key: Value,
        candidate_frequency: Value,
        victim_frequency: Value,
    ) -> Result<Value, Error> {
        // Frozen first, then the arguments: everything that can raise
        // happens before the core decides.
        rb_self.check_frozen()?;
        let key = integer(key, "key")?;
        let candidate = integer(candidate_frequency, "candidate_frequency")?;
        let victim = integer(victim_frequency, "victim_frequency")?;
        let evicted = CLASSES.get(ruby)?.evicted.get(ruby);
        // The core decides on a copy; the copy is kept only once the answer
        // exists, so a raise in `Evicted.new` loses no evicted key.
        keep(
            ruby,
            rb_self,
            &rb_self.0,
            |cache| cache.admit(key, candidate, victim),
            |admission| match admission {
                Ok(Admission::Hit) => Ok(symbol(ruby, "hit").as_value()),
                Ok(Admission::Inserted) => Ok(symbol(ruby, "inserted").as_value()),
                Ok(Admission::Evicted(key)) => evicted.funcall("new", (key,)),
                Err(refused) => Ok(refusal(ruby, refused)),
            },
        )
    }

    fn access(ruby: &Ruby, rb_self: Obj<Self>, key: Value) -> Result<Value, Error> {
        rb_self.check_frozen()?;
        let key = integer(key, "key")?;
        let access = rb_self.0.borrow_mut().access(key);
        Ok(Accesses::symbol(ruby, access).as_value())
    }

    fn remove(ruby: &Ruby, rb_self: Obj<Self>, key: Value) -> Result<Value, Error> {
        rb_self.check_frozen()?;
        let key = integer(key, "key")?;
        let removal = rb_self.0.borrow_mut().remove(key);
        Ok(Removals::symbol(ruby, removal).as_value())
    }

    fn clear(rb_self: Obj<Self>) -> Result<(), Error> {
        rb_self.check_frozen()?;
        rb_self.0.borrow_mut().clear();
        Ok(())
    }

    fn keys(&self) -> Vec<u64> {
        self.0.borrow().keys().collect()
    }

    fn len(&self) -> usize {
        self.0.borrow().len()
    }

    fn is_empty(&self) -> bool {
        self.0.borrow().is_empty()
    }

    fn victim(&self) -> Option<u64> {
        self.0.borrow().victim()
    }

    fn capacity(&self) -> usize {
        self.0.borrow().capacity()
    }
}
