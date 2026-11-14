# frozen_string_literal: true

require 'minitest/autorun'
require 'cache_machines'

class CacheMachinesTest < Minitest::Test
  MAX = (1 << 64) - 1
  Evicted = CacheMachines::Evicted

  def setup
    @cache = CacheMachines::Lru.new
  end

  def admit(key, candidate = 0, victim = 0)
    @cache.admit(key, candidate, victim)
  end

  def full_cache
    CacheMachines::Lru::CAPACITY.times { |key| assert_equal :inserted, admit(key) }
  end

  # What the cache shows of itself: equal before and after means nothing moved.
  def assert_unchanged(expected)
    before = [@cache.keys, @cache.victim]
    assert_equal expected, yield
    assert_equal before, [@cache.keys, @cache.victim]
  end

  def test_native_api_and_empty_state
    assert_nil CacheMachines::Lru.instance_method(:admit).source_location
    assert_match(/\A\d+\.\d+\.\d+\z/, CacheMachines::VERSION)
    assert_equal [64, 64], [CacheMachines::Lru::CAPACITY, @cache.capacity]
    assert_predicate @cache, :empty?
    assert_equal 0, @cache.size
    assert_empty @cache.keys
    assert_nil @cache.victim
    assert_unchanged(:miss) { @cache.access(1) }
    assert_unchanged(:absent) { @cache.remove(1) }
  end

  def test_hit_promotion_admission_ties_and_eviction
    full_cache
    assert_equal 0, @cache.victim
    assert_equal :hit, @cache.access(0)
    assert_equal 1, @cache.victim
    assert_unchanged(:outranked) { admit(100, 9, 9) }
    assert_unchanged(:outranked) { admit(100, 8, 9) }
    assert_equal Evicted.new(1), admit(100, 10, 9)
    assert_equal :hit, admit(2)
    assert_equal [2, 100, 0], @cache.keys.first(3)
    assert_unchanged(:miss) { @cache.access(1) }
  end

  def test_repeats_change_nothing
    admit(1)
    assert_unchanged(:hit) { admit(1) }
    assert_unchanged(:hit) { @cache.access(1) }
    assert_equal :removed, @cache.remove(1)
    assert_unchanged(:absent) { @cache.remove(1) }
  end

  def test_removal_clear_and_detached_snapshots
    assert_equal :inserted, admit(1)
    assert_equal :inserted, admit(2)
    snapshot = @cache.keys
    snapshot.clear
    assert_equal [2, 1], @cache.keys
    assert_equal :removed, @cache.remove(1)
    assert_equal [2], @cache.keys
    assert_nil @cache.clear
    assert_predicate @cache, :empty?
    assert_equal :inserted, admit(0, MAX, MAX)
  end

  def test_evicted_is_validated_like_every_other_value
    assert_equal 7, Evicted.new(7).key
    assert_equal Evicted.new(key: MAX), Evicted.new(MAX)
    [-1, MAX + 1].each { |v| assert_raises(RangeError) { Evicted.new(v) } }
    [1.0, '1', nil].each { |v| assert_raises(TypeError) { Evicted.new(v) } }
    assert_raises(NoMethodError) { CacheMachines.__evicted(1) }
  end

  def test_unsigned_integer_boundaries
    assert_equal :inserted, admit(MAX, MAX, MAX)
    assert_equal :hit, @cache.access(MAX)
    assert_equal [MAX], @cache.keys
    [-1, MAX + 1].each do |invalid|
      assert_raises(RangeError) { admit(invalid) }
      assert_raises(RangeError) { admit(1, invalid) }
      assert_raises(RangeError) { admit(1, 0, invalid) }
      assert_raises(RangeError) { @cache.access(invalid) }
      assert_raises(RangeError) { @cache.remove(invalid) }
    end
    assert_equal [MAX], @cache.keys
  end

  def test_strict_types_and_conversion_before_mutation
    coercible = Object.new
    def coercible.to_int = 1
    [1.0, 1.5, Float::INFINITY, Float::NAN, '1', nil, true, coercible].each do |invalid|
      assert_raises(TypeError) { admit(invalid) }
      assert_raises(TypeError) { admit(1, invalid) }
      assert_raises(TypeError) { admit(1, 0, invalid) }
      assert_raises(TypeError) { @cache.access(invalid) }
      assert_raises(TypeError) { @cache.remove(invalid) }
    end
    assert_predicate @cache, :empty?
    admit(1)
    admit(2)
    assert_raises(TypeError) { admit(1, 0, 'bad') }
    assert_equal [2, 1], @cache.keys
    assert_raises(ArgumentError) { @cache.admit(1) }
    assert_raises(ArgumentError) { @cache.admit(1, candidate_frequency: 0, victim_frequency: 0) }
    assert_raises(ArgumentError) { CacheMachines::Lru.new(64) }
  end

  # The defect: the eviction was applied before `Evicted.new` ran, so a raise
  # there lost the evicted key for good.
  def test_a_raise_while_building_the_answer_loses_nothing
    full_cache
    before = @cache.keys
    Evicted.singleton_class.alias_method(:original_new, :new)
    Evicted.define_singleton_method(:new) { |*| raise IOError, 'no memory' }
    assert_raises(IOError) { admit(100, 1, 0) }
    assert_equal before, @cache.keys
    cache = @cache
    Evicted.define_singleton_method(:new) { |*args| cache.remove(5) && original_new(*args) }
    assert_raises(RuntimeError) { admit(100, 1, 0) }
    assert_equal before - [5], @cache.keys, 'the re-entrant change stands, the outer call is not applied'
  ensure
    Evicted.singleton_class.alias_method(:new, :original_new)
    Evicted.singleton_class.remove_method(:original_new)
  end

  def test_an_answer_that_freezes_the_cache_applies_nothing
    full_cache
    before = @cache.keys
    cache = @cache
    Evicted.singleton_class.alias_method(:original_new, :new)
    Evicted.define_singleton_method(:new) { |*args| cache.freeze && original_new(*args) }
    assert_raises(FrozenError) { admit(100, 1, 0) }
    assert_predicate @cache, :frozen?
    assert_equal before, @cache.keys
  ensure
    Evicted.singleton_class.alias_method(:new, :original_new)
    Evicted.singleton_class.remove_method(:original_new)
  end

  def test_evicted_class_is_resolved_once_at_init
    full_cache
    CacheMachines.send(:remove_const, :Evicted)
    CacheMachines.const_set(:Evicted, String)
    GC.start
    answer = admit(100, 1, 0)
  ensure
    CacheMachines.send(:remove_const, :Evicted)
    CacheMachines.const_set(:Evicted, Evicted)
    assert_instance_of Evicted, answer
    assert_equal Evicted.new(0), answer
  end

  def test_frozen_mutations_fail_even_on_noop_paths
    admit(1)
    @cache.freeze
    assert_raises(FrozenError) { admit(1) }
    assert_raises(FrozenError) { @cache.access(9) }
    assert_raises(FrozenError) { @cache.remove(9) }
    assert_raises(FrozenError) { @cache.clear }
    assert_equal [1], @cache.keys
    assert_equal 1, @cache.size
  end

  def test_frozen_is_checked_before_the_arguments
    @cache.freeze
    assert_raises(FrozenError) { @cache.admit(-1, 0.5, nil) }
    assert_raises(FrozenError) { @cache.access('1') }
    assert_raises(FrozenError) { @cache.remove(2**64) }
  end

  def test_gc_compaction_and_instance_independence
    admit(MAX)
    100.times { CacheMachines::Lru.new }
    GC.start
    GC.compact if GC.respond_to?(:compact)
    assert_equal [MAX], @cache.keys
    assert_equal :hit, @cache.access(MAX)
    other = CacheMachines::Lru.new
    assert_predicate other, :empty?
    other.admit(0, 0, 0)
    assert_equal [MAX], @cache.keys
  end
end
