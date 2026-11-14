use magnus::{Error, Ruby, function, method, prelude::*};
use matryona_machines::ruby::Class;

use crate::{CAPACITY, CLASSES, Classes, Wrapper, checked_evicted};

/// Defines `CacheMachines::Lru` and resolves the classes the gem defined.
#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("CacheMachines")?;
    // Held for good: removing the constant must not let the GC take it.
    let evicted = Class::resolve(ruby, module, "Evicted")?;
    CLASSES.set(ruby, Classes { evicted })?;
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
