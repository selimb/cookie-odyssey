.PHONY: dev

setup:
	cargo install cargo-watch sea-orm-cli

orm-migrate-refresh:
	sea-orm-cli migrate refresh

# TODO: Keep in sync with .env...
orm-gen:
	sea-orm-cli generate entity -o entities/src -u sqlite://data/db.sqlite

dev:
	cargo watch --quiet --watch src --watch templates --exec "run"