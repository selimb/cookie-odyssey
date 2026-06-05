# Web Stack

I went with Axum and minijinja.

## minijinja

Askama is quite popular, but I can't imagine having to wait for a re-compile
when updating templates.
I went with minijinja since I already know Jinja2.
I initially went with Tera because it seemed more popular, but:

- minijinja supports rendering a fragment (block), which integrates nicely with HTMX!
- minijinja's `context!` macro is quite nice, compared to Tera's `let mut context = ...`.
