.PHONY: dev

dev:
	cargo watch --quiet --watch src --watch templates --exec "run"