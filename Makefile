.PHONY: dev

setup:
	cargo install cargo-watch sea-orm-cli

orm-migrate-refresh:
	sea-orm-cli migrate --verbose refresh

# TODO: Keep in sync with .env...
orm-gen:
	sea-orm-cli generate entity \
		--output-dir entities/src \
		--database-url sqlite://data/db.sqlite \
		--with-serde serialize \
		--lib

dev:
	cargo watch --quiet --watch src --watch templates --exec "run"