#!/usr/bin/env bash
#
# Start / stop a service's throwaway Postgres for its DB-gated test suite.
#
# Every service crate carries a `compose.test.yaml` describing one
# Postgres 18 container: superuser `loco`/`loco` on port 5432, the
# database its `config/test.yaml` expects, the shared extension init from
# ci/postgres-init, and its data directory on tmpfs. Those settings match
# what CI provides (.github/workflows/ci.yml `test-db`), so a suite that
# passes here passes there for the same reasons.
#
# Usage:
#   scripts/test-db.sh up      <crate-path>   start it and wait until healthy
#   scripts/test-db.sh down    <crate-path>   stop and remove it
#   scripts/test-db.sh psql    <crate-path>   open a psql shell on it
#   scripts/test-db.sh logs    <crate-path>   tail the container log
#   scripts/test-db.sh url     <crate-path>   print the DATABASE_URL
#   scripts/test-db.sh status                 list every running test database
#   scripts/test-db.sh down-all               stop every one of them
#
# Typical run:
#   scripts/test-db.sh up organization/organization-service-with-loco
#   scripts/ci-check.sh test-db organization/organization-service-with-loco
#   scripts/test-db.sh down organization/organization-service-with-loco
#
# Two databases at once (they would otherwise both want port 5432):
#   TEST_DB_PORT=5433 scripts/test-db.sh up <other-crate>
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

CMD="${1:?usage: test-db.sh <up|down|psql|logs|url|status|down-all> [crate-path]}"

# Host port the container publishes. One variable name across every
# service, because you only ever set it to move *this* run out of the way.
PORT="${TEST_DB_PORT:-5432}"

die() { echo "test-db: $*" >&2; exit 1; }

# Resolve a crate path to its compose file, failing loudly rather than
# letting compose report a confusing "no configuration file" error.
compose_file() {
  local crate="${1:?usage: test-db.sh ${CMD} <crate-path>}"
  crate="${crate%/}"
  local file="${crate}/compose.test.yaml"
  [[ -f "${file}" ]] || die "no ${file} — is '${crate}' a service crate?"
  printf '%s' "${file}"
}

# The database name the crate's compose file declares. Parsed from the one
# place it is written down, so this script cannot disagree with the
# container it just started.
db_name() {
  sed -n 's/^[[:space:]]*POSTGRES_DB:[[:space:]]*\([A-Za-z0-9_]*\).*/\1/p' "$1" | head -1
}

container_name() {
  sed -n 's/^[[:space:]]*container_name:[[:space:]]*\([A-Za-z0-9_.-]*\).*/\1/p' "$1" | head -1
}

compose() {
  local file="$1"; shift
  # `podman compose` shells out to whichever compose provider is
  # installed; the banner it prints on every call is noise here.
  TEST_DB_PORT="${PORT}" podman compose -f "${file}" "$@" 2>&1 \
    | grep -v 'Executing external compose provider' || true
}

case "${CMD}" in
  up)
    FILE="$(compose_file "${2:-}")"
    NAME="$(container_name "${FILE}")"
    DB="$(db_name "${FILE}")"
    compose "${FILE}" up -d
    # Wait on the container's own healthcheck rather than sleeping. An
    # initdb that has not finished accepts no connections, and a suite
    # started too early fails in a way that looks like a code defect.
    printf 'test-db: waiting for %s' "${NAME}"
    for _ in $(seq 1 60); do
      state="$(podman inspect "${NAME}" --format '{{.State.Health.Status}}' 2>/dev/null || echo unknown)"
      case "${state}" in
        healthy) echo; break ;;
        unhealthy) echo; die "${NAME} is unhealthy — try: scripts/test-db.sh logs ${2}" ;;
        *) printf '.'; sleep 1 ;;
      esac
    done
    [[ "${state:-}" == "healthy" ]] || die "${NAME} did not become healthy in 60s"
    # Healthy only means the server inside the container answers. Check
    # from *outside* too, because the published port is not guaranteed to
    # reach it: on macOS podman publishes on IPv6 `*` while another
    # Postgres may already hold IPv4 127.0.0.1 on the same port, and
    # `localhost` resolves to the latter first. That misdirection is
    # otherwise invisible — you get a real server answering with
    # "database does not exist", which reads like a broken container.
    if command -v psql >/dev/null 2>&1; then
      if ! PGPASSWORD=loco psql -h localhost -p "${PORT}" -U loco -d "${DB}" \
             -tAc 'select 1' >/dev/null 2>&1; then
        echo "test-db: ${NAME} is healthy, but localhost:${PORT} does not reach it." >&2
        echo "         Something else is probably listening there. Check with:" >&2
        echo "           lsof -nP -iTCP:${PORT} -sTCP:LISTEN" >&2
        echo "         then retry on a free port:" >&2
        echo "           TEST_DB_PORT=<free-port> scripts/test-db.sh up ${2%/}" >&2
        exit 1
      fi
    fi
    echo "test-db: ${NAME} ready"
    echo "  DATABASE_URL=postgres://loco:loco@localhost:${PORT}/${DB}"
    echo "  next: scripts/ci-check.sh test-db ${2%/}"
    ;;

  down)
    FILE="$(compose_file "${2:-}")"
    # `-v` because the point of this database is that nothing survives it.
    compose "${FILE}" down -v
    echo "test-db: $(container_name "${FILE}") removed"
    ;;

  psql)
    FILE="$(compose_file "${2:-}")"
    exec podman exec -it "$(container_name "${FILE}")" \
      psql -U loco -d "$(db_name "${FILE}")"
    ;;

  logs)
    FILE="$(compose_file "${2:-}")"
    exec podman logs -f "$(container_name "${FILE}")"
    ;;

  url)
    FILE="$(compose_file "${2:-}")"
    printf 'postgres://loco:loco@localhost:%s/%s\n' "${PORT}" "$(db_name "${FILE}")"
    ;;

  status)
    podman ps --filter "name=^mxi-.*-test-db$" \
      --format 'table {{.Names}}\t{{.Status}}\t{{.Ports}}'
    ;;

  down-all)
    # Stop by name pattern, so a container whose compose file has since
    # been edited (or deleted) is still cleanable.
    names="$(podman ps -aq --filter 'name=^mxi-.*-test-db$')"
    if [[ -z "${names}" ]]; then
      echo "test-db: nothing running"
    else
      # shellcheck disable=SC2086 # deliberate word splitting: one id per arg
      podman rm -f ${names} >/dev/null
      echo "test-db: removed $(printf '%s\n' ${names} | wc -l | tr -d ' ') container(s)"
    fi
    ;;

  *)
    die "unknown command: ${CMD}"
    ;;
esac
