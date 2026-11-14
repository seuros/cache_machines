//! Thin native binding; all ordering and admission decisions belong to the core.

use std::cell::RefCell;
use std::sync::OnceLock;

use cache_machines::{Access, Admission, Lru, Refusal, Removal};
use magnus::typed_data::Obj;
use magnus::value::Opaque;
use magnus::{Error, Integer, RClass, Ruby, TryConvert, Value, function, method, prelude::*};

const CAPACITY: usize = 64;

/// A key or a frequency: an Integer in `0..2**64`. Implicit conversions,
/// Float truncation included, are refused.
#[derive(Clone, Copy)]
struct Number(u64);

impl TryConvert for Number {
    fn try_convert(value: Value) -> Result<Self, Error> {
        let ruby = Ruby::get_with(value);
        if Integer::from_value(value).is_none() {
            return Err(Error::new(
                ruby.exception_type_error(),
                "key and frequency must be Integers",
            ));
        }
        u64::try_convert(value).map(Self).map_err(|_| {
            Error::new(
                ruby.exception_range_error(),
                "key and frequency must be in 0..2**64",
            )
        })
    }
}

/// Ruby symbol of a refusal: the snake case of the core variant.
fn refusal(ruby: &Ruby, refused: Refusal) -> Value {
    ruby.to_symbol(match refused {
        Refusal::Outranked => "outranked",
        Refusal::ZeroCapacity => "zero_capacity",
    })
    .as_value()
}

/// `CacheMachines::Evicted`, resolved once at init: reassigning the constant
/// later changes nothing the cache builds.
static EVICTED: OnceLock<Opaque<RClass>> = OnceLock::new();

fn evicted_class(ruby: &Ruby) -> Result<RClass, Error> {
    EVICTED
        .get()
        .map(|class| ruby.get_inner(*class))
        .ok_or_else(|| {
            Error::new(
                ruby.exception_runtime_error(),
                "the cache_machines extension is not initialised",
            )
        })
}

/// What `Evicted.new` stores: the key, validated as the core's `u64`.
const fn checked_evicted(key: Number) -> u64 {
    key.0
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
        key: Number,
        candidate_frequency: Number,
        victim_frequency: Number,
    ) -> Result<Value, Error> {
        rb_self.check_frozen()?;
        // Everything that can raise happens before the core decides.
        let evicted = evicted_class(ruby)?;
        // The core decides on a copy; the copy is kept only once the answer
        // exists, so a raise in `Evicted.new` loses no evicted key.
        let before = rb_self.0.borrow().clone();
        let mut after = before.clone();
        let answer = match after.admit(key.0, candidate_frequency.0, victim_frequency.0) {
            Ok(Admission::Hit) => ruby.to_symbol("hit").as_value(),
            Ok(Admission::Inserted) => ruby.to_symbol("inserted").as_value(),
            Ok(Admission::Evicted(key)) => evicted.funcall("new", (key,))?,
            Err(refused) => return Ok(refusal(ruby, refused)),
        };
        // `Evicted.new` may have frozen the cache: then nothing is applied.
        rb_self.check_frozen()?;
        let mut held = rb_self.0.borrow_mut();
        if *held != before {
            // `Evicted.new` called back in and changed the cache.
            return Err(Error::new(
                ruby.exception_runtime_error(),
                "the cache changed while the answer was built; the admission was not applied",
            ));
        }
        *held = after;
        Ok(answer)
    }

    fn access(ruby: &Ruby, rb_self: Obj<Self>, key: Number) -> Result<Value, Error> {
        rb_self.check_frozen()?;
        let access = rb_self.0.borrow_mut().access(key.0);
        Ok(ruby
            .to_symbol(match access {
                Access::Hit => "hit",
                Access::Miss => "miss",
            })
            .as_value())
    }

    fn remove(ruby: &Ruby, rb_self: Obj<Self>, key: Number) -> Result<Value, Error> {
        rb_self.check_frozen()?;
        let removal = rb_self.0.borrow_mut().remove(key.0);
        Ok(ruby
            .to_symbol(match removal {
                Removal::Removed => "removed",
                Removal::Absent => "absent",
            })
            .as_value())
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

#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("CacheMachines")?;
    let evicted: RClass = module.const_get("Evicted")?;
    // Held for good: removing the constant must not let the GC take it.
    ruby.gc_register_mark_object(evicted);
    EVICTED.set(evicted.into()).map_err(|_| {
        Error::new(
            ruby.exception_runtime_error(),
            "the cache_machines extension was initialised twice",
        )
    })?;
    module.define_singleton_method("__evicted", function!(checked_evicted, 1))?;
    let class = module.define_class("Lru", ruby.class_object())?;
    class.const_set("CAPACITY", CAPACITY)?;
    class.define_singleton_method("new", function!(Wrapper::new, 0))?;
    class.define_method("admit", method!(Wrapper::admit, 3))?;
    class.define_method("access", method!(Wrapper::access, 1))?;
    class.define_method("remove", method!(Wrapper::remove, 1))?;
    class.define_method("clear", method!(Wrapper::clear, 0))?;
    class.define_method("keys", method!(Wrapper::keys, 0))?;
    class.define_method("size", method!(Wrapper::len, 0))?;
    class.define_method("empty?", method!(Wrapper::is_empty, 0))?;
    class.define_method("victim", method!(Wrapper::victim, 0))?;
    class.define_method("capacity", method!(Wrapper::capacity, 0))
}
