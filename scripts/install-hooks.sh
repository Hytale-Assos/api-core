#!/usr/bin/env bash
# Point git at the repository-local hooks (formatting hook on pre-commit).
# Run once per clone. Idempotent.
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"
git config core.hooksPath .githooks
echo "git hooks installed (core.hooksPath=.githooks)"
