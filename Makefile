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
		--lib

local-s3:
	docker run \
	--rm \
	-p 4566:4566 \
	localstack/localstack:s3-latest

dev:
	cargo watch --quiet --watch src --watch templates --exec "run server"