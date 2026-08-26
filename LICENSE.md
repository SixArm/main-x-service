# License

This monorepo is multi-licensed. Each subproject declares its own SPDX
license expression in its manifest, and that declaration is
authoritative for that subproject:

- **Rust crates** (`Cargo.toml` `license`):
  `MIT OR Apache-2.0 OR BSD-3-Clause OR GPL-2.0-only OR GPL-3.0-only`
- **Front-end projects** (`package.json` `license`):
  `MIT OR Apache-2.0`

`OR` is the SPDX disjunction: you may use each subproject under **any
one** of its listed licenses, at your option.

## License texts

- MIT: <https://spdx.org/licenses/MIT.html>
- Apache-2.0: <https://spdx.org/licenses/Apache-2.0.html>
- BSD-3-Clause: <https://spdx.org/licenses/BSD-3-Clause.html>
- GPL-2.0-only: <https://spdx.org/licenses/GPL-2.0-only.html>
- GPL-3.0-only: <https://spdx.org/licenses/GPL-3.0-only.html>

## Copyright

Copyright © Joel Parker Henderson (<joel@joelparkerhenderson.com>).

## Notes

- A few older manifests still carry the deprecated SPDX identifiers
  `GPL-2.0` / `GPL-3.0`; these mean `GPL-2.0-only` / `GPL-3.0-only`
  and are being normalized (see `tasks.md` PRO-H3).
- Documentation and specification files in this repository are offered
  under the same terms as the subproject they document, or the Rust
  crate expression above for repo-level documents.
