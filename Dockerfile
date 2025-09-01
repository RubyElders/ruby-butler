# Ruby Butler Integration Test Environment
# 
# Multi-stage build for parallel Ruby compilation and optimized caching

# Base stage with common dependencies
FROM debian:trixie-slim AS base

# Install system dependencies (cached layer)
RUN apt-get update && apt-get install -y \
    curl \
    build-essential \
    git \
    libssl-dev \
    libreadline-dev \
    zlib1g-dev \
    libncurses5-dev \
    libffi-dev \
    libgdbm-dev \
    libyaml-dev \
    libsqlite3-dev \
    libtool \
    bison \
    pkg-config \
    autoconf \
    autotools-dev \
    automake \
    libbz2-dev \
    ca-certificates \
    libjemalloc-dev \
    parallel \
    && rm -rf /var/lib/apt/lists/*

# Install ruby-install once (cached layer)
RUN curl -fsSL https://github.com/postmodern/ruby-install/releases/download/v0.9.3/ruby-install-0.9.3.tar.gz | tar -xzf - \
    && cd ruby-install-0.9.3 \
    && make install \
    && cd .. \
    && rm -rf ruby-install-0.9.3

# Stage for Ruby 3.2.4 compilation
FROM base AS ruby-3-2-4

RUN mkdir -p /opt/rubies && \
    MAKE_OPTS="-j$(nproc)" \
    ruby-install \
        --install-dir /opt/rubies/ruby-3.2.4 \
        --jobs $(nproc) \
        --cleanup \
        ruby 3.2.4 \
        -- --with-jemalloc \
    && /opt/rubies/ruby-3.2.4/bin/gem install bundler --no-document

# Stage for Ruby 3.4.5 compilation  
FROM base AS ruby-3-4-5

RUN mkdir -p /opt/rubies && \
    MAKE_OPTS="-j$(nproc)" \
    ruby-install \
        --install-dir /opt/rubies/ruby-3.4.5 \
        --jobs $(nproc) \
        --cleanup \
        ruby 3.4.5 \
        -- --with-jemalloc \
    && /opt/rubies/ruby-3.4.5/bin/gem install bundler --no-document

# Final stage - copy compiled Rubies
FROM base AS final

# Install BATS (Bash Automated Testing System)
RUN git clone https://github.com/bats-core/bats-core.git /tmp/bats-core \
    && cd /tmp/bats-core \
    && ./install.sh /usr/local \
    && rm -rf /tmp/bats-core

# Copy compiled Ruby installations from parallel stages
COPY --from=ruby-3-2-4 /opt/rubies/ruby-3.2.4 /opt/rubies/ruby-3.2.4
COPY --from=ruby-3-4-5 /opt/rubies/ruby-3.4.5 /opt/rubies/ruby-3.4.5

# Create test user (non-root for realistic testing)
RUN useradd -m -s /bin/bash testuser \
    && mkdir -p /home/testuser/.gem \
    && chown -R testuser:testuser /home/testuser

# Set up working directory
WORKDIR /app

# Switch to test user
USER testuser

# Set up environment
ENV RUBIES_DIR=/opt/rubies
ENV GEM_HOME=/home/testuser/.gem
ENV PATH="/app:$PATH"

# Default command runs BATS tests
CMD ["bats", "tests/integration"]
