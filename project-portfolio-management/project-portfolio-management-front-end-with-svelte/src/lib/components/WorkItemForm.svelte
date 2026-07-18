<!--
  WorkItemForm — the shared create/edit form for a WorkItem.

  Purpose:
    A single controlled form bound to local `$state`, used by both
    `[collection]/new` (create) and `[collection]/[pid]/edit` (edit). It
    keeps array/optional fields as flat editable strings and assembles a
    clean `WorkItem` on submit, delegating the API call to its parent.

  $props:
    - kind       : WorkItemKind — fixed by the collection (shown read-only).
    - initial    : WorkItem — seed values.
    - submitLabel: string — button text.
    - onsubmit   : (record: WorkItem) => Promise<void> — parent persists;
                   errors thrown here surface in the inline banner.
-->
<script lang="ts">
  import { untrack } from "svelte";
  import { ALL_SCHEMES, ALL_STATUSES, isChildKind } from "$lib/api/types";
  import type {
    IdentifierScheme,
    WorkItem,
    WorkItemIdentifier,
    WorkItemKind,
    WorkItemStatus,
  } from "$lib/api/types";
  import { t } from "$lib/i18n.svelte";

  let {
    kind,
    initial,
    submitLabel,
    onsubmit,
  }: {
    kind: WorkItemKind;
    initial: WorkItem;
    submitLabel?: string;
    onsubmit: (record: WorkItem) => Promise<void>;
  } = $props();

  // Seed once from `initial` (untracked) so parent re-renders don't clobber
  // in-progress edits.
  const seed = untrack(() => initial);

  let name = $state(seed.name ?? "");
  let code = $state(seed.code ?? "");
  let ownerOrgId = $state(seed.owner_org_id ?? "");
  let ownerOrgName = $state(seed.owner_org_name ?? "");
  let portfolioRef = $state(seed.portfolio_ref ?? "");
  let status = $state<WorkItemStatus | "">(
    typeof seed.status === "string" ? seed.status : "",
  );
  let alternateNames = $state((seed.alternate_names ?? []).join(", "));
  let keywords = $state((seed.keywords ?? []).join(", "));
  let tags = $state((seed.tags ?? []).join(", "));
  let sameAs = $state((seed.same_as ?? []).join(", "));
  let inLanguage = $state(seed.in_language ?? "");
  let identifiers = $state<WorkItemIdentifier[]>(
    (seed.identifiers ?? []).filter((i) => typeof i.scheme === "string"),
  );

  let submitting = $state(false);
  let error = $state<string | null>(null);

  const childKind = $derived(isChildKind(kind));

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

  // Assemble the wire `WorkItem` from the flat form state.
  function build(): WorkItem {
    const record: WorkItem = { kind, name: name.trim() };
    record.code = blankToNull(code);
    record.owner_org_id = blankToNull(ownerOrgId);
    record.owner_org_name = blankToNull(ownerOrgName);
    record.portfolio_ref = childKind ? blankToNull(portfolioRef) : null;
    record.status = status === "" ? null : (status as WorkItemStatus);
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
  <p class="small muted" data-testid="form-kind">{kind}</p>
  <label>{t("form.title")}<input type="text" bind:value={name} required /></label>
  <div class="row">
    <label
      >{t("form.caseNumber")}<input
        type="text"
        bind:value={code}
        placeholder="PROJ-2026"
      /></label
    >
    <label
      >{t("form.status")}
      <select bind:value={status}>
        <option value="">{t("form.empty")}</option>
        {#each ALL_STATUSES as s (String(s))}
          <option value={s as WorkItemStatus}>{s}</option>
        {/each}
      </select>
    </label>
  </div>
  <div class="row">
    <label>{t("form.agencyId")}<input type="text" bind:value={ownerOrgId} placeholder="organization:…" /></label>
    <label>{t("form.agencyName")}<input type="text" bind:value={ownerOrgName} /></label>
  </div>
  {#if childKind}
    <label>{t("form.openedDate")}<input type="text" bind:value={portfolioRef} placeholder="portfolio pid (UUID)" /></label>
  {/if}
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
