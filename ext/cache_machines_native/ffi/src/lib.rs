//! Thin native binding; all ordering and admission decisions belong to the core.

use std::cell::RefCell;

use cache_machines::{Admission, Lru};
use magnus::scan_args::{get_kwargs, scan_args};
use magnus::typed_data::Obj;
use magnus::{
    Error, Integer, RHash, Ruby, Symbol, TryConvert, Value, function, method, prelude::*,
};

const CAPACITY: usize = 64;

/// Reject implicit conversions, especially Float truncation in Magnus integers.
#[derive(Clone, Copy)]
struct Number(u64);

impl TryConvert for Number {
    fn try_convert(value: Value) -> Result<Self, Error> {
        if Integer::from_value(value).is_none() {
            return Err(Error::new(
                Ruby::get_with(value).exception_type_error(),
                "key and frequency must be Integers",
            ));
        }
        u64::try_convert(value).map(Self)
    }
}

#[magnus::wrap(class = "CacheMachines::Lru", free_immediately, size)]
struct Wrapper(RefCell<Lru<u64, CAPACITY>>);

impl Wrapper {
    const fn new() -> Self {
        Self(RefCell::new(Lru::new()))
    }

    fn admit(
        ruby: &Ruby,
        rb_self: Obj<Self>,
        args: &[Value],
    ) -> Result<(Symbol, Option<u64>), Error> {
        rb_self.check_frozen()?;
        let args = scan_args::<(Number,), (), (), (), RHash, ()>(args)?;
        let (key,) = args.required;
        let (candidate, victim) = get_kwargs::<_, (Number, Number), (), ()>(
            args.keywords,
            &["candidate_frequency", "victim_frequency"],
            &[],
        )?
        .required;
        // Only native values cross the borrow; Ruby allocations happen afterwards.
        let outcome = rb_self.0.borrow_mut().admit(key.0, candidate.0, victim.0);
        let (status, evicted) = match outcome {
            Admission::Hit => ("hit", None),
            Admission::Inserted => ("inserted", None),
            Admission::Evicted(key) => ("evicted", Some(key)),
            Admission::Rejected => ("rejected", None),
            _ => {
                return Err(Error::new(
                    ruby.exception_runtime_error(),
                    "unknown admission",
                ));
            }
        };
        Ok((ruby.to_symbol(status), evicted))
    }

    fn access(rb_self: Obj<Self>, key: Number) -> Result<bool, Error> {
        rb_self.check_frozen()?;
        Ok(rb_self.0.borrow_mut().access(key.0))
    }

    fn remove(rb_self: Obj<Self>, key: Number) -> Result<bool, Error> {
        rb_self.check_frozen()?;
        Ok(rb_self.0.borrow_mut().remove(key.0))
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
}

#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("CacheMachines")?;
    let class = module.define_class("Lru", ruby.class_object())?;
    class.const_set("CAPACITY", CAPACITY)?;
    class.define_singleton_method("new", function!(Wrapper::new, 0))?;
    class.define_method("admit", method!(Wrapper::admit, -1))?;
    class.define_method("access", method!(Wrapper::access, 1))?;
    class.define_method("remove", method!(Wrapper::remove, 1))?;
    class.define_method("clear", method!(Wrapper::clear, 0))?;
    class.define_method("keys", method!(Wrapper::keys, 0))?;
    class.define_method("size", method!(Wrapper::len, 0))?;
    class.define_method("empty?", method!(Wrapper::is_empty, 0))?;
    class.define_method("victim", method!(Wrapper::victim, 0))
}
