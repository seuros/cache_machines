# frozen_string_literal: true

require_relative 'cache_machines/version'

# A missing extension is an installation error. There is no Ruby policy fallback.
require 'cache_machines_native/cache_machines_native'
