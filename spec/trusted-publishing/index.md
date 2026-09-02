# Trusted Publishing

Trusted Publishing is a secure way to publish your Rust crates from CI/CD platforms like GitHub Actions and GitLab CI/CD without manually managing API tokens. It uses OpenID Connect (OIDC) to verify that your workflow is running from your repository, then provides a short-lived token for publishing.

Instead of storing long-lived API tokens in your repository secrets, Trusted Publishing allows your CI/CD platform to authenticate directly with crates.io using cryptographically signed tokens that prove the workflow's identity.

We intend to add "Trusted Publishing" when it is production-ready across all our code forges (GitHub.com, GitLab.com, Codeberg.org, etc.) and across all our target destinations (Rust crates.io, NPM npmjs.com, etc.).

This is about the publishing *mechanism* (how a `cargo publish` authenticates), not who decides to run it. That governance question is answered elsewhere: [`AI_STATEMENT.md`](../../AI_STATEMENT.md) §5/§6 and [`GOVERNANCE.md`](../../GOVERNANCE.md) — this repository's AI tooling may judge an already-merged version bump ready to release and execute the publish, for a crate this repository already publishes to crates.io. A move to Trusted Publishing changes the credential the publish step authenticates with; it does not change who or what is authorized to trigger that step.
