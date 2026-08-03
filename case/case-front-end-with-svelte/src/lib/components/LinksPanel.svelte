<!--
  Cross-service links panel — the `subject_of` (case → person) edges this
  case asserts (`agents/share/cross-service-linking.md` §4.1, §9).

  Sensitivity (§10): a `subject_of` edge asserts that a named person is
  the subject of a governmental case, which is itself sensitive data —
  the service authorises reading and writing it at the same level as
  reading the case, and audits every write. The UI treats it accordingly:
  a plainly-labelled section with an explanatory note, not a decorative
  widget, and an explicit confirmation before a withdrawal.

  Because a case may originate exactly one edge kind, there is no kind
  picker: `kind` is fixed to `subject_of` and the service rejects
  anything else with 422.

  State:
    - links       : the loaded edges (empty until fetched).
    - loading/error: load-phase flags for the list.
    - toRef/confidence/provenance/validFrom/validTo : bound form fields.
    - submitting  : disables the submit button while a POST is in flight.
    - formError   : validation / server message for the form.
    - withdrawing : id of the edge currently being withdrawn, if any.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { CaseRepository } from "$lib/api/cases";
  import { ApiError } from "$lib/api/client";
  import { SUBJECT_OF } from "$lib/api/types";
  import type { EntityLink } from "$lib/api/types";
  import { validateLink } from "$lib/components/link-validation";
  import { t, translate } from "$lib/i18n.svelte";

  interface Props {
    /** Persistent id of the case whose links this panel manages. */
    casePid: string;
  }

  let { casePid }: Props = $props();

  const repo = CaseRepository.withFetch();

  let links = $state<EntityLink[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  let toRef = $state("");
  let confidence = $state<number | null>(null);
  let provenance = $state("");
  let validFrom = $state("");
  let validTo = $state("");
  let submitting = $state(false);
  let formError = $state<string | null>(null);
  let withdrawing = $state<string | null>(null);

  // The service answers a rejected edge with loco's ErrorDetail —
  // `{error: "validation", description: "<reason>"}` — and the shared
  // client's generic message extractor prefers `error`, which is the
  // machine code, not the reason. Reach for `description` first so the
  // operator sees "edge kind ... does not permit ..." rather than the
  // word "validation".
  function describe(err: unknown): string {
    if (err instanceof ApiError) {
      const body = err.body;
      if (body && typeof body === "object") {
        const detail = (body as Record<string, unknown>).description;
        if (typeof detail === "string" && detail.length > 0) {
          return `${err.status}: ${detail}`;
        }
      }
      return `${err.status}: ${err.message}`;
    }
    return err instanceof Error ? err.message : String(err);
  }

  /** (Re)load this case's active edges. */
  async function load() {
    loading = true;
    error = null;
    try {
      links = await repo.listLinks(casePid);
    } catch (err) {
      error = `${t("links.loadFailed")}: ${describe(err)}`;
    } finally {
      loading = false;
    }
  }

  onMount(load);

  /** Assert `subject_of` → the entered person, then refresh the list. */
  async function record() {
    const guardKey = validateLink(toRef.trim(), confidence);
    if (guardKey) {
      formError = t(guardKey);
      return;
    }
    submitting = true;
    formError = null;
    try {
      await repo.createLink(casePid, {
        kind: SUBJECT_OF,
        to_ref: toRef.trim(),
        // Omit-as-null: the service treats an absent optional as unset,
        // and blank provenance falls back to "operator" server-side.
        confidence,
        provenance: provenance.trim() || null,
        valid_from: validFrom || null,
        valid_to: validTo || null,
      });
      toRef = "";
      confidence = null;
      provenance = "";
      validFrom = "";
      validTo = "";
      await load();
    } catch (err) {
      formError = `${t("links.recordFailed")}: ${describe(err)}`;
    } finally {
      submitting = false;
    }
  }

  /**
   * Withdraw one edge after an explicit confirmation. Withdrawal is a
   * soft-delete plus an `unlinked` event and an audit row — not a silent
   * tidy-up — so the prompt names the person reference being retracted.
   */
  async function withdraw(link: EntityLink) {
    const question = translate("links.withdrawConfirm").replace(
      "{ref}",
      link.to_ref,
    );
    if (!confirm(question)) return;
    withdrawing = link.id;
    error = null;
    try {
      await repo.deleteLink(casePid, link.id);
      await load();
    } catch (err) {
      error = `${t("links.withdrawFailed")}: ${describe(err)}`;
    } finally {
      withdrawing = null;
    }
  }

  /** "2026-01-15 → 2026-04-01", or a single bound, or an em dash. */
  function validity(link: EntityLink): string {
    if (!link.valid_from && !link.valid_to) return "—";
    return `${link.valid_from ?? "—"} → ${link.valid_to ?? "—"}`;
  }
</script>

<section class="stack" style="margin-top:1.5rem" data-testid="links-panel">
  <h2>{t("links.title")}</h2>
  <p class="muted small">{t("links.note")}</p>

  {#if loading}
    <p>{t("links.loading")}</p>
  {:else}
    {#if error}
      <p class="banner error" role="alert" data-testid="links-error">{error}</p>
    {/if}
    {#if links.length === 0}
      <p class="surface" data-testid="links-empty">{t("links.empty")}</p>
    {:else}
      <table class="surface" data-testid="links-table">
        <thead>
          <tr>
            <th>{t("links.person")}</th>
            <th>{t("links.confidence")}</th>
            <th>{t("links.validity")}</th>
            <th>{t("links.provenance")}</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {#each links as link (link.id)}
            <tr>
              <td><code>{link.to_ref}</code></td>
              <td>{link.confidence !== null ? link.confidence.toFixed(2) : "—"}</td>
              <td>{validity(link)}</td>
              <td>{link.provenance}</td>
              <td>
                <button
                  type="button"
                  onclick={() => withdraw(link)}
                  disabled={withdrawing === link.id}
                >
                  {withdrawing === link.id
                    ? t("links.withdrawing")
                    : t("links.withdraw")}
                </button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  {/if}

  <div class="surface stack">
    <h3>{t("links.addTitle")}</h3>
    <label>
      {t("links.person")}
      <input
        type="text"
        bind:value={toRef}
        placeholder="person:00000000-0000-0000-0000-000000000000"
        data-testid="links-to-ref"
      />
      <small class="muted">{t("links.personHint")}</small>
    </label>
    <div class="row">
      <label>
        {t("links.confidence")}
        <input
          type="number"
          min="0"
          max="1"
          step="0.01"
          bind:value={confidence}
          data-testid="links-confidence"
        />
        <small class="muted">{t("links.confidenceHint")}</small>
      </label>
      <label>
        {t("links.provenance")}
        <input
          type="text"
          bind:value={provenance}
          placeholder="operator"
          data-testid="links-provenance"
        />
        <small class="muted">{t("links.provenanceHint")}</small>
      </label>
    </div>
    <div class="row">
      <label>
        {t("links.validFrom")}
        <input type="date" bind:value={validFrom} data-testid="links-valid-from" />
      </label>
      <label>
        {t("links.validTo")}
        <input type="date" bind:value={validTo} data-testid="links-valid-to" />
      </label>
    </div>
    <div class="row">
      <button
        type="button"
        class="button primary"
        onclick={record}
        disabled={submitting}
        data-testid="links-submit"
      >
        {submitting ? t("links.recording") : t("links.record")}
      </button>
    </div>
    {#if formError}
      <p class="banner error" role="alert" data-testid="links-form-error">
        {formError}
      </p>
    {/if}
  </div>
</section>

<style>
  label {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.875rem;
  }
  th,
  td {
    text-align: start;
    padding: 0.4rem 0.6rem;
    border-bottom: 1px solid var(--mxi-color-border);
  }
</style>
