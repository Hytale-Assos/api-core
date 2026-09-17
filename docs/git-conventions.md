# Git conventions

Conventions used in this repository. Keep commits readable, atomic and
reviewable.

## Branches

Branch off `develop` (integration branch) and target it with your PR.
`main` holds stable releases.

| Prefix | Usage |
|---|---|
| `feat/<slug>` | new feature |
| `fix/<slug>` | bug fix |
| `refactor/<slug>` | behavior-preserving rework |
| `docs/<slug>` | documentation only |
| `chore/<slug>` | tooling, deps, config, CI |

Examples: `feat/whitelist-write-scope`, `fix/hmac-replay-window`,
`refactor/split-routes`.

Slug rules: lowercase, hyphen-separated, short but descriptive.

## Commits

Format:

```
type(scope): short imperative message

Optional body explaining WHAT and WHY (not HOW).
Wrap at ~72 characters.

Optional footer (BREAKING CHANGE, refs).
```

### Types

| Type | Usage |
|---|---|
| `feat` | new feature (user-facing) |
| `fix` | bug fix |
| `refactor` | code change that neither fixes a bug nor adds a feature |
| `docs` | documentation only |
| `test` | adding or fixing tests |
| `chore` | build, deps, tooling, config |
| `perf` | performance improvement |
| `style` | formatting, no code meaning change |

### Scope

The touched area, short and lowercase. Common scopes here: `core`, `api`,
`auth`, `db`, `crypto`, `config`, `docs`.

Omit the scope only when the change is truly global.

### Rules

- **English**, imperative present tense (`add`, not `added` / `adds`).
- **No trailing period** on the subject line.
- Subject ≤ 72 characters. Blank line before the body.
- One logical change per commit: it should be revertable on its own.
- Never commit secrets (`.env`, keys, tokens). `.env.example` is fine.
- Reference issues in the footer when relevant: `Refs: #12`.

## Atomic commits

Split work by **intent**, not by file count. A good commit answers "what does
this change do?" without needing the others. Typical split for a feature:
migration → models/data access → route → tests.

Avoid mixing refactors with behavior changes in the same commit.

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

## Workflow

```bash
git switch develop && git pull
git switch -c feat/my-feature
# ... commits ...
git push -u origin feat/my-feature
# open a PR targeting develop
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

## Tests before pushing

Run these and ensure they pass:

```bash
cargo fmt
cargo clippy --all-targets
cargo test
```
