# General code style

Applies to all languages.

## No Unicode characters

Do not use Unicode characters in code, comments, or markdown.
Use ASCII equivalents instead, e.g. `--` for em dash.

## Line breaks

In markdown, only break lines at the end of a sentence.
In code comments, break lines to keep within 80 characters _and_ at the end of each sentence.

## Comment anchors

Use `[anchor-name]` tags to link related comments across files or locations.
A comment containing only an anchor marks a definition; a reference points to it:

```rust
// [toast]              <- definition in toast.rs
// See [toast]          <- reference elsewhere
```

Anchors work in any comment syntax.
This makes it easy to grep in both directions.

## SYNC comments

Mark a type or constant with `// SYNC` when it must be kept in sync with a corresponding definition in another language (Rust <-> TypeScript):

```rust
// SYNC JournalEntryMediaCommitItem
pub struct JournalEntryMediaCommitItem {}
```

```typescript
// SYNC JournalEntryMediaCommitItem
type JournalEntryMediaCommitItem = {};
```
