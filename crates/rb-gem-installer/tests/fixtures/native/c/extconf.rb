require 'mkmf'
raise 'Missing nested build job limit' unless ENV.fetch('NJOBS') == '2' && ENV.fetch('MAKEFLAGS') == '-j2'
raise 'Full RubyGems was loaded' if $LOADED_FEATURES.any? { |path| path.include?('rubygems/core_ext/kernel_require') }
raise 'Bundler was loaded' if defined?(Bundler)
require 'open3'
output, status = Open3.capture2e(Gem.ruby, '-e', "print Gem.loaded_specs.fetch('dinner').version")
raise "Nested Ruby lost compatibility: #{output}" unless status.success? && output == '1.2.3'
create_makefile('butler_native')
