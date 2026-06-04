---
paths:
  - "templates/**/*.html"
---

# Template rules

## Engine

Minijinja -- Jinja2 syntax: `{% %}` control flow, `{{ }}` output, `{# #}` comments.
Undefined variables are a hard error (`UndefinedBehavior::Strict`).

## Layout

All pages extend `base.html` and fill in `{% block content %}`.
The base template loads assets, sets up HTMX Boost on `<body>`, and includes the navbar and toast component.

## Shared components

Reusable macros and partials live in `common/`.
Import with `{% import "common/datetime.html" as dt %}` and call as `{{ dt.some_macro(...) }}`.

## Assets

Use the `asset()` function for all JS/CSS references -- never hardcode `/assets/dist/` paths:

```jinja
<link rel="stylesheet" href="{{ asset('css/app.css') }}" />
```

This resolves the key against `manifest.json` at runtime, giving content-hashed URLs.

## Styling

- Use **DaisyUI** component classes for all UI primitives (`btn`, `card`, `form-control`, `alert`, `modal`, etc.)
- Use **Tailwind** utility classes to fill gaps DaisyUI doesn't cover
- Two themes: `night` (dark) and `nord` (light), toggled via `data-theme` on `<html>`

## HTMX

- HTMX Boost is enabled globally on `<body hx-boost="true">` -- standard `<a>` and `<form>` navigations are automatically boosted
- Use explicit `hx-get` / `hx-post` / `hx-target` / `hx-swap` attributes for HTMX routes
- HTMX routes start with `/hx/`; pass their URLs from the handler as `href_*` variables -- never hardcode them in templates

## Fragment blocks

Blocks intended for HTMX partial rendering use a `fragment_` prefix (e.g. `fragment_media_list`, `fragment_comment_list`).
The handler calls `render_ctx_fragment` with the block name; full-page renders pass `None` and render the whole template.

## URLs

Never construct URLs in templates.
Receive all URLs as `href_*` context variables from the handler.
