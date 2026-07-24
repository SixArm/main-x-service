# Pillar 4 — Talent management & development

## Performance reviews

Review **cycles** (period + status open/calibrating/closed) contain
one review per employee: goals (title, due, status), continuous
**feedback entries** (attachable any time, not just in cycles), and
the appraisal (`draft → submitted → calibrated → shared`) with a 1–5
rating. Calibration happens at cycle level (HR flips submitted
reviews to calibrated after moderation); `shared` releases the
review to the employee's self-service view. Review content is
sensitive — reads are audited and ABAC-scoped (self + manager + HR).

## Learning (LMS via the course registry)

HCM deliberately does **not** host courses — the family's
[course-service](../../course/course-service-with-loco/) owns course
identity and offerings. HCM owns **TrainingEnrollments**: employee ×
`course:` / `courseinstance:` URN, status (`enrolled → completed |
failed | withdrawn`), completion date, and an optional certification
expiry. The compliance view lists employees with missing mandatory
trainings or **expiring certifications** (next 90 days) — the
strategic reason enrollments live here. Course names are resolved
best-effort (the display-name client pattern), never copied as truth.

## Succession planning

Succession plans name a key **position** (title + department +
criticality 1–5) and its incumbent, then rank a pipeline of
candidate employees by **readiness** (`ready_now` / `ready_1y` /
`ready_2y`) with development notes. The dashboard surfaces uncovered
critical positions (criticality ≥ 4 with no `ready_now` candidate) —
succession as a measurable gap list, not a slide deck. Succession
data is HR-persona-only under ABAC.
