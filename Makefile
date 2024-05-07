.PHONY: dev

setup:
	cargo install cargo-watch sea-orm-cli

dev:
	cargo watch --quiet --watch src --watch templates --exec "run"