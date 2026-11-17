.PHONY: github-pages ci ci-full ci-msrv hooks hooks-off

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

# Check and test every crate on Rust N-2 (the declared MSRV in ci/msrv.txt)
# and N-1 (the next minor up). Needs both toolchains installed; see
# scripts/ci-msrv.sh. Pass CRATES="path/to/crate ..." to narrow it.
ci-msrv:
	scripts/ci-msrv.sh $(CRATES)

# Opt in to the pre-push hook (the fast tier under CI_LOCAL_BUDGET,
# default 600 s); `make hooks-off` undoes it.
hooks:
	scripts/install-hooks.sh

hooks-off:
	scripts/install-hooks.sh --uninstall
