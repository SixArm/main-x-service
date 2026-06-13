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

  // Seed the form once from `initial` (read without tracking).
  const seed = untrack(() => initial);

  let title = $state(seed.title ?? "");
  let caseNumber = $state(seed.case_number ?? "");
  let agencyId = $state(seed.agency_id ?? "");
  let agencyName = $state(seed.agency_name ?? "");
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
  let alternateTitles = $state((seed.alternate_titles ?? []).join(", "));
  let subjects = $state((seed.subjects ?? []).join(", "));
  let keywords = $state((seed.keywords ?? []).join(", "));
  let sameAs = $state((seed.same_as ?? []).join(", "));
  let inLanguage = $state((seed.in_language ?? []).join(", "));
  let identifiers = $state<CaseIdentifier[]>(
    (seed.identifiers ?? []).filter((i) => typeof i.scheme === "string"),
  );

  let submitting = $state(false);
  let error = $state<string | null>(null);

  function splitList(s: string): string[] {
    return s
      .split(",")
      .map((x) => x.trim())
      .filter((x) => x.length > 0);
  }
  function blankToNull(s: string): string | null {
    const t = s.trim();
    return t.length > 0 ? t : null;
  }

  function addIdentifier() {
    identifiers = [...identifiers, { scheme: "Docket", value: "" }];
  }
  function removeIdentifier(i: number) {
    identifiers = identifiers.filter((_, idx) => idx !== i);
  }

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
