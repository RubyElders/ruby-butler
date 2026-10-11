# Native gem build snapshot

Build from the repository root. The image includes the current working-tree crate sources, Ruby installed through rv, Rust/Cargo, native build tools, and development libraries for ImageMagick, ICU, YAJL, PostgreSQL, MySQL, SQLite, XML and SSL. Clang/LLVM 19 is provided for Rust extensions that use Bindgen.

```sh
docker build -f docker/native/Dockerfile -t butler-native:snapshot .
docker run --rm -it -v "$PWD/../gems-corpus:/work/corpus:ro" butler-native:snapshot
```

Inside the container:

```sh
rb version
ruby --version
butler-compile /work/corpus/gems/rmagick/rmagick-7.0.4.gem /work/rmagick ruby
butler-compile /work/corpus/gems/ffi-yajl/ffi-yajl-2.7.11.gem /work/ffi-yajl ruby
butler-compile /path/to/charlock_holmes.gem /work/charlock ruby --with-cxxflags=-std=c++17
```

Use the actual archive paths from the corpus manifest. Destinations must not already exist. `butler-compile` and `butler-install` are the standalone installer examples; the CLI snapshot is not wired to that installer yet.

RubyGems installs the build helpers, the Ruby `pkg-config` gem, `libyajl2`, and Libdatadog 7 during image preparation. Their metadata is exported to `/opt/build-dependencies.json` for microgems; native build subprocesses use the minimal compatibility layer. Add gem-specific dependencies to that registry when needed. Cargo can download missing crates, and keeps its cache under `/usr/local/cargo`; mount a named volume there to retain downloads between containers.

Run the smoke check against the local corpus:

```sh
docker run --rm \
  -v "$PWD/../gems-corpus:/corpus:ro" \
  -v "$PWD/docker/native/smoke.rb:/smoke.rb:ro" \
  butler-native:snapshot ruby /smoke.rb
```

It compiles RMagick, FFI YAJL, Charlock Holmes, CommonMarker, Prometheus Client MMAP and Rubydex through Butler, then loads RMagick and FFI YAJL and exercises their APIs. Pass gem names after `/smoke.rb` to select a smaller set. The existing corpus runners supply their own dependency registry and force offline Cargo; use this check for the image's additional prerequisites and online Cargo downloads.

For the heavy builds, run this in two terminals with `job=grpc` and `job=libv8-node`. These require enough memory and CPU for both containers; the settings below allow eight jobs and up to 20 GiB per container. gRPC builds Protobuf first. MiniRacer builds after Libv8 succeeds. The runner saves timings and logs and checks runtime loading without RubyGems.

```sh
job=grpc
mkdir -p ../gems-corpus/docker-parallel
docker run --rm --cpus=8 --memory=20g --memory-swap=20g \
  -e RB_GEM_BUILD_JOBS=8 -e RB_GEM_BUILD_TIMEOUT=1800 \
  -v "$PWD/../gems-corpus:/corpus:ro" \
  -v "$PWD/../gems-corpus/docker-parallel:/results" \
  -v "$PWD/docker/native/heavy.rb:/heavy.rb:ro" \
  butler-native:snapshot ruby /heavy.rb "$job"
```

Use a fresh results directory for another run; successful installation destinations cannot be overwritten.

The eight-job parallel run built gRPC 1.81.1 in 25 minutes 41 seconds and passed its load check with RubyGems disabled. Libv8 Node 24.12.0.1 still exceeded the 30-minute command limit while compiling, so MiniRacer was not attempted. These timings describe cold source builds on the local machine, not a guaranteed duration on other hardware.

Libv8 Node subsequently exceeded a 90-minute limit. A checkpoint of its extracted source and objects was resumed through `NativeBuilder::build` with a three-hour limit and completed in another 83 minutes 42 seconds. The two attempts consumed about 2 hours 54 minutes of command time, including recompilation after regenerating build inputs; this is not a fresh-build timing or a successful 90-minute installation. MiniRacer then compiled in 7 seconds and evaluated JavaScript through microgems with RubyGems disabled. Logs and the separate continuation report are in `../gems-corpus/docker-v8-90m/`. Runtime checks use manifest activation rather than adding every dependency directory directly to Ruby's load path.

The default Ruby is 4.0.7. Build with `--build-arg RUBY_VERSION=3.3.7` for the older Ddtrace version; its matching Libdatadog prerequisite is included. Sass Embedded's remote installer API remains unsupported. Heavy source builds can still exceed the default 30-minute command limit; override `RB_GEM_BUILD_TIMEOUT` or `RB_GEM_BUILD_JOBS` when running the container.

The image is currently for x86_64 Linux. It retains sources and Cargo for further compatibility testing. It does not include the corpus or publish an image remotely.
