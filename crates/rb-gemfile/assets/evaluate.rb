require_relative "gemfile"
require_relative "sources"
require_relative "gemspec"
require_relative "compatibility"

begin
  evaluator = RubyButler::Gemfile.new
  RubyButler.define_singleton_method(:evaluator) { evaluator }
  evaluator.eval_gemfile(ARGV.fetch(0))
  File.write(ARGV.fetch(1), JSON.generate(manifest: evaluator.manifest, inputs: evaluator.inputs))
rescue StandardError, ScriptError => error
  warn "#{error.class}: #{error.message}"
  warn error.backtrace.first(8).join("\n")
  exit 1
end
