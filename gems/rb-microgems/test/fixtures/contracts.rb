require 'microgems/compatibility'
require 'stringio'
require 'rubygems/package'
require 'rubygems/ext'
require 'rubygems/command'

def check(value, message)
  raise message unless value
end

def raises(type)
  begin
    yield
  rescue type
    return
  end
  raise "Expected #{type}"
end

root = File.expand_path(ENV.fetch('FIXTURE_ROOT'))

check(!defined?(Bundler), 'Bundler loaded')
check(!$LOADED_FEATURES.any? { |file| file.include?('rubygems/core_ext') }, 'Full RubyGems loaded')
check(Gem::Command.build_args == ['--with-extra=a b'], 'Build arguments changed')
check(gem('fixture', '~> 2.1'), 'Activation failed')
check(!gem('fixture', '>= 2'), 'Repeated activation changed state')
check(Gem.loaded_specs.fetch('fixture').metadata.fetch('purpose') == 'test', 'Metadata missing')
check(Gem.loaded_specs.fetch('fixture').full_gem_path == root, 'Root changed')
raises(Gem::LoadError) { gem('fixture', '< 2') }
raises(Gem::MissingSpecError) { gem('missing') }
raises(LoadError) { require 'rubygems/remote_fetcher' }
raises(NoMethodError) { Gem.unsupported_api }

header = Gem::Package::TarHeader.new(name: 'agent/file', size: 3)
tar = header.to_s + "abc" + ("\0" * 509) + ("\0" * 1024)
entries = []
Gem::Package::TarReader.new(StringIO.new(tar)) do |reader|
  reader.each { |entry| entries << [entry.full_name, entry.read(1), entry.read] }
end
check(entries == [['agent/file', 'a', 'bc']], 'Tar contents changed')
raises(Gem::Package::TarInvalidError) { Gem::Package::TarReader.new(StringIO.new(tar[0, 514])).each(&:read) }
corrupt = tar.dup
corrupt[0] = 'X'
raises(Gem::Package::TarInvalidError) { Gem::Package::TarReader.new(StringIO.new(corrupt)).each(&:read) }
results = []
Gem::Ext::Builder.run([Gem.ruby, '--disable=gems', '-e', 'print ARGV.fetch(0)', 'a; echo incorrect'], results)
check(results.last == 'a; echo incorrect', 'Command arguments interpreted by shell')
raises(Gem::InstallError) { Gem::Ext::Builder.run([Gem.ruby, '--disable=gems', '-e', 'exit 3'], []) }

record = { 'name' => 'choice', 'version' => '1.0', 'root' => root, 'platform' => 'ruby' }
MicroGems.configure('gems' => [record, record.merge('version' => '2.0')])
check(Gem::Specification.find_by_name('choice', '< 2').version.to_s == '1.0', 'Requirement lookup ignored')
check(Gem::Specification.find_by_name('choice').version.to_s == '2.0', 'Latest version not chosen')
raises(Gem::MissingSpecError) { Gem::Specification.find_by_name('choice', '> 3') }
raises(ArgumentError) { MicroGems.configure('gems' => [record, record]) }

$LOAD_PATH.delete(File.join(root, 'lib'))
MicroGems.configure('gems' => [record.merge('name' => 'auto')])
require 'fixture_api'
check(FIXTURE_API == 42 && Gem.loaded_specs.key?('auto'), 'Require did not activate explicit dependency')
puts 'contracts passed'
