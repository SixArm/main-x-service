# ABAC policy cookbook

Six worked examples of the policy JSON consumed by the shared ABAC engine
(`authentication-verifier` crate, `src/abac.rs`) — see
[`agents/share/authorization-attributes.md`](../../agents/share/authorization-attributes.md)
for the full policy language. Point `<ENTITY>_ABAC_POLICY_FILE` at one of
these (or start from one) to try a rule shape without writing it from
scratch. Every file here is parsed by `abac.rs`'s
`every_example_policy_parses` test, so the cookbook can't silently drift
into invalid JSON.

- **`dept-scoped-read-deny.json`** — denies `read` to any subject whose
  `dept` attribute is not `cardiology`. Shows a `!`-negated `when` value
  on a plain subject attribute (no record needed, so this works on the
  coarse blanket-guard path).

- **`closed-case-write-deny.json`** — an `access=admin` override allow
  first, then denies `write` when the loaded record's
  `resource.status` is `closed`. Requires `evaluate_with_resource`
  (record-level, handler-opt-in) — the coarse guard never populates
  `resource.*`, so this rule is inert there.

- **`after-hours-deny.json`** — an `access=admin` override allow first,
  then denies `write` when `env.after_hours` is `true`. Requires
  `evaluate_with_context` (environment attributes derived by the service
  at its edge — clock, source IP, …).

- **`ownership.json`** — allows `read`/`write` when the record's
  `resource.owner` equals the caller's own subject id, via the `$sub`
  template. Expresses "you may act on what you own" without an explicit
  attribute per resource.

- **`masked-read-obligation.json`** — full `read` for `dept=cardiology`,
  and a fallback `read` for everyone else carrying the `mask` obligation.
  Shows an allow decision driving a masked response instead of a second
  endpoint.

- **`machine-peer-grant.json`** — allows `write` and `destructive` to any
  subject with `svc=true`. The first rule of the family's built-in
  default policy (`authorization-attributes.md` §5): machine peers get
  everything, since they carry no `access` tier of their own.
