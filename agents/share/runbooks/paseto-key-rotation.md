# Runbook: rotating the PASETO signing key

This is the OPS-1 slice for **cross-service token signing key rotation**
— distinct from [`integrity-activation.md`](integrity-activation.md)'s
MAC-key rotation, which protects audit rows, not tokens. See
[`authentication-sessions.md`](../authentication-sessions.md) §5 for the
design and
[`authentication/authentication-service-with-loco/config/keys/README.md`](../../../authentication/authentication-service-with-loco/config/keys/README.md)
for the crate-local configuration reference this runbook operationalises
(and corrects — see §5).

## What you're rotating, and why it's safe to

authentication-service signs every cross-service PASETO v4.public token
with one Ed25519 **primary** key (`TOKEN_PRIVATE_KEY_SEED` /
`TOKEN_PRIVATE_KEY_FILE`) and publishes its public key(s) at
`/.well-known/paseto-keys`. Every peer (`case`, `person`, … + link-graph)
verifies **offline** against a key set it fetched at boot or refreshes on
a timer — no peer ever calls back to authentication-service per request.
That offline-verification property is exactly what makes rotation safe
without downtime: two keys can be valid simultaneously (`TOKEN_
ADDITIONAL_PUBLIC_KEYS`), so old tokens keep verifying while new ones are
signed by the new key, and each peer's own key set catches up on its own
schedule.

## The one thing every peer gets wrong if you don't know it

**A peer does not refetch on `UnknownKid`.** There is no such trigger
anywhere in `authentication-verifier` or any peer's `src/auth.rs` — only
two things make a peer refresh its key set:

1. **Boot** — `<ENTITY>_PASETO_KEYS_URL` fetched once at process start.
2. **The refresh timer** — `<ENTITY>_PASETO_KEYS_REFRESH_SECS`, default
   `3600`. **`0` disables it.** If `<ENTITY>_PASETO_KEYS_URL` is unset
   (the peer is on a static `<ENTITY>_PASETO_KEYS` env value instead),
   there is no refresh loop at all, ever.

So after you promote a new primary, a given peer keeps verifying with
its *old* key set for anywhere from zero seconds up to its own
`_REFRESH_SECS` — or forever, if it isn't configured to poll. Size the
wait in step 3 below to the **slowest** peer in your fleet, not the
fastest.

## Rotation sequence

Config mechanics (env vars, `kid` derivation, key-set shape) are in the
crate README linked above; this is the *order* that keeps every peer
verifying throughout, worked out from how the refresh timer actually
behaves rather than assumed:

1. **Generate** a new 32-byte Ed25519 seed (there is no `cargo loco task`
   for this, unlike the MAC key's `integrity_key` task — the only
   documented path is a shell one-liner, see the README). Do **not**
   promote it yet.
2. **Publish it as an *additional* key first**, primary unchanged:
   ```sh
   export TOKEN_ADDITIONAL_PUBLIC_KEYS=<new base64url public key>
   ```
   Restart authentication-service. `/.well-known/paseto-keys` now lists
   two keys; the service still *signs* with the old primary.
3. **Wait for every peer to have polled at least once** — the largest
   `<ENTITY>_PASETO_KEYS_REFRESH_SECS` across your deployed services (or
   restart the slower ones instead of waiting, if that's cheaper). Skip
   this wait entirely and you will 401 every peer that hasn't refreshed
   the moment step 4 lands.
4. **Promote**: swap `TOKEN_PRIVATE_KEY_SEED` to the new seed, move the
   *old* public key into `TOKEN_ADDITIONAL_PUBLIC_KEYS` (drop the new
   one — it's the primary now). Restart. New tokens sign with the new
   key; tokens already issued under the old key still verify everywhere,
   because every peer has, per step 3, already loaded it as a known
   `kid`.
5. **Wait at least `TOKEN_EXPIRATION`** (default `300`s) for every
   already-issued old-key token to expire naturally.
6. **Retire**: drop the old public key from `TOKEN_ADDITIONAL_PUBLIC_KEYS`,
   restart. A token signed by the retired key now gets `UnknownKid`
   everywhere.

Steps 2, 4, and 6 each require an authentication-service restart (keys
load once, lazily, on first use — not at boot, see §4). Nothing here is
transactional across the fleet: each peer catches up independently, on
its own timer.

## Checks — what to look at before and after each step

There is **no endpoint or metric on any peer that lists the `kid`s it
currently holds** — this is a real observability gap, not an oversight
in this runbook. What you have instead:

| Check | Command | What it tells you |
|---|---|---|
| Authoritative key set | `curl https://<auth-host>/.well-known/paseto-keys \| jq '.keys[].kid'` | The ground truth — what should eventually be everywhere |
| A peer's last refresh | grep its logs for `PASETO key set fetched over HTTP` (boot) or `refreshed PASETO key set` (timer tick), field `keys=<n>` | *How many* keys it holds, not *which* — count alone still tells you if it caught the two-key window |
| A peer stuck on stale keys | grep for `PASETO key-set refresh failed; keeping current keys` | The peer is still trying, but every attempt is failing — check network/DNS to authentication-service, not the key rotation itself |
| End-to-end proof | mint a token, `curl -H "Authorization: Bearer <token>" https://<peer-host>/api/<plural>/whoami` | The one always-protected route in every entity service; a live functional check beats reading logs |

## Symptoms → checks → actions

**"Some requests started 401ing right after I promoted the new key."**
Body reads `no verification key for kid "<new kid>"`. You skipped or
under-sized step 3's wait — that peer hasn't polled since the promotion.
Actions: restart the affected peer (forces an immediate boot-time
refetch), or wait out its `_REFRESH_SECS`. Confirm with the `whoami`
check above once resolved.

**"A peer never seems to pick up new keys, no matter how long I wait."**
Check whether `<ENTITY>_PASETO_KEYS_URL` is even set for that peer — if
it's running on static `<ENTITY>_PASETO_KEYS` env JSON instead, it will
**never** refetch, by design (`configuration.md` §4). Or check
`<ENTITY>_PASETO_KEYS_REFRESH_SECS=0`, which disables the timer
entirely, deliberately. Either requires a restart to fix, not a wait.

**"Every request everywhere started failing after a rotation."**
Check for a duplicate `kid` in the published set — the verifier refuses
to build at all with one (`"duplicate kid … in key set"`), so a peer
that loads a duplicated set falls back to its *previous* good set (on a
refresh failure) or to `<ENTITY>_PASETO_KEYS` / an empty reject-all set
(on a boot failure) — never to a broken one. This only happens if the
key set was hand-assembled rather than generated by
`TOKEN_ADDITIONAL_PUBLIC_KEYS`, since that path can't collide (`kid` is
a hash of the key). Fix authentication-service's key set and have
affected peers refetch.

**"401s mention `unsupported algorithm", not `UnknownKid`."**
A refetch will not fix this — the peer binary doesn't implement that
algorithm at all. This only arises if authentication-service ever
publishes a non-Ed25519 key (a future post-quantum rollout,
[`authentication-sessions.md`](../authentication-sessions.md) §5.1);
resolve by upgrading the peer binary, not by waiting.

**"authentication-service is up and healthy, but token issuance and key
publication are both broken."**
This is `TOKEN_PRIVATE_KEY_SEED`/`_FILE` missing or malformed in
production. Because keys load **lazily on first use, not at boot**
(`App::initializers` is deliberately empty here — nothing forces the key
material to resolve until something needs it), the process passes its
own health check and only fails on the first call that touches
`/.well-known/paseto-keys` or token issuance/redemption. The panic
message names the exact fix:
`refusing to sign with the built-in development seed in production: set
TOKEN_PRIVATE_KEY_SEED (base64url 32-byte Ed25519 seed) or
TOKEN_PRIVATE_KEY_FILE`. Set the seed and restart; there is no live
remediation short of that.

## What this runbook cannot help you do

- **Inspect which `kid`s a running peer currently trusts.** No such
  endpoint exists on any service. The nearest proxy is the `keys=<n>`
  count in its own boot/refresh log line, or the functional `whoami`
  probe above.
- **Force a peer to refetch on demand.** There is no admin endpoint or
  signal for this; a restart is the only lever.
- **See a rotation-in-progress dashboard.** No metric exports key count,
  `kid`, or verification-failure reasons on any peer today — the
  `authentication-verifier` crate's own docs note `unsupported_key_count()`
  as "worth exporting", but no service does.

If any of the above becomes a recurring operational pain, it's a code
change (a `/keys/status` endpoint, a Prometheus gauge for held key
count), not something this runbook can work around.
