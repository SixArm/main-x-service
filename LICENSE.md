# License

This monorepo is multi-licensed. Each subproject declares its own SPDX
license expression in its manifest, and that declaration is
authoritative for that subproject:

- **Rust crates** (`Cargo.toml` `license`):
  `MIT OR Apache-2.0 OR BSD-3-Clause OR GPL-2.0-only OR GPL-3.0-only OR OGL-UK-3.0 OR CC-BY-NC-SA-4.0`
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
- OGL-UK-3.0: <https://spdx.org/licenses/OGL-UK-3.0.html>
- CC-BY-NC-SA-4.0: <https://spdx.org/licenses/CC-BY-NC-SA-4.0.html>

## Copyright

Copyright © Joel Parker Henderson (<joel@joelparkerhenderson.com>).

## Notes

- A few older manifests still carry the deprecated SPDX identifiers
  `GPL-2.0` / `GPL-3.0`; these mean `GPL-2.0-only` / `GPL-3.0-only`
  and are being normalized (see `tasks.md` PRO-H3).
- Documentation and specification files in this repository are offered
  under the same terms as the subproject they document, or the Rust
  crate expression above for repo-level documents.

## Full license texts

The full text of each license is in its own directory:

- [MIT](LICENSES/mit/index.md)
- [Apache-2.0](LICENSES/apache-2.0/index.md)
- [BSD-3-Clause](LICENSES/bsd-3-clause/index.md)
- [GPL-2.0-only](LICENSES/gpl-2.0-only/index.md)
- [GPL-3.0-only](LICENSES/gpl-3.0-only/index.md)
- [OGL-UK-3.0](LICENSES/ogl-uk-3.0/index.md)
- [CC-BY-NC-SA-4.0](LICENSES/cc-by-nc-sa-4.0/index.md)
