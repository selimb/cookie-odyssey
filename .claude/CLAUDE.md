# Cookie Odyssey

A personal travel journal web application.
Optimized for low bandwidth (mobile LTE) and cheap hosting (small VPS).

## Stack

- Backend: Rust + Axum
- Auth: axum-login + tower-sessions (SQLite store)
- Database: SQLite + SeaORM
- Templating: Minijinja (Jinja2 syntax)
- Frontend: HTMX + Stimulus.js
- Styling: Tailwind CSS + DaisyUI
- Storage: Azure Blob Storage

## Commands

- Lint Rust: `just lint-rust` or `just lint-rust-fix` to auto-fix
- Lint Typescript: `just lint-js` or `just lint-js-fix` to auto-fix
- Lint HTML: `just lint-html`
- Test Rust: `cargo test`

There is no test suite for the app as a whole, so verify behavior manually.
Pure functions get unit tests in a `#[cfg(test)]` module next to them.

## Project layout

```
src/                  Rust backend (Axum app)
tools/                Development scripts
app_config/           App configuration (env vars, AppConfig)
entities/             SeaORM-generated entities -- do not edit manually
migration/            SeaORM migrations
templates/            Minijinja HTML templates
assets/js/            TypeScript source (Stimulus controllers, HTMX setup)
assets/css/           Tailwind CSS
```
