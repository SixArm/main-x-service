.PHONY: github-pages ci ci-full hooks hooks-off

# Push the docs site subdirectory out to the github-pages remote via
# git's subtree mechanism. See spec/monorepo-github-pages/index.md.
github-pages:
	bin/make-github-pages

# Run the CI gate locally (spec/local-ci/index.md). `ci` is the fast tier,
# change-aware against origin/main; `ci-full` adds deny, msrv, bench,
# test-db, front-end build and the enrolled Playwright suites.
ci:
	scripts/ci-local.sh fast

ci-full:
	scripts/ci-local.sh full

# Opt in to the pre-push hook (the fast tier under CI_LOCAL_BUDGET,
# default 600 s); `make hooks-off` undoes it.
hooks:
	scripts/install-hooks.sh

hooks-off:
	scripts/install-hooks.sh --uninstall
