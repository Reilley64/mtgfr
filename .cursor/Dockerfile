# Cursor Cloud Agent environment for mtgfr (CI-parity toolchain + local Postgres).
# Do not COPY the repo — Cursor clones the correct commit into the workspace.

FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    ca-certificates \
    curl \
    git \
    libsqlite3-dev \
    pkg-config \
    postgresql \
    postgresql-client \
    protobuf-compiler \
    python3 \
    sudo \
    sysvinit-utils \
    unzip \
    xz-utils \
    && rm -rf /var/lib/apt/lists/*

# Passwordless sudo for the agent user (Cursor expectation).
RUN id -u ubuntu >/dev/null 2>&1 || useradd -m -s /bin/bash ubuntu \
    && usermod -aG sudo ubuntu \
    && echo "ubuntu ALL=(ALL) NOPASSWD:ALL" > /etc/sudoers.d/ubuntu \
    && chmod 0440 /etc/sudoers.d/ubuntu

# Allow password auth over TCP so DATABASE_URL / WEB_DATABASE_URL work.
RUN PG_CONF="$(find /etc/postgresql -name postgresql.conf | head -1)" \
    && PG_HBA="$(find /etc/postgresql -name pg_hba.conf | head -1)" \
    && sed -i "s/^#listen_addresses = 'localhost'/listen_addresses = 'localhost'/" "$PG_CONF" \
    && printf '\nhost all all 127.0.0.1/32 scram-sha-256\nhost all all ::1/128 scram-sha-256\n' >> "$PG_HBA"

# Seed role + databases to match CI / docker-compose defaults.
RUN service postgresql start \
    && su - postgres -c "psql -v ON_ERROR_STOP=1 -c \"CREATE USER mtgfr WITH PASSWORD 'mtgfr' SUPERUSER;\"" \
    && su - postgres -c "psql -v ON_ERROR_STOP=1 -c \"CREATE DATABASE mtgfr OWNER mtgfr;\"" \
    && su - postgres -c "psql -v ON_ERROR_STOP=1 -c \"CREATE DATABASE mtgfr_web OWNER mtgfr;\"" \
    && service postgresql stop

USER ubuntu
WORKDIR /home/ubuntu

# Rust stable + rustfmt/clippy (matches verify-jobs.yml).
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --default-toolchain stable --profile minimal -c rustfmt -c clippy
ENV PATH=/home/ubuntu/.cargo/bin:${PATH}

# just + cargo-nextest (CI uses setup-just + taiki-e/install-action).
RUN cargo install just --locked \
    && cargo install cargo-nextest --locked

# Bun pinned to .bun-version (1.3.14).
RUN curl -fsSL https://bun.sh/install | bash -s "bun-v1.3.14"
ENV PATH=/home/ubuntu/.bun/bin:${PATH}

ENV DATABASE_URL=postgresql://mtgfr:mtgfr@127.0.0.1:5432/mtgfr
ENV WEB_DATABASE_URL=postgresql://mtgfr:mtgfr@127.0.0.1:5432/mtgfr_web
