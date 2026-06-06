---
paths:
  - "assets/**/*.ts"
  - "assets/**/*.css"
  - "templates/**/*.html"
---

# Frontend rules

## Division of responsibility

- **HTMX** handles navigation and form submissions.
  If something can be expressed with HTMX attributes alone, use HTMX.
- **Stimulus** handles anything that requires imperative JS: file uploads, gallery interactions, theme toggling, toast display, etc.

Do not reach for Stimulus for things HTMX can handle declaratively.

## Stimulus controllers

Controllers live in `assets/js/`.
Register them in `app.ts`.
Use Stimulus naming conventions: `data-controller` values are kebab-case and map to the controller's identifier.

TypeScript is used throughout.
Prefer typed controller targets and values via the typed wrapper in `stimulus-typed.ts`.

## API calls from controllers

When a Stimulus controller needs to submit data and receive an HTML fragment back, use `htmx.ajax()` so HTMX handles the swap.
For non-HTML responses (upload URLs, SAS tokens), use `fetch` directly and validate the response shape with `zod`.

## TypeScript

- Use `type` for all type definitions -- never `interface`
- Strict mode is on
- Use `zod` for runtime validation of any data coming from API responses
- No `any` casts without a comment explaining why
- When suppressing an ESLint rule, add a reason after `--`: `// eslint-disable-next-line no-console -- TODO [error-reporting]`
