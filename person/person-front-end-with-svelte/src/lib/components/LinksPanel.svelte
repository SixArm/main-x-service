<!--
  LinksPanel — the operator surface for this person's **cross-service**
  entity links (`agents/share/cross-service-linking.md` §4.1): the edges
  asserting that this person is the same human as a worker record, or is
  affiliated with an organization.

  Deliberately NOT the `links` field on the Person record itself — that is
  the within-entity person→person merge/dedup relationship and a matcher
  signal. Cross-service edges are never a matcher signal (§7), so the two
  live in separate sections and separate types.

  Reads `GET /api/persons/{id}/links` on mount, asserts new edges through
  `POST …/links`, and withdraws one through `DELETE …/links/{linkId}`.
  Writes are optimistic server-side (no call to the target service), so a
  link to a record that does not exist is accepted here and resolved later
  by the link-graph aggregator — the form validates only what is knowable
  locally (kind ↔ target-type), and surfaces the server's own reason for
  anything else.

  Props:
    - personId: string — the person whose outbound edges these are.
-->
<script lang="ts">
    import { onMount } from "svelte";
    import { PersonRepository } from "$lib/api/persons.js";
    import type { CreateLinkRequest, EntityLink } from "$lib/api/types.js";
    import {
        PERSON_LINK_KINDS,
        refPlaceholder,
        validateToRef,
        expectedTargetType,
        type PersonLinkKind,
    } from "$lib/links.js";
    import { t } from "$lib/i18n.svelte.js";

    let { personId }: { personId: string } = $props();

    const repo = PersonRepository.withFetch();

    let links = $state<EntityLink[]>([]);
    let loading = $state(true);
    // The list's own failure (load / withdraw), shown above the table.
    let listError = $state<string | null>(null);
    // The form's failure (client-side rule or the server's 422 reason).
    let formError = $state<string | null>(null);
    let submitting = $state(false);
    // Id of the edge currently being withdrawn, so only its button
    // shows the pending label.
    let withdrawing = $state<string | null>(null);

    // Form fields. `kind` drives both the expected target type and the
    // input's placeholder, so selecting it first teaches the constraint.
    let kind = $state<PersonLinkKind>("same_identity");
    let toRef = $state("");
    // `bind:value` on `type="number"` yields a number (or null when the
    // field is empty) — never a string — so this is typed accordingly.
    let confidence = $state<number | null>(null);
    let provenance = $state("");
    let validFrom = $state("");
    let validTo = $state("");

    const placeholder = $derived(refPlaceholder(kind));

    // Translated label per kind, kept beside the kind list so adding a
    // kind is a compile error here until it has a label.
    const KIND_LABELS: Record<PersonLinkKind, () => string> = {
        same_identity: () => t("links.kind.sameIdentity"),
        works_at: () => t("links.kind.worksAt"),
        member_of: () => t("links.kind.memberOf"),
    };

    async function load() {
        loading = true;
        listError = null;
        try {
            links = await repo.listLinks(personId);
        } catch (err) {
            listError = err instanceof Error ? err.message : String(err);
        } finally {
            loading = false;
        }
    }

    onMount(load);

    // Map a client-side rejection to its translated message. Kept apart
    // from the server's message so the operator can tell which side
    // refused.
    function localProblemMessage(): string | null {
        const problem = validateToRef(kind, toRef);
        if (problem === null) return null;
        if (problem === "required") return t("links.error.required");
        if (problem === "malformed") return t("links.error.malformedRef");
        return (
            t("links.error.wrongType.prefix") +
            expectedTargetType(kind) +
            t("links.error.wrongType.suffix")
        );
    }

    async function handleSubmit(event: SubmitEvent) {
        event.preventDefault();
        formError = localProblemMessage();
        if (formError !== null) return;

        // Send only what the operator filled in: the server defaults
        // provenance and leaves the rest unset, and an empty string is
        // not the same as "unspecified".
        const request: CreateLinkRequest = { kind, to_ref: toRef.trim() };
        if (confidence !== null && Number.isFinite(confidence)) {
            request.confidence = confidence;
        }
        if (provenance.trim() !== "") request.provenance = provenance.trim();
        if (validFrom !== "") request.valid_from = validFrom;
        if (validTo !== "") request.valid_to = validTo;

        submitting = true;
        try {
            await repo.createLink(personId, request);
            // Clear only the target: the operator usually asserts several
            // edges of the same kind in a row.
            toRef = "";
            confidence = null;
            provenance = "";
            validFrom = "";
            validTo = "";
            await load();
        } catch (err) {
            formError = err instanceof Error ? err.message : String(err);
        } finally {
            submitting = false;
        }
    }

    // Withdrawing is a real mutation of the cross-service graph, so it
    // confirms first — matching the soft-delete convention on the person
    // detail page.
    async function handleWithdraw(link: EntityLink) {
        if (!confirm(t("links.confirmWithdraw"))) return;
        withdrawing = link.id;
        listError = null;
        try {
            await repo.deleteLink(personId, link.id);
            await load();
        } catch (err) {
            listError = err instanceof Error ? err.message : String(err);
        } finally {
            withdrawing = null;
        }
    }
</script>

<section class="surface stack">
    <h2>{t("links.title")}</h2>

    {#if listError}
        <div class="banner error" role="alert">{listError}</div>
    {/if}

    {#if loading}
        <p class="muted">{t("links.loading")}</p>
    {:else if links.length === 0}
        <p class="muted">{t("links.empty")}</p>
    {:else}
        <ul class="links">
            {#each links as link (link.id)}
                <li class="link">
                    <div class="link-main">
                        <strong>{link.kind}</strong>
                        <code>{link.to_ref}</code>
                    </div>
                    <div class="meta small muted">
                        {#if link.role}{t("links.role")}: {link.role} ·
                        {/if}
                        {#if link.confidence != null}{t("links.confidence")}: {link.confidence}
                            ·
                        {/if}
                        {t("links.provenance")}: {link.provenance}
                        {#if link.valid_from}
                            · {t("links.validFrom")}: {link.valid_from}{/if}
                        {#if link.valid_to}
                            · {t("links.validTo")}: {link.valid_to}{/if}
                    </div>
                    <button
                        class="button danger"
                        disabled={withdrawing === link.id}
                        onclick={() => handleWithdraw(link)}
                    >
                        {withdrawing === link.id
                            ? t("links.withdrawing")
                            : t("links.withdraw")}
                    </button>
                </li>
            {/each}
        </ul>
    {/if}

    <form class="assert" onsubmit={handleSubmit}>
        <h3>{t("links.assertHeading")}</h3>

        {#if formError}
            <div class="banner error" role="alert">{formError}</div>
        {/if}

        <div class="fields">
            <div class="field">
                <label for="link-kind">{t("links.kind")}</label>
                <select id="link-kind" bind:value={kind}>
                    {#each PERSON_LINK_KINDS as k (k)}
                        <option value={k}>{KIND_LABELS[k]()}</option>
                    {/each}
                </select>
            </div>

            <div class="field grow">
                <label for="link-to-ref">{t("links.toRef")}</label>
                <input id="link-to-ref" bind:value={toRef} {placeholder} />
                <small class="hint">{t("links.toRefHint")}</small>
            </div>

            <div class="field">
                <label for="link-confidence">{t("links.confidence")}</label>
                <input
                    id="link-confidence"
                    type="number"
                    min="0"
                    max="1"
                    step="0.01"
                    bind:value={confidence}
                />
                <small class="hint">{t("links.confidenceHint")}</small>
            </div>

            <div class="field">
                <label for="link-provenance">{t("links.provenance")}</label>
                <input
                    id="link-provenance"
                    bind:value={provenance}
                    placeholder="operator"
                />
                <small class="hint">{t("links.provenanceHint")}</small>
            </div>

            <div class="field">
                <label for="link-valid-from">{t("links.validFrom")}</label>
                <input
                    id="link-valid-from"
                    type="date"
                    bind:value={validFrom}
                />
            </div>

            <div class="field">
                <label for="link-valid-to">{t("links.validTo")}</label>
                <input id="link-valid-to" type="date" bind:value={validTo} />
            </div>
        </div>

        <button class="button primary" type="submit" disabled={submitting}>
            {submitting ? t("links.submitting") : t("links.submit")}
        </button>
    </form>
</section>

<style>
    .links {
        list-style: none;
        padding: 0;
        margin: 0;
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
    }
    .link {
        display: grid;
        grid-template-columns: 1fr max-content;
        gap: 0.25rem 0.75rem;
        align-items: center;
        padding: 0.625rem 0.75rem;
        border: 1px solid var(--mxi-color-border);
        border-radius: var(--mxi-radius);
    }
    .link-main {
        display: flex;
        gap: 0.5rem;
        align-items: baseline;
        flex-wrap: wrap;
    }
    .link button {
        grid-row: 1 / span 2;
    }
    .meta {
        grid-column: 1;
    }
    .assert {
        border-top: 1px solid var(--mxi-color-border);
        padding-top: var(--mxi-spacing);
    }
    .fields {
        display: flex;
        flex-wrap: wrap;
        gap: 0.75rem;
        margin-bottom: 0.75rem;
    }
    .field {
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
    }
    .field.grow {
        flex: 1 1 20rem;
    }
    label {
        font-weight: 600;
        font-size: 0.875rem;
    }
    .hint {
        color: var(--mxi-color-muted);
        font-size: 0.75rem;
    }
</style>
