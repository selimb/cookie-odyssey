# Directory Structure

I like Django's approach of grouping by domain instead of grouping by model/view/controller.
In other words, I want this:

```
/
  src/
    journal/
      routes.rs
      queries.rs
      mutations.rs
    ...
```

instead of

```
  src/
    routes/
      journal.rs
    some-other-frameworky-thing/
      journal.rs
```

## Entities

[sea-orm](https://www.sea-ql.org/SeaORM/docs/migration/setting-up-migration/#workspace-structure)
generates all entities under a single directory.
I followed their recommendation of generating the entities in a separate `entities` crate.

## Templates

I would've liked to colocate `templates` for each domain in their respective directory,
but this leads to weird references.

TODO explain

## Summary

```
/
  entities/
    src/
      journal.rs
      ...
  migration/
    src/
      ...
  src/
    journal/
      routes.rs
  templates/
    journal/
      journal_list.html
```
