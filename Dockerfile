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
    xz-utils \
    && rm -rf /var/lib/apt/lists/*

# Install rv once (cached layer)
ARG RV_VERSION=0.5.3
ARG RV_DOWNLOAD_URL=https://github.com/spinel-coop/rv/releases/download/v${RV_VERSION}/rv-x86_64-unknown-linux-gnu.tar.xz

RUN mkdir -p /tmp/rv-install \
    && curl -fsSL "$RV_DOWNLOAD_URL" | tar -xJ --strip-components=1 -C /tmp/rv-install \
    && install -m 0755 /tmp/rv-install/rv /usr/local/bin/rv \
    && rm -rf /tmp/rv-install

# Stage for Ruby 4.0.1 compilation
FROM base AS ruby-4-0-1

RUN mkdir -p /opt/rubies && \
    RV_UPDATE_MODE=none rv ruby install --install-dir /opt/rubies 4.0.1 \
    && rm -f /usr/local/bin/rv

# Stage for Ruby 3.4.5 compilation  
FROM base AS ruby-3-4-5

RUN mkdir -p /opt/rubies && \
    RV_UPDATE_MODE=none rv ruby install --install-dir /opt/rubies 3.4.5 \
    && rm -f /usr/local/bin/rv

# Final stage - copy compiled Rubies
FROM base AS final

# Install ShellSpec (modern shell testing framework)
RUN git clone https://github.com/shellspec/shellspec.git /tmp/shellspec \
    && cd /tmp/shellspec \
    && make install PREFIX=/usr/local \
    && rm -rf /tmp/shellspec \
    && rm -f /usr/local/bin/rv

# Copy compiled Ruby installations from parallel stages
COPY --from=ruby-4-0-1 /opt/rubies/ruby-4.0.1 /opt/rubies/ruby-4.0.1
COPY --from=ruby-3-4-5 /opt/rubies/ruby-3.4.5 /opt/rubies/ruby-3.4.5

# Create test user (non-root for realistic testing)
RUN useradd -m -s /bin/bash testuser \
    && mkdir -p /home/testuser/.gem \
    && chown -R testuser:testuser /home/testuser \
    && mkdir -p /app \
    && chown -R testuser:testuser /app \
    && mkdir -p /app/report \
    && chown -R testuser:testuser /app/report

# Set up working directory
WORKDIR /app

# Switch to test user
USER testuser
