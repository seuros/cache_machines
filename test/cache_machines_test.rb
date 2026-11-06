# frozen_string_literal: true

require 'minitest/autorun'
require 'cache_machines'

class CacheMachinesTest < Minitest::Test
  MAX = (1 << 64) - 1

  def setup
    @cache = CacheMachines::Lru.new
  end

  def admit(key, candidate = 0, victim = 0)
    @cache.admit(key, candidate_frequency: candidate, victim_frequency: victim)
  end

  def full_cache
    CacheMachines::Lru::CAPACITY.times { |key| assert_equal [:inserted, nil], admit(key) }
  end

  def test_native_api_and_empty_state
    assert_nil CacheMachines::Lru.instance_method(:admit).source_location
    assert_match(/\A\d+\.\d+\.\d+\z/, CacheMachines::VERSION)
    assert_equal 64, CacheMachines::Lru::CAPACITY
    assert_predicate @cache, :empty?
    assert_equal 0, @cache.size
    assert_empty @cache.keys
    assert_nil @cache.victim
    refute @cache.access(1)
    refute @cache.remove(1)
  end

  def test_hit_promotion_admission_ties_and_eviction
    full_cache
    assert_equal 0, @cache.victim
    assert @cache.access(0)
    assert_equal 1, @cache.victim
    assert_equal [:rejected, nil], admit(100, 9, 9)
    assert_equal [:rejected, nil], admit(100, 8, 9)
    assert_equal 1, @cache.victim
    assert_equal [:evicted, 1], admit(100, 10, 9)
    assert_equal [:hit, nil], admit(2)
    assert_equal [2, 100, 0], @cache.keys.first(3)
    refute @cache.access(1)
  end

  def test_removal_clear_and_detached_snapshots
    assert_equal [:inserted, nil], admit(1)
    assert_equal [:inserted, nil], admit(2)
    snapshot = @cache.keys
    snapshot.clear
    assert_equal [2, 1], @cache.keys
    assert @cache.remove(1)
    assert_equal [2], @cache.keys
    assert_nil @cache.clear
    assert_predicate @cache, :empty?
    assert_equal [:inserted, nil], admit(0, MAX, MAX)
  end

  def test_unsigned_integer_boundaries
    assert_equal [:inserted, nil], admit(MAX, MAX, MAX)
    assert @cache.access(MAX)
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
    assert_raises(ArgumentError) { @cache.admit(1, candidate_frequency: 0, victim_frequency: 0, typo: 1) }
    assert_raises(ArgumentError) { CacheMachines::Lru.new(64) }
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

  def test_gc_compaction_and_instance_independence
    admit(MAX)
    100.times { CacheMachines::Lru.new }
    GC.start
    GC.compact if GC.respond_to?(:compact)
    assert_equal [MAX], @cache.keys
    assert @cache.access(MAX)
    other = CacheMachines::Lru.new
    assert_predicate other, :empty?
    other.admit(0, candidate_frequency: 0, victim_frequency: 0)
    assert_equal [MAX], @cache.keys
  end
end
