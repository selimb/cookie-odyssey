
# Setup global Rust binaries
setup-rust:
    cargo install cargo-watch sea-orm-cli

# Setup djlint
setup-djlint:
    uv tool install djlint

# =============================================================================
# Database
# =============================================================================

# orm-migrate-new:
# sea-orm-cli migrate generate

orm-migrate-refresh:
    sea-orm-cli migrate --verbose refresh

orm-gen:
    # TODO: Keep in sync with .env...
    sea-orm-cli generate entity \
        --output-dir entities/src \
        --database-url sqlite://data/db.sqlite \
        --with-serde both \
        --ignore-tables seaql_migrations \
        --ignore-tables tower_sessions \
        --lib

# Creates a new admin user
admin:
    cargo run -- create-admin

# =============================================================================
# Linting (Rust)
# =============================================================================
lint-rust:
    cargo fmt --check
    cargo check
    cargo clippy --all-targets

lint-rust-fix:
    cargo fmt
    cargo fix
    cargo clippy --all-targets --fix

# =============================================================================
# Linting (JS)
# =============================================================================
lint-js:
    bun run lint

lint-js-fix:
    bun run lint:fix

# =============================================================================
# Linting (HTML)
# =============================================================================
lint-html:
    djlint templates

# =============================================================================
# Build scripts
# =============================================================================

build-server:
    cargo build --release

dev-server:
    cargo watch --quiet --no-vcs-ignores -w src -w templates -w assets/dist/manifest.json --exec "run server"

build-js:
    bun run tools/build-js.ts

dev-js:
    # Need to watch src and templates because of tailwind.
    cargo watch --quiet -w assets/js -w assets/css -w src -w templates -- just build-js

# Build it all
build: build-server build-js

# Run all build tools in watch mode
dev:
    bun run tools/dev.ts

# =============================================================================
# Misc
# =============================================================================

azurite:
    docker compose up
