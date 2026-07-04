// Pure validation + wire-normalisation helpers for CourseForm,
// extracted from the component so they are unit-testable without a DOM
// mount (spec §6 FR-4, §11). The component imports both; behaviour is a
// pure function of the input Course.
import type { Course } from "$lib/api/types.js";
import type { FieldErrors } from "$lib/forms/form.svelte.js";

/**
 * Client-side mirror of the Course Service's required / format / range
 * rules (spec §6 FR-4), so obvious errors are caught before the HTTP
 * round-trip. Returns a field→message map; an empty map means valid.
 *
 * Rules: `name` required; `url` / `additional_type` / `license` must be
 * fully-qualified http(s) URLs when present (service FR-25);
 * `course_code` ≤ 100 chars; `number_of_credits` ≥ 0.
 */
export function validateCourse(value: Course): FieldErrors {
  const errors: FieldErrors = {};
  if (!value.name.trim()) errors.name = "Required";
  const urlFields: [keyof Course, string][] = [
    ["url", "URL"],
    ["additional_type", "Additional type"],
    ["license", "License URL"],
  ];
  for (const [field, label] of urlFields) {
    const v = value[field];
    if (typeof v === "string" && v.length > 0 && !/^https?:\/\//i.test(v)) {
      errors[field as string] = `${label} must start with http(s)://`;
    }
  }
  if (typeof value.course_code === "string" && value.course_code.length > 100) {
    errors.course_code = "Max 100 chars";
  }
  if (
    typeof value.number_of_credits === "number" &&
    value.number_of_credits < 0
  ) {
    errors.number_of_credits = "Must be ≥ 0";
  }
  return errors;
}

/**
 * Strip blank string fields so the wire shape doesn't ship `url: ""`
 * and trip the service's FR-25 scheme check (empty strings fail
 * `"".startsWith("http://")` → 422). Empty strings on optional fields
 * are a form-UI artefact, not a real value; convert them to `undefined`
 * before submit so the omitted-key branch of the service's serde
 * default fires. Identifier `url` / `name` follow the same rule.
 */
export function normalizeForWire(c: Course): Course {
  const blankToUndef = <T>(v: T): T | undefined =>
    typeof v === "string" && v.trim() === "" ? undefined : v;
  return {
    ...c,
    description: blankToUndef(c.description),
    disambiguating_description: blankToUndef(c.disambiguating_description),
    url: blankToUndef(c.url),
    license: blankToUndef(c.license),
    additional_type: blankToUndef(c.additional_type),
    course_code: blankToUndef(c.course_code),
    typical_age_range: blankToUndef(c.typical_age_range),
    time_required: blankToUndef(c.time_required),
    version: blankToUndef(c.version),
    audience: blankToUndef(c.audience),
    educational_use: blankToUndef(c.educational_use),
    identifiers: c.identifiers?.map((i) => ({
      ...i,
      url: blankToUndef(i.url),
      name: blankToUndef(i.name),
    })),
  };
}
