# frozen_string_literal: true

require 'minitest/autorun'
require 'open3'
require 'rbconfig'

# The class of the value the extension wraps (Lru) is fixed when it loads.
# Nothing a caller does later, redefining the constant or `Object.const_get`,
# reaches a cache being built. Each case runs in a fresh process: the class is
# fixed once per process, at load.
class ClassLookupTest < Minitest::Test
  LIB = File.expand_path('../lib', __dir__)

  PRELUDE = <<~RUBY
    require 'cache_machines'
    C = CacheMachines
  RUBY

  # Runs `body` after PRELUDE in a new Ruby; answers its last line of output.
  def isolated(body)
    out, err, status = Open3.capture3(RbConfig.ruby, '-I', LIB, '-e', PRELUDE + body)
    assert status.success?, "exited #{status.exitstatus}: #{err}"
    out.lines.last&.chomp
  end

  # Bug: an Lru constant redefined before the first `new` made the value an
  # instance of the new class, without #admit.
  def test_a_redefined_lru_constant_does_not_change_the_cache_class
    assert_equal 'true inserted 1', isolated(<<~RUBY)
      original = C::Lru
      C.send(:remove_const, :Lru)
      C.const_set(:Lru, Class.new)
      cache = original.new
      puts [cache.instance_of?(original), cache.admit(1, 0, 0), cache.size].join(' ')
    RUBY
  end

  # Bug: building the first cache ran `Object.const_get`, Ruby code in the
  # middle of a native constructor.
  def test_building_a_cache_runs_no_const_get
    assert_equal '0 Lru', isolated(<<~RUBY)
      hits = 0
      Object.define_singleton_method(:const_get) do |*args|
        hits += 1
        super(*args)
      end
      cache = C::Lru.new
      puts [hits, cache.class.name.split('::').last].join(' ')
    RUBY
  end

  # Bug: a redefined `Object.const_get` that raised hit an `unwrap` in the
  # extension: an uncatchable fatal, and the process exited.
  def test_a_raising_const_get_does_not_kill_the_process
    assert_equal 'inserted 1', isolated(<<~RUBY)
      Object.define_singleton_method(:const_get) { |*| raise IOError, 'lookup ran in a constructor' }
      cache = C::Lru.new
      puts [cache.admit(1, 0, 0), cache.size].join(' ')
    RUBY
  end
end
