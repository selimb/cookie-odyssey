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
		--with-serde both \
		--ignore-tables seaql_migrations \
		--ignore-tables tower_sessions \
		--lib

admin:
	cargo run -- create-admin

fix-all:
	cargo fix
	cargo clippy --all-targets --fix

azurite:
	docker compose up

dev:
	cargo watch --quiet --watch src --watch templates --exec "run server"

tw:
	bun run tailwindcss -i ./assets/app.css -o ./assets/dist/app.css

tw-dev:
	bun run tailwindcss -i ./assets/app.css -o ./assets/dist/app.css --watch