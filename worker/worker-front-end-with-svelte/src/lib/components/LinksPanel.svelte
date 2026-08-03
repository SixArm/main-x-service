<!--
  Cross-service links panel — the operator surface for this worker's
  outbound `entity_links` edges (see
  `agents/share/cross-service-linking.md` §4.1). An edge asserts that this
  worker IS a person record (`same_identity`) or is employed by an
  organization (`employed_by`); it is NOT the within-service
  `Worker.links` worker↔worker reference.

  Writes are optimistic and local to the worker service: it stores the
  assertion and emits a `linked` event without calling the target service,
  so a target that does not exist yet is accepted here and reconciled by
  the link-graph aggregator later.

  Props:
    - workerId — the worker whose outbound edges this panel manages.

  $state:
    - links / loading / loadError — the current edge list and its fetch.
    - kind…validTo — the assert-a-link form fields.
    - formError / submitting / withdrawingId — mutation feedback.
-->
<script lang="ts">
    import { onMount } from "svelte";
    import { WorkerRepository } from "$lib/api/workers.js";
    import type { EntityLink, WorkerEdgeKind } from "$lib/api/types.js";
    import {
        WORKER_EDGE_KINDS,
        checkConfidence,
        checkToRef,
        targetEntityType,
        targetRefExample,
    } from "$lib/api/links.js";
    import { t, tf } from "$lib/i18n.svelte.js";

    let { workerId }: { workerId: string } = $props();

    const repo = WorkerRepository.withFetch();

    let links = $state<EntityLink[]>([]);
    let loading = $state(true);
    let loadError = $state<string | null>(null);

    // Assert-a-link form. Kept as strings (raw field values) and coerced
    // at submit, so a half-typed number never becomes NaN in state.
    let kind = $state<WorkerEdgeKind>("same_identity");
    let toRef = $state("");
    let role = $state("");
    // A number input binds to `null` when blank, so this is not a string.
    let confidence = $state<number | null>(null);
    let provenance = $state("");
    let validFrom = $state("");
    let validTo = $state("");

    let formError = $state<string | null>(null);
    let submitting = $state(false);
    let withdrawingId = $state<string | null>(null);

    // The hint under the target field tracks the selected kind, so the
    // required entity type is visible before the server says so.
    const refExample = $derived(targetRefExample(kind));

    /** Human-readable message for anything thrown by the repository. */
    function messageOf(err: unknown): string {
        return err instanceof Error ? err.message : String(err);
    }

    /** Translated label for an edge kind (falls back to the raw token). */
    function kindLabel(value: string): string {
        if (value === "same_identity") return t("links.kindSameIdentity");
        if (value === "employed_by") return t("links.kindEmployedBy");
        return value;
    }

    /** (Re)load this worker's active outbound edges. */
    async function load() {
        loading = true;
        loadError = null;
        try {
            links = await repo.listLinks(workerId);
        } catch (err) {
            loadError = messageOf(err);
        } finally {
            loading = false;
        }
    }

    onMount(load);

    /**
     * Validate client-side (mirroring the service's `validate_edge`), then
     * assert the edge and refresh the list. A 422 the pre-check did not
     * catch is surfaced verbatim — the server is the authority.
     */
    async function handleSubmit(event: SubmitEvent) {
        event.preventDefault();
        formError = null;

        const problem = checkToRef(kind, toRef);
        if (problem === "required") {
            formError = t("links.errRefRequired");
            return;
        }
        if (problem === "malformed") {
            formError = tf("links.errMalformedRef", { example: refExample });
            return;
        }
        if (problem === "wrong_target") {
            formError = tf("links.errWrongTarget", {
                expected: targetEntityType(kind),
            });
            return;
        }
        if (checkConfidence(confidence) === "invalid") {
            formError = t("links.errConfidenceRange");
            return;
        }

        submitting = true;
        try {
            await repo.createLink(workerId, {
                kind,
                to_ref: toRef.trim(),
                role: role.trim() || null,
                confidence,
                provenance: provenance.trim() || null,
                valid_from: validFrom || null,
                valid_to: validTo || null,
            });
            // Clear the assertion fields but keep `kind`, since asserting
            // several edges of one kind is the common case.
            toRef = "";
            role = "";
            confidence = null;
            provenance = "";
            validFrom = "";
            validTo = "";
            await load();
        } catch (err) {
            formError = messageOf(err);
        } finally {
            submitting = false;
        }
    }

    /** Withdraw one edge after a confirm (it is a real mutation). */
    async function handleWithdraw(link: EntityLink) {
        if (!confirm(t("links.confirmWithdraw"))) return;
        formError = null;
        withdrawingId = link.id;
        try {
            await repo.deleteLink(workerId, link.id);
            await load();
        } catch (err) {
            formError = messageOf(err);
        } finally {
            withdrawingId = null;
        }
    }
</script>

<section class="surface stack">
    <h2>{t("links.heading")}</h2>
    <p class="muted small">{t("links.intro")}</p>

    {#if loading}
        <p class="muted">{t("common.loading")}</p>
    {:else if loadError}
        <div class="banner error" role="alert">{loadError}</div>
    {:else if links.length === 0}
        <p class="muted">{t("links.empty")}</p>
    {:else}
        <div class="table-wrap">
            <table>
                <thead>
                    <tr>
                        <th>{t("links.colKind")}</th>
                        <th>{t("links.colTarget")}</th>
                        <th>{t("links.colRole")}</th>
                        <th>{t("links.colConfidence")}</th>
                        <th>{t("links.colValidity")}</th>
                        <th>{t("links.colProvenance")}</th>
                        <th></th>
                    </tr>
                </thead>
                <tbody>
                    {#each links as link (link.id)}
                        <tr>
                            <td>{kindLabel(link.kind)}</td>
                            <td><code>{link.to_ref}</code></td>
                            <td>{link.role ?? "—"}</td>
                            <td>{link.confidence ?? "—"}</td>
                            <td>
                                {#if link.valid_from || link.valid_to}
                                    {link.valid_from ?? "—"} → {link.valid_to ?? "—"}
                                {:else}
                                    —
                                {/if}
                            </td>
                            <td>{link.provenance}</td>
                            <td>
                                <button
                                    type="button"
                                    class="button danger"
                                    disabled={withdrawingId === link.id}
                                    onclick={() => void handleWithdraw(link)}
                                >
                                    {withdrawingId === link.id
                                        ? t("links.withdrawing")
                                        : t("links.withdraw")}
                                </button>
                            </td>
                        </tr>
                    {/each}
                </tbody>
            </table>
        </div>
    {/if}

    <form class="stack" onsubmit={handleSubmit}>
        <h3>{t("links.assertHeading")}</h3>

        {#if formError}
            <div class="banner error" role="alert">{formError}</div>
        {/if}

        <div class="fields">
            <label for="link-kind">{t("links.kind")}</label>
            <select id="link-kind" bind:value={kind}>
                {#each WORKER_EDGE_KINDS as edgeKind}
                    <option value={edgeKind}>{kindLabel(edgeKind)}</option>
                {/each}
            </select>

            <label for="link-to-ref">{t("links.toRef")}</label>
            <span class="field">
                <input
                    id="link-to-ref"
                    type="text"
                    bind:value={toRef}
                    placeholder={refExample}
                    aria-describedby="link-to-ref-hint"
                />
                <small id="link-to-ref-hint" class="muted small">
                    {tf("links.toRefHint", { example: refExample })}
                </small>
            </span>

            <label for="link-role">{t("links.role")}</label>
            <span class="field">
                <input
                    id="link-role"
                    type="text"
                    bind:value={role}
                    aria-describedby="link-role-hint"
                />
                <small id="link-role-hint" class="muted small">
                    {t("links.roleHint")}
                </small>
            </span>

            <label for="link-confidence">{t("links.confidence")}</label>
            <span class="field">
                <input
                    id="link-confidence"
                    type="number"
                    min="0"
                    max="1"
                    step="0.01"
                    bind:value={confidence}
                    aria-describedby="link-confidence-hint"
                />
                <small id="link-confidence-hint" class="muted small">
                    {t("links.confidenceHint")}
                </small>
            </span>

            <label for="link-provenance">{t("links.provenance")}</label>
            <input
                id="link-provenance"
                type="text"
                bind:value={provenance}
                placeholder={t("links.provenancePlaceholder")}
            />

            <label for="link-valid-from">{t("links.validFrom")}</label>
            <input id="link-valid-from" type="date" bind:value={validFrom} />

            <label for="link-valid-to">{t("links.validTo")}</label>
            <input id="link-valid-to" type="date" bind:value={validTo} />
        </div>

        <div class="row">
            <button type="submit" class="button primary" disabled={submitting}>
                {submitting ? t("links.submitting") : t("links.submit")}
            </button>
        </div>
    </form>
</section>

<style>
    .table-wrap {
        overflow-x: auto;
    }
    table {
        border-collapse: collapse;
        width: 100%;
        font-size: 0.875rem;
    }
    th,
    td {
        text-align: start;
        padding: 0.375rem 0.5rem;
        border-bottom: 1px solid var(--mxi-color-border);
        vertical-align: top;
    }
    .fields {
        display: grid;
        grid-template-columns: max-content minmax(12rem, 24rem);
        gap: 0.5rem 1rem;
        align-items: start;
    }
    .fields label {
        padding-top: 0.5rem;
    }
    .field {
        display: flex;
        flex-direction: column;
        gap: 0.125rem;
    }
</style>
