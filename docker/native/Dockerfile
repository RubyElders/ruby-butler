FROM rust:1.98.1-trixie

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl xz-utils git build-essential pkg-config \
    autoconf automake libtool bison cmake ninja-build python3 \
    clang-19 libclang-19-dev llvm-19-dev \
    libssl-dev libreadline-dev zlib1g-dev libyaml-dev libffi-dev \
    libgdbm-dev libncurses-dev libicu-dev libyajl-dev \
    imagemagick libmagickwand-dev libpq-dev default-libmysqlclient-dev \
    libsqlite3-dev libxml2-dev libxslt1-dev libidn-dev libcurl4-openssl-dev \
    libbz2-dev liblzma-dev patch unzip \
    && rm -rf /var/lib/apt/lists/*

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

ARG RV_VERSION=0.5.3
ARG RUBY_VERSION=4.0.7
RUN mkdir -p /tmp/rv-install /opt/rubies \
    && curl -fsSL "https://github.com/spinel-coop/rv/releases/download/v${RV_VERSION}/rv-x86_64-unknown-linux-gnu.tar.xz" \
       | tar -xJ --strip-components=1 -C /tmp/rv-install \
    && install -m 0755 /tmp/rv-install/rv /usr/local/bin/rv \
    && RV_UPDATE_MODE=none rv ruby install --install-dir /opt/rubies "$RUBY_VERSION" \
    && ln -s "/opt/rubies/ruby-${RUBY_VERSION}" /opt/ruby \
    && rm -rf /tmp/rv-install

ENV PATH="/opt/ruby/bin:${PATH}" \
    LIBCLANG_PATH=/usr/lib/llvm-19/lib \
    CARGO_NET_OFFLINE=false \
    RB_GEM_BUILD_TIMEOUT=1800 \
    RB_GEM_BUILD_JOBS=2 \
    RB_GEM_BUILD_DEPENDENCIES=/opt/build-dependencies.json

RUN gem install --no-document --bindir /usr/local/bin rake rake-compiler rake-compiler-dock rb_sys mini_portile2 ffi-compiler libyajl2 pkg-config \
    && gem install --no-document --bindir /usr/local/bin libdatadog --version '~> 7.0.0.1.0' \
    && ruby -rjson -e 'File.write("/opt/build-dependencies.json", JSON.pretty_generate(Gem::Specification.map { |s| {name: s.name, version: s.version.to_s, platform: s.platform.to_s, root: s.full_gem_path, require_paths: s.full_require_paths, executables: s.executables, bindir: s.bindir} }))'

WORKDIR /opt/butler
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates ./crates
COPY gems ./gems
RUN cargo build --locked --release -p rb-cli -p rb-gem-installer --examples --bin rb \
    && install -m 0755 target/release/rb /usr/local/bin/rb \
    && install -m 0755 target/release/examples/compile /usr/local/bin/butler-compile \
    && install -m 0755 target/release/examples/install /usr/local/bin/butler-install

ENV RB_RUBIES_DIR=/opt/rubies

WORKDIR /work
CMD ["bash"]
