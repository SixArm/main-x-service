<!--
  PlanForm — the shared create/edit form for a Plan.

  Purpose:
    A single controlled form bound to local `$state`, used by both
    `plans/new` (create) and `plans/[pid]/edit` (edit). It keeps
    array/optional fields as flat editable strings and assembles a clean
    `Plan` on submit, delegating the API call to its parent.

  $props:
    - initial    : Plan — seed values.
    - submitLabel: string — button text.
    - onsubmit   : (record: Plan) => Promise<void> — parent persists;
                   errors thrown here surface in the inline banner.
-->
<script lang="ts">
  import { untrack } from "svelte";
  import { ALL_KINDS, ALL_SCHEMES, ALL_STATUSES } from "$lib/api/types";
  import type {
    IdentifierScheme,
    Plan,
    PlanIdentifier,
    PlanKind,
    PlanStatus,
  } from "$lib/api/types";
  import { t } from "$lib/i18n.svelte";

  let {
    initial,
    submitLabel,
    onsubmit,
  }: {
    initial: Plan;
    submitLabel?: string;
    onsubmit: (record: Plan) => Promise<void>;
  } = $props();

  // Seed once from `initial` (untracked) so parent re-renders don't clobber
  // in-progress edits.
  const seed = untrack(() => initial);

  let name = $state(seed.name ?? "");
  // `kind` is an optional descriptive label ("" = none).
  let kind = $state<PlanKind | "">(seed.kind ?? "");
  let code = $state(seed.code ?? "");
  let ownerOrgId = $state(seed.owner_org_id ?? "");
  let ownerOrgName = $state(seed.owner_org_name ?? "");
  let parentRef = $state(seed.parent_ref ?? "");
  let status = $state<PlanStatus | "">(
    typeof seed.status === "string" ? seed.status : "",
  );
  let alternateNames = $state((seed.alternate_names ?? []).join(", "));
  let keywords = $state((seed.keywords ?? []).join(", "));
  let tags = $state((seed.tags ?? []).join(", "));
  let sameAs = $state((seed.same_as ?? []).join(", "));
  let inLanguage = $state(seed.in_language ?? "");
  let identifiers = $state<PlanIdentifier[]>(
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
    const trimmed = s.trim();
    return trimmed.length > 0 ? trimmed : null;
  }

  function addIdentifier() {
    identifiers = [...identifiers, { scheme: "JiraProjectKey", value: "" }];
  }
  function removeIdentifier(i: number) {
    identifiers = identifiers.filter((_, idx) => idx !== i);
  }

  // Assemble the wire `Plan` from the flat form state.
  function build(): Plan {
    const record: Plan = { name: name.trim() };
    record.kind = kind === "" ? null : (kind as PlanKind);
    record.code = blankToNull(code);
    record.owner_org_id = blankToNull(ownerOrgId);
    record.owner_org_name = blankToNull(ownerOrgName);
    record.parent_ref = blankToNull(parentRef);
    record.status = status === "" ? null : (status as PlanStatus);
    record.in_language = blankToNull(inLanguage);
    record.alternate_names = splitList(alternateNames);
    record.keywords = splitList(keywords);
    record.tags = splitList(tags);
    record.same_as = splitList(sameAs);
    record.identifiers = identifiers
      .filter((i) => i.value.trim().length > 0)
      .map((i) => ({ scheme: i.scheme, value: i.value.trim() }));
    return record;
  }

  async function handleSubmit(event: SubmitEvent) {
    event.preventDefault();
    error = null;
    if (name.trim().length === 0) {
      error = t("form.titleRequired");
      return;
    }
    submitting = true;
    try {
      await onsubmit(build());
    } catch (err) {
      error = err instanceof Error ? err.message : t("form.saveFailed");
    } finally {
      submitting = false;
    }
  }
</script>

<form class="stack" onsubmit={handleSubmit}>
  <label>{t("form.title")}<input type="text" bind:value={name} required /></label>
  <div class="row">
    <label
      >{t("form.caseType")}
      <select bind:value={kind} data-testid="form-kind">
        <option value="">{t("form.empty")}</option>
        {#each ALL_KINDS as k (k)}
          <option value={k}>{k}</option>
        {/each}
      </select>
    </label>
    <label
      >{t("form.status")}
      <select bind:value={status}>
        <option value="">{t("form.empty")}</option>
        {#each ALL_STATUSES as s (String(s))}
          <option value={s as PlanStatus}>{s}</option>
        {/each}
      </select>
    </label>
  </div>
  <div class="row">
    <label
      >{t("form.caseNumber")}<input
        type="text"
        bind:value={code}
        placeholder="PROJ-2026"
      /></label
    >
    <label>{t("form.openedDate")}<input type="text" bind:value={parentRef} placeholder="parent plan pid (UUID)" /></label>
  </div>
  <div class="row">
    <label>{t("form.agencyId")}<input type="text" bind:value={ownerOrgId} placeholder="organization:…" /></label>
    <label>{t("form.agencyName")}<input type="text" bind:value={ownerOrgName} /></label>
  </div>
  <label
    >{t("form.alternateTitles")} <small>{t("form.commaSeparated")}</small><input
      type="text"
      bind:value={alternateNames}
    /></label
  >
  <label
    >{t("form.keywords")} <small>{t("form.commaSeparated")}</small><input
      type="text"
      bind:value={keywords}
    /></label
  >
  <label
    >{t("form.subjects")} <small>{t("form.commaSeparated")}</small><input
      type="text"
      bind:value={tags}
    /></label
  >
  <label
    >{t("form.sameAs")} <small>{t("form.commaSeparated")}</small><input
      type="text"
      bind:value={sameAs}
    /></label
  >
  <label
    >{t("form.languages")} <small>{t("form.commaSeparatedIso")}</small><input
      type="text"
      bind:value={inLanguage}
    /></label
  >

  <fieldset class="stack">
    <legend>{t("form.identifiers")}</legend>
    {#each identifiers as identifier, i (i)}
      <div class="row">
        <select bind:value={identifier.scheme}>
          {#each ALL_SCHEMES as scheme (String(scheme))}
            <option value={scheme as IdentifierScheme}>{scheme}</option>
          {/each}
        </select>
        <input type="text" bind:value={identifier.value} placeholder={t("form.valuePlaceholder")} />
        <button type="button" onclick={() => removeIdentifier(i)}>{t("form.remove")}</button>
      </div>
    {/each}
    <button type="button" onclick={addIdentifier}>{t("form.addIdentifier")}</button>
  </fieldset>

  <button class="button" type="submit" disabled={submitting}>
    {submitting ? t("form.saving") : (submitLabel ?? t("form.save"))}
  </button>
  {#if error}<p class="banner" role="alert">{error}</p>{/if}
</form>
