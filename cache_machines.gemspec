# frozen_string_literal: true

require_relative 'lib/cache_machines/version'

Gem::Specification.new do |spec|
  spec.name = 'cache_machines'
  spec.version = CacheMachines::VERSION
  spec.authors = ['Abdelkader Boudih']
  spec.email = ['oss@seuros.com']
  spec.summary = 'Bounded native LRU and admission policy for Ruby'
  spec.description = 'A native binding to the allocation-free cache-machines Rust core. ' \
                     'Tracks numeric keys and caller-measured admission frequencies; ' \
                     'payload storage and frequency measurement remain with the caller.'
  spec.homepage = 'https://github.com/seuros/cache_machines'
  spec.license = 'MIT'
  spec.required_ruby_version = '>= 4.0.0'
  spec.metadata['source_code_uri'] = spec.homepage
  spec.metadata['bug_tracker_uri'] = "#{spec.homepage}/issues"
  spec.metadata['rubygems_mfa_required'] = 'true'
  spec.metadata['cargo_crate_name'] = 'cache_machines_native'
  spec.metadata['cargo_manifest_path'] = 'ext/cache_machines_native/ffi/Cargo.toml'

  # README.md under core/ is compiled by include_str! and must ship with sources.
  spec.files = Dir.glob(%w[
    lib/**/*.rb sig/**/*.rbs ext/**/*.{rb,rs,toml,md} ext/**/LICENSE
    Cargo.toml Cargo.lock README.md CHANGELOG.md LICENSE.txt
  ]).select { |path| File.file?(path) }
  spec.require_paths = ['lib']
  spec.extensions = ['ext/cache_machines_native/extconf.rb']
  spec.add_dependency 'rb_sys', '~> 0.9', '>= 0.9.130'
end
