---
paths:
  - "src/**/*.rs"
  - "app_config/**/*.rs"
---

# Rust backend rules

## Route types and naming

Three kinds of routes exist.
Both the URL prefix and the handler function name must reflect the kind:

| Kind | URL prefix | Handler prefix | Returns                                     |
| ---- | ---------- | -------------- | ------------------------------------------- |
| Page | _(none)_   | `page_`        | Full HTML page via `Templ`                  |
| HTMX | `/hx/`     | `hx_`          | HTML fragment or HTMX response headers      |
| API  | `/api/`    | `api_`         | Non-HTML (JSON, binary, machine-to-machine) |

Handler names always end with the HTTP method: `_get`, `_post`, `_put`, or `_delete` (e.g. `page_login_get`, `page_login_post`, `api_media_upload_proxy_put`).
POST/PUT/DELETE handlers do not need to share a URL with their corresponding GET -- prefer a `/hx/` URL for mutation handlers.

## URLs

All URLs are generated through the `Route` enum.
Never hardcode URL strings in handlers or pass them directly to templates.
Pass pre-computed `href_*` variables from the handler instead.
The one accepted exception is the root path `"/"`, which may appear as a literal.

## Handler return type

All handlers return `RouteResult` (`Result<Response, RouteError>`).
`RouteError` converts `DbErr`, `minijinja::Error`, and `anyhow::Error` into HTTP 500 via `?`.

## Error handling strategies

Choose based on context:

- **`RouteResult` / `RouteError`** -- default for unhandled/unexpected errors.
  Let `?` propagate; the error becomes a 500.
  Dev mode shows the full error chain; prod shows "Something went wrong".

- **`Toast`** -- for user-visible feedback in HTMX flows where the page content should not change (`HX-Reswap: none`).

- **`FormError`** -- for form validation failures.

- **`NotFound`** -- renders `oops.html`.
  Returns 200 (not 404) to avoid broken behavior with HTMX Boost.

- **Ad-hoc tuple responses** -- for middleware and non-template contexts: `(StatusCode::UNAUTHORIZED, "message").into_response()`.

## `Templ` extractor

Use `Templ` in any handler that renders HTML.
The default template context is defined in `TemplContext`.

Render methods:

- `templ.render("template.html")` -- no extra context
- `templ.render_ctx("template.html", context! { ... })` -- with extra context
- `templ.render_ctx_fragment("template.html", context! { ... }, Some("block_name"))` -- renders a single named block (HTMX partial updates)

## Form structs

Apply `#[serde(deserialize_with = "string_trim")]` to every `String` field in a form struct that comes from user input.

## Database queries

Prefer moving complex or reused database queries into a `queries.rs` module within the feature directory.
Simple one-liners (e.g. `User::delete_by_id(...).exec(&db).await?`) are fine inline in handlers.

## Permissions

The only permission levels are unauthenticated, authenticated, and `Admin`.

Route-level access control happens at the router via `permission_required!` and `login_required!` macros -- not inside handlers.

Row-level visibility happens inside queries.
`AuthBackend::filter_journal_entries` appends a `draft = false` filter for non-admins; call it on any query that selects journal entries so drafts stay hidden from regular users.
