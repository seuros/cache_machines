# frozen_string_literal: true

require_relative 'cache_machines/version'

module CacheMachines
  # The core's `Admission::Evicted`: the admitted key replaced `key`, the least
  # recently used one; drop its payload. Like every key the core holds, `key`
  # is an Integer in 0..2**64-1: anything else raises RangeError (TypeError for
  # a non-Integer).
  Evicted = Data.define(:key) do
    def initialize(key:)
      key = CacheMachines.__send__(:__evicted, key)
      super
    end
  end
end

# A missing extension is an installation error. There is no Ruby policy
# fallback. It validates and builds the Data value above.
require 'cache_machines_native/cache_machines_native'

module CacheMachines
  private_class_method :__evicted

  # Sealed: no subclass can skip the check by overriding `initialize`, and no
  # bare `allocate` builds a value that never ran it.
  Evicted.singleton_class.undef_method(:allocate)
  Evicted.define_singleton_method(:inherited) { |_| raise TypeError, "#{Evicted} cannot be subclassed" }

  # Lru wraps native state and has no allocator: a subclass's `new` would
  # build an Lru anyway, so subclassing raises too.
  Lru.define_singleton_method(:inherited) { |_| raise TypeError, "#{Lru} cannot be subclassed" }
end
