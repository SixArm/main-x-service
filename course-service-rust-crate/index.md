# Course Service — index

Navigation aid + worked examples. The behavioural source of truth is
[`spec.md`](spec.md); deep references live in [`AGENTS/`](AGENTS/).

## Top-level documents

| Document | Purpose |
|---|---|
| [spec.md](spec.md) | Single source of truth (§1–§18; live work queue in §13) |
| [README.md](README.md) | User-facing intro, quick start, env vars |
| [CLAUDE.md](CLAUDE.md) | Re-export of `AGENTS.md` for Claude Code session bootstrap |
| [AGENTS.md](AGENTS.md) | Agent guide |
| [CHANGELOG.md](CHANGELOG.md) | Keep-a-Changelog history |

## AGENTS/ (per-area detail)

| Document | Purpose |
|---|---|
| [AGENTS/index.md](AGENTS/index.md) | This directory's index |
| [AGENTS/spec-driven-development.md](AGENTS/spec-driven-development.md) | SDD discipline |
| [AGENTS/models.md](AGENTS/models.md) | Domain model reference |
| [AGENTS/matching.md](AGENTS/matching.md) | Matching algorithm reference |
| [AGENTS/restful.md](AGENTS/restful.md) | REST API + library API reference |
| [AGENTS/testing.md](AGENTS/testing.md) | Testing strategy |

## Worked examples

### Create a course

```bash
curl -X POST http://localhost:8080/api/courses \
  -H "content-type: application/json" \
  -d '{
    "name": "Introduction to Computer Science",
    "course_code": "CS101",
    "number_of_credits": 4,
    "educational_level": "Undergraduate",
    "teaches": [
      "computational thinking",
      "abstraction",
      "basic algorithms"
    ],
    "keywords": ["computer science", "programming", "algorithms"],
    "available_language": ["en"],
    "is_accessible_for_free": false
  }'
```

### Check for duplicates without writing

```bash
curl -X POST http://localhost:8080/api/courses/check-duplicates \
  -H "content-type: application/json" \
  -d '{
    "name": "Intro to CS",
    "course_code": "CS101",
    "provider_id": "00000000-0000-0000-0000-000000000001"
  }'
```

### Add an instance

```bash
curl -X POST http://localhost:8080/api/courses/{course_id}/instances \
  -H "content-type: application/json" \
  -d '{
    "course_id": "{course_id}",
    "name": "CS101 — Fall 2026",
    "course_mode": "blended",
    "status": "scheduled",
    "schedule": {
      "start_date": "2026-09-01T09:00:00Z",
      "end_date":   "2026-12-15T17:00:00Z",
      "time_zone":  "America/Los_Angeles",
      "recurrence": "FREQ=WEEKLY;BYDAY=TU,TH;BYHOUR=9"
    },
    "maximum_attendee_capacity": 200
  }'
```

### Match a candidate

```bash
curl -X POST http://localhost:8080/api/courses/match \
  -H "content-type: application/json" \
  -d '{
    "name": "Intro Comp Sci",
    "course_code": "CS101",
    "educational_level": "Undergraduate",
    "threshold": 0.7
  }'
```

### Merge confirmed duplicates

```bash
curl -X POST http://localhost:8080/api/courses/merge \
  -H "content-type: application/json" \
  -d '{
    "main_course_id":      "uuid-main",
    "duplicate_course_id": "uuid-dup",
    "merge_reason":        "Same course code, same provider, identical syllabus"
  }'
```
