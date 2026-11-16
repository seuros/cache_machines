# frozen_string_literal: true

require 'minitest/autorun'
require 'cache_machines'

# Sealing and the native boundary.
class BindingTest < Minitest::Test
  Evicted = CacheMachines::Evicted

  # `class ForgedEvicted < Evicted; end` raises TypeError, yet Ruby binds the
  # constant before `inherited` runs: the subclass leaks. Its `initialize`
  # skips the check by binding `Data#initialize` directly.
  def forged_class
    return Object.const_get(:ForgedEvicted) if Object.const_defined?(:ForgedEvicted)

    eval('class ForgedEvicted < CacheMachines::Evicted; end', TOPLEVEL_BINDING, __FILE__, __LINE__)
    flunk 'a named subclass of Evicted was accepted'
  rescue TypeError
    Object.const_get(:ForgedEvicted).class_eval do
      def initialize(**) = Data.instance_method(:initialize).bind_call(self, key: -1)
    end
    Object.const_get(:ForgedEvicted)
  end

  def test_evicted_is_sealed
    assert_raises(TypeError) { Class.new(Evicted) { def initialize(**) = nil } }
    assert_raises(NoMethodError) { Evicted.allocate }
    assert_raises(RangeError) { Evicted.new(1).with(key: -1) }
    # The named form raises too, but the constant leaks: the native boundary,
    # not the seal, is the authority.
    assert_operator forged_class, :<, Evicted
  end

  def test_lru_is_sealed
    assert_raises(TypeError) { CacheMachines::Lru.allocate }
    error = assert_raises(TypeError) { Class.new(CacheMachines::Lru) }
    assert_equal 'CacheMachines::Lru cannot be subclassed', error.message
  end

  def test_a_forged_evicted_is_never_a_cache_answer
    forged = forged_class.new(key: 0)
    assert_equal(-1, forged.key)
    # The cache reads no Evicted from Ruby, so a forged one reaches nothing,
    # and it never equals a real answer.
    cache = CacheMachines::Lru.new
    CacheMachines::Lru::CAPACITY.times { |key| cache.admit(key, 0, 0) }
    answer = cache.admit(100, 1, 0)
    assert_instance_of Evicted, answer
    refute_equal forged, answer
  end

  def test_the_extension_alone_is_a_name_error
    lib = File.expand_path('../lib', __dir__)
    script = 'begin; require "cache_machines_native/cache_machines_native"; ' \
             'rescue NameError => e; print e.class; end'
    output = IO.popen([RbConfig.ruby, '-I', lib, '-e', script], err: %i[child out], &:read)
    assert_equal 'NameError', output
  end
end
