<!--
  Entry detail — the page this application exists for.

  Five properties it is built around, all from `../../../spec/`:

  1. **Live versus draft is never ambiguous.** Every locale row says
     which revision readers see and whether the draft has moved past
     it. "Save" and "go live" are different verbs.
  2. **A stale save is a comparison, not a retry.** Saving with an
     out-of-date `base_revision_pid` returns `409`; this page renders
     the competing revision and makes the author choose, because a
     retry button would silently discard someone's work.
  3. **The publish gate explains itself.** Blockers arrive with the
     rule and the remedy, and both are shown — a refusal an author
     cannot act on is just a locked door.
  4. **Restore writes a new revision.** The UI says so before doing
     it, because "restore" reads like undo and it is not.
  5. **Preview is a server round trip.** The token never reaches this
     page (`../../../spec/auth.md`).
-->
<script lang="ts">
  import { page } from "$app/state";
  import { t } from "$lib/i18n.svelte";
  import * as cms from "$lib/api/cms";
  import { ApiError } from "$lib/api/client";
  import { actor, staleness, when } from "$lib/format";
  import BlockEditor from "$lib/components/BlockEditor.svelte";
  import StateBadge from "$lib/components/StateBadge.svelte";
  import type {
    Block,
    Diff,
    Entry,
    LocaleRow,
    Preview,
    PublishCheck,
    Revision,
    RevisionSummary,
    Variant,
  } from "$lib/api/cms";

  const pid = $derived(page.params.pid ?? "");

  let entry = $state<Entry | null>(null);
  let variants = $state<Variant[]>([]);
  let locales = $state<LocaleRow[]>([]);
  let locale = $state<string | null>(null);
  let history = $state<RevisionSummary[]>([]);
  let editing = $state<Revision | null>(null);
  let blocks = $state<Block[]>([]);
  let title = $state("");
  let gate = $state<PublishCheck | null>(null);
  let comparison = $state<Diff | null>(null);
  let previewed = $state<Preview | null>(null);
  let action = $state("publish");
  let reason = $state("");
  let notice = $state<string | null>(null);
  let failure = $state<string | null>(null);
  /** Set when a save lost a race: the revision that won. */
  let conflict = $state<RevisionSummary | null>(null);

  /** Transitions the service refuses without a reason. */
  const NEEDS_REASON = ["reject", "unpublish", "archive", "restore"];

  const variant = $derived(variants.find((v) => v.locale === locale) ?? null);
  const siteKey = $derived(page.url.searchParams.get("site") ?? "");
  /** Has the draft moved past what readers see? */
  const ahead = $derived(
    variant !== null &&
      variant.published_revision_pid !== null &&
      variant.current_revision_pid !== variant.published_revision_pid,
  );

  async function load() {
    failure = null;
    try {
      const [detail, matrix] = await Promise.all([
        cms.getEntry(pid),
        cms.entryTranslations(pid),
      ]);
      entry = detail.entry;
      variants = detail.variants;
      locales = matrix.locales;
      locale ??= detail.entry.source_locale;
      await loadLocale();
    } catch (error: unknown) {
      failure = error instanceof Error ? error.message : String(error);
    }
  }

  async function loadLocale() {
    if (!locale) return;
    conflict = null;
    previewed = null;
    comparison = null;
    const [rows, check] = await Promise.all([
      cms.listRevisions(pid, locale),
      cms.publishCheck(pid, locale),
    ]);
    history = rows;
    gate = check;
    const current = rows.find((r) => r.is_current) ?? rows[0];
    editing = current ? await cms.getRevision(current.pid) : null;
    blocks = editing ? [...editing.blocks] : [];
    title = editing?.title ?? "";
  }

  $effect(() => {
    if (pid) void load();
  });

  async function save() {
    if (!locale || !editing) return;
    notice = null;
    conflict = null;
    try {
      await cms.createRevision(pid, locale, {
        base_revision_pid: editing.pid,
        title,
        blocks,
      });
      notice = t("common.saved");
      await load();
    } catch (error: unknown) {
      if (error instanceof ApiError && error.status === 409) {
        // Do not retry, and do not discard: show what won the race and
        // let the author compare before deciding.
        const rows = await cms.listRevisions(pid, locale);
        conflict = rows.find((r) => r.is_current) ?? null;
        if (conflict && editing) {
          comparison = await cms.diff(editing.pid, conflict.pid);
        }
      } else {
        failure = error instanceof Error ? error.message : String(error);
      }
    }
  }

  async function transition() {
    if (!locale) return;
    notice = null;
    try {
      await cms.transition(pid, locale, action, reason || undefined);
      reason = "";
      await load();
    } catch (error: unknown) {
      failure = error instanceof Error ? error.message : String(error);
    }
  }

  async function restore(revision: string) {
    if (!locale || !reason) {
      failure = t("workflow.reasonRequired");
      return;
    }
    await cms.restore(pid, locale, revision, reason);
    reason = "";
    await load();
  }

  async function compare(from: string, to: string) {
    comparison = await cms.diff(from, to);
  }

  async function openPreview() {
    if (!locale || !siteKey) return;
    previewed = await cms.preview(pid, locale, siteKey, editing?.pid);
  }
</script>

<svelte:head><title>{entry?.key ?? t("nav.entries")}</title></svelte:head>

{#if failure}
  <div class="panel error">{t("common.error")}: {failure}</div>
{/if}

{#if !entry}
  <div class="panel">{t("common.loading")}</div>
{:else}
  <h1>{entry.key}</h1>
  <p class="muted">{entry.content_type_key} · {actor(entry.owner_ref)}</p>

  <!-- The locale matrix: status, what is live, and how far behind a
       translation has fallen — with the count, never a bare badge. -->
  <section class="panel">
    <h2>{t("entry.locales")}</h2>
    <table>
      <thead>
        <tr>
          <th>{t("common.locale")}</th>
          <th>{t("common.status")}</th>
          <th>{t("entry.liveRevision")}</th>
          <th>{t("translations.stale")}</th>
        </tr>
      </thead>
      <tbody>
        {#each locales as row (row.locale)}
          {@const state = staleness(row.staleness)}
          <tr class:selected={row.locale === locale}>
            <td>
              <button type="button" class="link" onclick={() => { locale = row.locale; void loadLocale(); }}>
                {row.locale}{row.is_source ? " ★" : ""}
              </button>
            </td>
            <td><StateBadge status={row.status} /></td>
            <td>{row.published ? "✓" : "—"}</td>
            <td class:ahead={state.tone === "stale"} class:no-data={state.tone === "unknown"}>
              {state.text}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </section>

  {#if variant}
    {#if ahead}
      <p class="ahead">{t("entry.draftAhead")}</p>
    {/if}

    <!-- The publish gate, with the rule and the remedy for each
         blocker. A refusal an author cannot act on is a locked door. -->
    {#if gate}
      <section class="panel">
        <h2>{gate.ready ? t("workflow.ready") : t("workflow.blockers")}</h2>
        {#if gate.blockers.length > 0}
          <table>
            <thead>
              <tr><th>{t("insights.rule")}</th><th>{t("common.title")}</th><th>{t("workflow.remedy")}</th></tr>
            </thead>
            <tbody>
              <!-- Keyed by position: a blocker has no id, and one page can
                     carry two blockers with the same rule and subject. -->
                {#each gate.blockers as blocker, index (index)}
                <tr>
                  <td><span class="rule">{blocker.rule}</span></td>
                  <td>{blocker.subject}</td>
                  <td>{blocker.remedy}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        {/if}
      </section>
    {/if}

    {#if conflict}
      <!-- A lost race. Never a retry button: that would discard
           whatever the other author wrote. -->
      <section class="panel error">
        <h2>{t("entry.conflict")}</h2>
        <p>{t("entry.conflictHelp")}</p>
        <p>
          #{conflict.number} · {actor(conflict.author_ref)} · {when(conflict.created_at)}
        </p>
        {#if comparison}
          <p>
            {comparison.diff.identical
              ? t("entry.identical")
              : comparison.diff.block_comparison}
          </p>
        {/if}
      </section>
    {/if}

    <label class="panel">
      {t("common.title")}
      <input bind:value={title} />
    </label>

    <BlockEditor bind:blocks />

    <section class="panel row">
      <button class="primary" type="button" onclick={save}>{t("entry.save")}</button>
      {#if siteKey}
        <button type="button" onclick={openPreview}>{t("preview.open")}</button>
      {/if}
      {#if notice}<span class="muted">{notice}</span>{/if}
    </section>

    <section class="panel">
      <h2>{t("nav.workflow")}</h2>
      <div class="row">
        <label>
          {t("workflow.action")}
          <select bind:value={action}>
            {#each ["submit", "approve", "reject", "publish", "unpublish", "archive"] as name (name)}
              <option value={name}>{name}</option>
            {/each}
          </select>
        </label>
        <label>
          {t("workflow.reason")}
          <input bind:value={reason} />
        </label>
        <button type="button" onclick={transition}>{t("workflow.action")}</button>
      </div>
      {#if NEEDS_REASON.includes(action) && !reason}
        <p class="ahead">{t("workflow.reasonRequired")}</p>
      {/if}
    </section>

    <section class="panel">
      <h2>{t("entry.history")}</h2>
      <p class="muted">{t("entry.restoreHelp")}</p>
      <table>
        <thead>
          <tr>
            <th>#</th>
            <th>{t("common.title")}</th>
            <th>{t("common.author")}</th>
            <th>{t("common.updated")}</th>
            <th>{t("common.actions")}</th>
          </tr>
        </thead>
        <tbody>
          {#each history as revision (revision.pid)}
            <tr>
              <td>
                {revision.number}
                {#if revision.is_published}<span class="state published">{t("entry.published")}</span>{/if}
              </td>
              <td>{revision.title}</td>
              <td>{actor(revision.author_ref)}</td>
              <td>{when(revision.created_at)}</td>
              <td>
                {#if editing && revision.pid !== editing.pid}
                  <button type="button" onclick={() => compare(revision.pid, editing!.pid)}>
                    {t("entry.diff")}
                  </button>
                  <button type="button" onclick={() => restore(revision.pid)}>
                    {t("entry.restore")}
                  </button>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>

      {#if comparison && !conflict}
        <p>
          #{comparison.from.number} → #{comparison.to.number}:
          {comparison.diff.identical ? t("entry.identical") : comparison.diff.block_comparison}
        </p>
      {/if}
    </section>

    {#if previewed}
      <section class="panel">
        <h2>{t("preview.heading")}</h2>
        <p class="muted">{t("preview.serverSide")}</p>
        {#if !previewed.is_published_revision}
          <p class="ahead">{t("preview.notLive")}</p>
        {/if}
        <p>{t("preview.localeServed")}: {previewed.locale}</p>
        <!-- Preview bodies are rendered as text, block by block. The
             service sanitizes on write; that is a boundary control,
             not a reason to hand its output to `{@html}`. -->
        {#each previewed.revision.blocks as block, index (index)}
          <p><strong>{block.kind}</strong>: {String(block.text ?? block.alt ?? "")}</p>
        {/each}
      </section>
    {/if}
  {/if}
{/if}

<style>
  tr.selected {
    outline: 2px solid var(--accent);
  }
  button.link {
    background: none;
    border: none;
    color: var(--accent);
    cursor: pointer;
    font: inherit;
    padding: 0;
    text-decoration: underline;
  }
  .row {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    flex-wrap: wrap;
  }
</style>
