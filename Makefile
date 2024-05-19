.PHONY: dev

setup:
	cargo install cargo-watch sea-orm-cli

# orm-migrate-new:
# sea-orm-cli migrate generate

orm-migrate-refresh:
	sea-orm-cli migrate --verbose refresh

# TODO: Keep in sync with .env...
orm-gen:
	sea-orm-cli generate entity \
		--output-dir entities/src \
		--database-url sqlite://data/db.sqlite \
		--with-serde serialize \
		--ignore-tables seaql_migrations \
		--ignore-tables tower_sessions \
		--lib

admin:
	cargo run -- create-admin

fix-all:
	cargo fix
	cargo clippy --all-targets --fix

local-s3:
	docker run \
	--rm \
	-p 4566:4566 \
	localstack/localstack:s3-latest

dev:
	cargo watch --quiet --watch src --watch templates --exec "run server"

tw:
	bun run tailwindcss -i ./assets/app.css -o ./assets/dist/app.css

tw-dev:
	bun run tailwindcss -i ./assets/app.css -o ./assets/dist/app.css --watch