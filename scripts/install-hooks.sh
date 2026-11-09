#!/usr/bin/env bash
#
# Opt in to this repo's tracked git hooks (hooks/), or opt out.
# spec/local-ci/index.md, tasks.md LCI-2.
#
# Usage:
#   scripts/install-hooks.sh              point core.hooksPath at hooks/
#   scripts/install-hooks.sh --uninstall  unset core.hooksPath
#
# The only hook today is pre-push, which runs the fast tier of the local
# CI gate under a time budget (CI_LOCAL_BUDGET, default 600 s). Skip it
# for one push with `git push --no-verify`.
#
# This sets the setting in this clone's .git/config only. It refuses to
# replace a core.hooksPath that points somewhere else, so a hook manager
# already in use is never silently disabled.

set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "${ROOT}"

current="$(git config --local --get core.hooksPath || true)"

case "${1:-}" in
  --uninstall)
    if [[ "${current}" == hooks ]]; then
      git config --local --unset core.hooksPath
      echo "install-hooks: core.hooksPath unset; git uses .git/hooks again"
    elif [[ -z "${current}" ]]; then
      echo "install-hooks: not installed; nothing to do"
    else
      echo "install-hooks: core.hooksPath is '${current}', not set by this script; left alone" >&2
      exit 1
    fi
    ;;
  "")
    if [[ -n "${current}" && "${current}" != hooks ]]; then
      echo "install-hooks: core.hooksPath is already '${current}'; refusing to replace it" >&2
      exit 1
    fi
    git config --local core.hooksPath hooks
    echo "install-hooks: core.hooksPath = hooks (pre-push runs the fast gate)"
    echo "install-hooks: undo with scripts/install-hooks.sh --uninstall"
    ;;
  *)
    echo "usage: scripts/install-hooks.sh [--uninstall]" >&2
    exit 2
    ;;
esac
