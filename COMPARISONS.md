# Comparisons

Where the Main X Index sits relative to adjacent kinds of systems.
These are orientation notes, not benchmarks or feature-war tables.

## Master data management (MDM) platforms

Commercial MDM suites (e.g. Informatica MDM, IBM InfoSphere, Reltio)
cover similar ground: golden records, probabilistic matching, merge,
survivorship, stewardship queues. The Main X Index differs by being
**one entity per service** (federated, not monolithic), open source,
Rust end to end, and spec-driven — with matching algorithms shipped as
standalone, dependency-light matcher crates you can embed anywhere.

## Healthcare master patient indexes (EMPI)

EMPI products focus on the person entity with clinical integrations.
The person/worker registries here follow the same matching literature
(Jaro-Winkler, deterministic short-circuits, review queues) and expose
FHIR R5 surfaces, but the family generalizes the pattern to ten entity
types and adds cross-service linking with a read-model graph.

## Project-management tools

The project-portfolio-management subproject deliberately competes with
Jira/Asana-class tools (one recursive Plan tree, boards, workflows,
OKRs, flow metrics, forecasting) while remaining a registry with
matching and merge — a combination those tools do not offer.

## Identity providers

The authentication-service is a deliberately small SSO: passwordless
magic links, Postgres cookie sessions, PASETO v4.public tokens. It is
not an OIDC/OAuth2 provider like Keycloak; it trades protocol breadth
for offline verification with no shared secrets and no introspection
hop (see [agents/share/jwt.md](agents/share/jwt.md) for why JWT
sessions are rejected).

## Context

If you know of a system this should be compared against, see
[RFC.md](RFC.md) — that feedback is explicitly wanted.
