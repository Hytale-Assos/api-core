---
paths:
  - '*'
---

# Commits

## Format

This repository uses [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <subject>
```

Example: `feat(sessions): reuse open session on session_started`

## Language

Always write in English: commit messages (subject and body), branch names, and
PR titles and descriptions. Never mix French or other languages into them,
even for internal or WIP commits.

## Types

| Type | Use for |
| --- | --- |
| feat | New user-facing feature or capability |
| fix | Bug fix |
| refactor | Code change that neither fixes a bug nor adds a feature |
| perf | Performance improvement |
| style | Formatting, naming, lint-only changes (no logic change) |
| test | Adding or correcting tests only |
| docs | Documentation only |
| chore | Tooling, dependencies, config, seeders, demo data |
| ci | CI workflow changes |
| build | Build system or package manager changes |

## Scope

- Lowercase and short, matching the domain touched: `modules`, `servers`,
  `sessions`, `whitelists`, `queue`, `auth`, `db`, `crypto`, `http`, `ingest`,
  `core`, `docs`, `tests`, `deps`, `git`, `rules`, `container`, `ci`.
- Omit the scope only when no single domain applies.

## Subject

- English, imperative mood, lowercase after the colon, no trailing period,
  72 characters max.
- Describe what the commit changes, not which files it touches.
- Optional body explaining WHAT and WHY (not HOW), wrapped at ~72 characters.
- Reference issues in the footer when relevant: `Refs: #12`.

## Branches

- Branch from `develop`, never from `main`. `main` only receives merges from
  `develop` via pull request.
- Never push directly to `main`. The only exception is a hotfix: branch
  `hotfix/<slug>` from `main`, fix, open a PR to `main`, then back-merge or
  re-apply the fix onto `develop`.
- Name branches `<type>/<kebab-case-slug>`, matching the commit type and the
  change in a few words: `feat/whitelist-write-scope`, `fix/hmac-replay`,
  `chore/deps-bump`. Append the issue number when one exists.
- Create the branch with `git switch -c <type>/<slug>` from an up-to-date
  `develop` (`git switch develop && git pull`, then branch).
- One branch per logical change; keep its scope small enough for a single
  reviewable PR.
- After the PR is merged, delete the local branch:
  `git switch develop && git pull && git branch -d <type>/<slug>`. Use `-D`
  only when the branch was never merged.
- Never rewrite history on shared branches: no force-push to `develop` or
  `main`.

## Structure

- Commit at every atomic unit of work: as soon as a function, handler,
  service method, migration, or test is added and the change builds (and its
  tests pass), commit it. Do not save up finished features into one commit:
  the history should narrate the whole development of the branch, step by
  step. The default state of the working tree is clean (`git status` empty).
- If something already committed needs rework later in the same branch, add a
  new commit on top instead of leaving uncommitted changes lying around.
- Keep commits atomic: one logical change each, that builds and passes its
  own tests. A good commit answers "what does this change do?" without
  needing the others.
- One increment per commit, with the tests that cover it in the same commit.
  A feature spreads over several increments (e.g. migration → model/repo →
  service → handler + route), each building and passing its own tests.
- Calibrate granularity to the smallest coherent, testable unit: never land
  dead code alone (a service method without its handler), and never bundle
  two concerns into one commit to save time.
- Plan the commit sequence before starting the work; it doubles as the work
  plan and gets adjusted as the work reveals itself.
- Intermediate commits are not individually deployable: they must build and
  pass their own tests, but deployability is only guaranteed at the merge
  boundary. Keep the seams small (a service method lands in the commit right
  before its handler).
- Never mix a refactor or chore with a feature commit. When a file straddles
  two logical changes, split it with `git add -p` and commit in dependency
  order.
- `Cargo.lock` changes ship with the `Cargo.toml` change that caused them.
- Never commit secrets or `.env` values; new env keys go into `.env.example`.
- Run the narrowest relevant test suite
  (`cargo test -- <filter>` or `cargo test <test_name>`) before committing a
  behavior change.
- Always re-run the complete test suite before pushing, never push on stale
  green.

## Tests before pushing

Run these and ensure they pass:

```bash
cargo fmt
cargo clippy --all-targets
cargo test
```

## Pull requests

- **Base branch**: target `develop` (never `main` directly).
- **Title**: same convention as commits, in English:
  `type(scope): short imperative summary`.
- **Size**: keep it reviewable. If the branch does several unrelated things,
  split it. A single feature + its tests + docs is a good size.
- **Description**: fill in the template below. Explain **why**, not just what.
- **Reviewers**: request at least one. Do not self-merge without approval.
- **Checks**: CI must be green (`cargo fmt --check`, `clippy`, `test`).
- **Merge**: squash or merge commit per project preference; delete the branch
  after merge.
- Link the issue: use `Closes #<n>` / `Refs: #<n>` in the description.

### PR template

```markdown
## What

<what this PR does, in 1–3 sentences>

## Why

<the problem it solves / motivation>

## Changes

- <main change>
- <main change>

## Testing

<how it was tested: commands, manual steps>

## Notes

<breaking changes, migrations, follow-ups, open questions>
```

## Examples

Good:

```
feat(auth): add HMAC replay protection

Reject requests whose X-Timestamp differs from now by more than
HMAC_TOLERANCE_SECS, preventing replay of captured payloads.
```

```
fix(db): reuse open session on session_started

Previously a new row was inserted on every webhook call, creating
duplicate open sessions for the same player.
```

Bad:

```
update stuff          # no type, vague
fix: fixed the bug.   # past tense, trailing period, no scope
feat(api): add routes + refactor db + fix tests   # too many intents
```
