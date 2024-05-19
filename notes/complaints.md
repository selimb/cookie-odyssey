# Complaint Jar

This is a collection of complaints regarding Rust and/or its ecosystem.

## serde, config-rs

1. Parsing stops at first error.
2. config-rs error messages don't/can't refer to the environment variable name.
3. Defining defaults is awkward.
   Either through `set_default` on the `ConfigBuilder`, or through `#[serde(default="default_fn")]` which [doesn't support string literals](https://github.com/serde-rs/serde/issues/368).

## Frontend

No blessed way of bundling assets (JS/CSS) or linking to them from HTML templates.
TBH this is a pain point in most "classic" server-side frameworks.
[django-components](https://github.com/EmilStenstrom/django-components/tree/master) (discovered in
https://www.youtube.com/watch?v=3GObi93tjZI) has a neat UX, although I don't think it bundles/minifies
anything though.

https://pen.so/2023/07/31/asset-pipeline-for-rust/ looks good.

## HTMX

Is this really supposed to be easier than Next.js + TRPC, or even API + SPA?
You still need to add routes for every little action, and the routes are now tightly coupled
with where they're called from...

### Boost + Error Handling

ARGH! If submitting a form that redirects, or clicking on a link, and the response returns an error (e.g. 404), then the redirect does not happen (although the URL is still updated).
