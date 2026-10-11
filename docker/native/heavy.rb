require 'json'
require 'fileutils'
require 'rubygems/package'

$stdout.sync = true
job = ARGV.fetch(0)
steps = case job
        when 'grpc' then %w[google-protobuf grpc]
        when 'libv8-node' then %w[libv8-node mini_racer]
        else abort 'Expected grpc or libv8-node'
        end
rows = JSON.parse(File.read('/corpus/manifest.json')).fetch('gems')
records = JSON.parse(File.read(ENV.fetch('RB_GEM_BUILD_DEPENDENCIES')))
dependencies = "/results/#{job}-dependencies.json"
report = {ruby: RUBY_DESCRIPTION, jobs: ENV.fetch('RB_GEM_BUILD_JOBS'), timeout: ENV.fetch('RB_GEM_BUILD_TIMEOUT'), results: []}
steps.each do |name|
  row = rows.find { |gem| gem.fetch('name') == name } or abort "Missing #{name}"
  archive = "/corpus/#{row.fetch('path')}"
  destination = "/results/builds/#{name}"
  FileUtils.mkdir_p(File.dirname(destination))
  File.write(dependencies, JSON.pretty_generate(records))
  started = Process.clock_gettime(Process::CLOCK_MONOTONIC)
  passed = File.open("/results/#{name}.log", 'w') do |log|
    system({'RB_GEM_BUILD_DEPENDENCIES' => dependencies}, 'butler-compile', archive, destination, 'ruby', out: log, err: log)
  end
  result = {name: name, passed: passed, seconds: (Process.clock_gettime(Process::CLOCK_MONOTONIC) - started).round(3), partial_installation: !passed && File.exist?(destination)}
  report[:results] << result
  File.write("/results/#{job}.json", JSON.pretty_generate(report) + "\n")
  puts JSON.generate(result)
  abort "Failed #{name}; dependent step skipped" unless passed
  spec = Gem::Package.new(archive).spec
  records << {'name' => spec.name, 'version' => spec.version.to_s, 'platform' => spec.platform.to_s, 'root' => destination, 'require_paths' => spec.require_paths, 'metadata' => spec.metadata, 'executables' => spec.executables, 'bindir' => spec.bindir}
end
File.write("/results/#{job}-runtime.json", JSON.pretty_generate(gems: records))
code = job == 'grpc' ? 'require "grpc"; raise unless Gem.loaded_specs.key?("grpc"); puts GRPC::VERSION' : 'require "mini_racer"; raise unless Gem.loaded_specs.key?("mini_racer"); raise unless MiniRacer::Context.new.eval("6 * 7") == 42; puts "MiniRacer runtime passed"'
runtime = system({'RB_MICROGEMS_MANIFEST' => "/results/#{job}-runtime.json"}, 'ruby', '--disable=gems', '-I/opt/butler/gems/rb-microgems/lib', '-rmicrogems/compatibility', '-e', code)
report[:runtime_passed] = runtime
File.write("/results/#{job}.json", JSON.pretty_generate(report) + "\n")
abort 'Runtime check failed' unless runtime
