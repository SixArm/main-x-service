.PHONY: github-pages ci ci-full ci-minimum-supported-rust-version test-benchmarks test-coverage hooks hooks-off

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
# bin/make-ci-minimum-supported-rust-version. Pass CRATES="path/to/crate ..." to narrow it.
ci-minimum-supported-rust-version:
	bin/make-ci-minimum-supported-rust-version $(CRATES)

# Run the criterion benchmarks (the same step ci-full runs).
test-benchmarks:
	scripts/ci-check.sh bench

# Measure test coverage of the un-gated suite with cargo-tarpaulin, one
# report per crate under target/tarpaulin/ (spec/testing §1.5). A report,
# not a gate. Pass CRATES="path/to/crate ..." to narrow it.
test-coverage:
	bin/make-test-coverage $(CRATES)

# Opt in to the pre-push hook (the fast tier under CI_LOCAL_BUDGET,
# default 600 s); `make hooks-off` undoes it.
hooks:
	scripts/install-hooks.sh

hooks-off:
	scripts/install-hooks.sh --uninstall
