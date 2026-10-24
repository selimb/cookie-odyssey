---
name: review-branch
description: Review the current branch against its base for code quality, consistency, maintainability and correctness, and fix what is found. Use when the user asks for a review of the branch, or before opening a PR.
---

This skill reviews everything the branch changes relative to its base, using a fresh subagent that has not seen the implementation.
The reviewer edits files.

## Process

1. Find the base branch.
   Use the PR's base if one exists (`gh pr view --json baseRefName`), otherwise `main`.
   The diff to review is `git diff <base>...HEAD`, plus any uncommitted changes.
2. Run `just lint-rust`, `just lint-js`, `just lint-html`.
   Fix failures before reviewing.
3. Spawn one subagent with the brief below.
   Use a smart model.
   Tell it the base branch, and nothing about what you think is wrong, so that it is not anchored.
4. When it reports, read the diff of what it changed.
   Re-run the linters.
   Commit and push only if the user asked for that.
5. Report to the user:
   - What was fixed, including anything that was broken on purpose.
   - What was left alone, and why.
   - Anything that needs the user's decision.

## Brief for the reviewer

Review the changes of this branch against `<base>` (`git diff <base>...HEAD`).
Read the changed files in full, not only the diff.

Review for:

- Correctness: wrong results, missed cases, broken invariants, queries that skip `AuthBackend::filter_journal_entries`, data a User should not see.
- Consistency: with the surrounding code, with the Route naming and URL rules, and with the domain terms in `docs/LANGUAGE.md` and the ADRs in `docs/adr/`.
- Maintainability: duplication, abstractions that do not earn their keep, code that is more complicated than what it replaces.
- Comments: read the comment rules in `.claude/rules/general-code-style.md` and check every comment added by the diff.
  In particular, a comment that mentions behavior defined or enforced somewhere else must reference an anchor defined there.
  Search the added comments for such references, they are easy to miss.

Do not be afraid to break things if it improves code quality.
Renaming, moving, deleting and changing behavior or schema are all fine, including in existing code that the branch touches.
Do not keep something for backwards compatibility unless the user said to.
Keep every such change small enough to explain in one line.
Never run anything against `data/*.sqlite`.
Use a scratch database for any manual testing.

Fix what you find, and re-run the linters.
Do not commit.

Reply with:

- Fixed: each change, with the file, and a flag on every one that changes behavior or breaks something.
- Not fixed: each finding you left alone, with the reason.
- Questions: anything only the user can decide.

Do not report style nitpicks that the linters already enforce.
