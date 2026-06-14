<!--
  CaseForm — the shared create/edit form for a Case.

  Purpose:
    A single controlled form bound to local `$state`, used by both
    `new/+page.svelte` (create) and `[pid]/edit/+page.svelte` (edit). It
    keeps array/optional fields as flat editable strings and assembles a
    clean `Case` on submit, delegating the actual API call to its parent.

  $props:
    - initial    : Case   — seed values (empty `{title:""}` for create).
    - submitLabel: string — button text (default "Save").
    - onsubmit   : (record: Case) => Promise<void> — callback the parent
                   supplies to persist; errors thrown here are caught and
                   shown in the inline banner.

  Local $state: every editable field (title, caseNumber, …), the
    comma-joined list fields, the identifiers array, plus `submitting`
    and `error` UI flags. No `$derived` — the `Case` is built imperatively
    in `build()` on submit.
-->
<script lang="ts">
  import { untrack } from "svelte";
  import {
    ALL_CASE_TYPES,
    ALL_PRIORITIES,
    ALL_SCHEMES,
    ALL_STATUSES,
  } from "$lib/api/types";
  import type {
    Case,
    CaseIdentifier,
    CaseStatus,
    CaseType,
    IdentifierScheme,
    Priority,
  } from "$lib/api/types";

  let {
    initial,
    submitLabel = "Save",
    onsubmit,
  }: {
    initial: Case;
    submitLabel?: string;
    onsubmit: (record: Case) => Promise<void>;
  } = $props();

  // Seed the form once from `initial` (read without tracking) so later
  // parent re-renders of `initial` don't clobber in-progress edits.
  const seed = untrack(() => initial);

  // Scalar fields mirror the record's optional values as plain strings
  // (empty string standing in for null/absent).
  let title = $state(seed.title ?? "");
  let caseNumber = $state(seed.case_number ?? "");
  let agencyId = $state(seed.agency_id ?? "");
  let agencyName = $state(seed.agency_name ?? "");
  // Enum selects use "" for the unselected ("—") option. `Custom` variants
  // are objects, so only seed the select when the value is a bare string.
  let caseType = $state<CaseType | "">(
    typeof seed.case_type === "string" ? seed.case_type : "",
  );
  let status = $state<CaseStatus | "">(
    typeof seed.status === "string" ? seed.status : "",
  );
  let priority = $state<Priority | "">(
    typeof seed.priority === "string" ? seed.priority : "",
  );
  let openedDate = $state(seed.opened_date ?? "");
  // List fields are edited as a single comma-separated text input; joined
  // here, split back in `splitList` on submit.
  let alternateTitles = $state((seed.alternate_titles ?? []).join(", "));
  let subjects = $state((seed.subjects ?? []).join(", "));
  let keywords = $state((seed.keywords ?? []).join(", "));
  let sameAs = $state((seed.same_as ?? []).join(", "));
  let inLanguage = $state((seed.in_language ?? []).join(", "));
  // Identifiers stay structured; drop any seeded `Custom`-scheme rows since
  // the scheme `<select>` only offers the unit schemes.
  let identifiers = $state<CaseIdentifier[]>(
    (seed.identifiers ?? []).filter((i) => typeof i.scheme === "string"),
  );

  // UI flags: `submitting` disables the button; `error` drives the banner.
  let submitting = $state(false);
  let error = $state<string | null>(null);

  // Split a comma-separated input into trimmed, non-empty tokens.
  function splitList(s: string): string[] {
    return s
      .split(",")
      .map((x) => x.trim())
      .filter((x) => x.length > 0);
  }
  // Collapse a blank/whitespace input to `null` (the wire shape for absent).
  function blankToNull(s: string): string | null {
    const t = s.trim();
    return t.length > 0 ? t : null;
  }

  // Append a fresh identifier row (immutable reassignment so `$state` reacts).
  function addIdentifier() {
    identifiers = [...identifiers, { scheme: "Docket", value: "" }];
  }
  // Drop the identifier row at index `i`.
  function removeIdentifier(i: number) {
    identifiers = identifiers.filter((_, idx) => idx !== i);
  }

  // Assemble the wire `Case` from the flat form state: trim title, collapse
  // blanks to null, split list fields, map "" enum back to null, and drop
  // empty identifier rows.
  function build(): Case {
    const record: Case = { title: title.trim() };
    record.case_number = blankToNull(caseNumber);
    record.agency_id = blankToNull(agencyId);
    record.agency_name = blankToNull(agencyName);
    record.case_type = caseType === "" ? null : (caseType as CaseType);
    record.status = status === "" ? null : (status as CaseStatus);
    record.priority = priority === "" ? null : (priority as Priority);
    record.opened_date = blankToNull(openedDate);
    record.alternate_titles = splitList(alternateTitles);
    record.subjects = splitList(subjects);
    record.keywords = splitList(keywords);
    record.same_as = splitList(sameAs);
    record.in_language = splitList(inLanguage);
    record.identifiers = identifiers
      .filter((i) => i.value.trim().length > 0)
      .map((i) => ({ scheme: i.scheme, value: i.value.trim() }));
    return record;
  }

  // Submit handler: block the native navigation, client-validate the
  // required title, then hand the built record to the parent's `onsubmit`,
  // surfacing any thrown error in the banner.
  async function handleSubmit(event: SubmitEvent) {
    event.preventDefault();
    error = null;
    if (title.trim().length === 0) {
      error = "Title is required.";
      return;
    }
    submitting = true;
    try {
      await onsubmit(build());
    } catch (err) {
      error = err instanceof Error ? err.message : "Save failed";
    } finally {
      submitting = false;
    }
  }
</script>

<!-- Controlled form: each field two-way binds to a `$state` variable above;
     `onsubmit` is intercepted by `handleSubmit`. -->


<form class="stack" onsubmit={handleSubmit}>
  <label>Title<input type="text" bind:value={title} required /></label>
  <div class="row">
    <label
      >Case type
      <select bind:value={caseType}>
        <option value="">—</option>
        {#each ALL_CASE_TYPES as type (String(type))}
          <option value={type as CaseType}>{type}</option>
        {/each}
      </select>
    </label>
    <label
      >Status
      <select bind:value={status}>
        <option value="">—</option>
        {#each ALL_STATUSES as s (String(s))}
          <option value={s as CaseStatus}>{s}</option>
        {/each}
      </select>
    </label>
    <label
      >Priority
      <select bind:value={priority}>
        <option value="">—</option>
        {#each ALL_PRIORITIES as p (p)}
          <option value={p}>{p}</option>
        {/each}
      </select>
    </label>
  </div>
  <div class="row">
    <label
      >Case number<input
        type="text"
        bind:value={caseNumber}
        placeholder="2026-HB-0042"
      /></label
    >
    <label>Opened date<input type="date" bind:value={openedDate} /></label>
  </div>
  <div class="row">
    <label>Agency id<input type="text" bind:value={agencyId} /></label>
    <label>Agency name<input type="text" bind:value={agencyName} /></label>
  </div>
  <label
    >Alternate titles <small>(comma-separated)</small><input
      type="text"
      bind:value={alternateTitles}
    /></label
  >
  <label
    >Subjects <small>(comma-separated)</small><input
      type="text"
      bind:value={subjects}
    /></label
  >
  <label
    >Keywords <small>(comma-separated)</small><input
      type="text"
      bind:value={keywords}
    /></label
  >
  <label
    >Same-as URLs <small>(comma-separated)</small><input
      type="text"
      bind:value={sameAs}
    /></label
  >
  <label
    >Languages <small>(comma-separated ISO 639-1)</small><input
      type="text"
      bind:value={inLanguage}
    /></label
  >

  <fieldset class="stack">
    <legend>Identifiers</legend>
    {#each identifiers as identifier, i (i)}
      <div class="row">
        <select bind:value={identifier.scheme}>
          {#each ALL_SCHEMES as scheme (String(scheme))}
            <option value={scheme as IdentifierScheme}>{scheme}</option>
          {/each}
        </select>
        <input type="text" bind:value={identifier.value} placeholder="value" />
        <button type="button" onclick={() => removeIdentifier(i)}>Remove</button
        >
      </div>
    {/each}
    <button type="button" onclick={addIdentifier}>+ Add identifier</button>
  </fieldset>

  <button class="button" type="submit" disabled={submitting}>
    {submitting ? "Saving…" : submitLabel}
  </button>
  {#if error}<p class="banner" role="alert">{error}</p>{/if}
</form>
