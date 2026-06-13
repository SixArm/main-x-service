<script lang="ts">
    import { page } from '$app/state';
    import { cache } from '$lib/store/cache.svelte';
    import { api, ApiError } from '$lib/api/client';
    import type { Folder } from '$lib/store/types';
    import { formatNhsNumber, isValidNhsNumber, normaliseNhsNumber } from '$lib/store/nhs';

    import BackLink from '$lib/components/BackLink/BackLink.svelte';
    import Alert from '$lib/components/Alert/Alert.svelte';
    import Badge from '$lib/components/Badge/Badge.svelte';
    import Form from '$lib/components/Form/Form.svelte';
    import Field from '$lib/components/Field/Field.svelte';
    import Button from '$lib/components/Button/Button.svelte';
    import UnitedKingdomNationalHealthServiceNumberInput from '$lib/components/UnitedKingdomNationalHealthServiceNumberInput/UnitedKingdomNationalHealthServiceNumberInput.svelte';

    let nhsNumber = $state('');
    let folderId = $state('');
    let toCabinetId = $state<string>('');
    let workerId = $state<string>('');
    let movedBy = $state('');
    let reason = $state('');

    let nhsError = $state('');
    let folderError = $state('');
    let cabinetError = $state('');
    let success = $state<string | null>(null);

    // Folders for the currently-typed NHS Number, looked up against the API
    // on input change (debounced).
    let patientFolders = $state<Folder[]>([]);
    let lookupDebounce: ReturnType<typeof setTimeout> | null = null;

    async function lookupFolders(nhs: string) {
        if (normaliseNhsNumber(nhs).length !== 10) {
            patientFolders = [];
            return;
        }
        try {
            const list = await api.folders.list({ nhsNumber: nhs });
            patientFolders = list.items;
        } catch {
            patientFolders = [];
        }
    }

    function onNhsInput() {
        if (lookupDebounce) clearTimeout(lookupDebounce);
        lookupDebounce = setTimeout(() => lookupFolders(nhsNumber), 300);
    }

    $effect(() => {
        const folderParam = page.url.searchParams.get('folder');
        const nhsParam = page.url.searchParams.get('nhs');
        if (folderParam) {
            folderId = folderParam;
            // Best-effort: if we already know the folder via /folders cache,
            // prefill the NHS Number from it.
            const known = cache.folders.find((f) => f.id === folderParam);
            if (known) {
                nhsNumber = known.nhsNumber;
                lookupFolders(known.nhsNumber);
            }
        }
        if (nhsParam) {
            nhsNumber = formatNhsNumber(nhsParam);
            lookupFolders(nhsNumber);
        }
    });

    function badgeType(status: string): 'success' | 'warning' | 'info' | 'default' {
        if (status === 'in-cabinet') return 'success';
        if (status === 'in-transit') return 'warning';
        return 'default';
    }

    async function handleSubmit() {
        nhsError = '';
        folderError = '';
        cabinetError = '';
        success = null;

        const formatted = formatNhsNumber(nhsNumber);
        if (!isValidNhsNumber(formatted)) {
            nhsError = 'Enter a valid 10-digit NHS Number.';
        }
        if (!folderId) {
            folderError = 'Select which folder to move.';
        }
        if (nhsError || folderError) return;

        const target = toCabinetId === '__transit' || toCabinetId === '' ? null : toCabinetId;
        try {
            const event = await cache.recordMove({
                folderId,
                toCabinetId: target,
                workerId: workerId || null,
                movedBy: movedBy.trim() || undefined,
                reason: reason.trim() || undefined
            });
            success = `Recorded move of ${event.patientName} — ${event.folderTitle} from ${event.fromCabinetLabel} to ${event.toCabinetLabel}.`;
            reason = '';
            // Refresh the patient folders pane to reflect the new location.
            lookupFolders(nhsNumber);
        } catch (e) {
            if (e instanceof ApiError && e.status === 422) {
                const body = e.body as { errors?: Record<string, string> } | null;
                const errs = body?.errors ?? {};
                folderError = errs.folder_id ?? folderError;
                cabinetError = errs.to_cabinet_id ?? cabinetError;
                if (!folderError && !cabinetError) {
                    folderError = e.message;
                }
            } else if (e instanceof ApiError && e.status === 404) {
                folderError = 'Folder not found.';
            } else {
                folderError = (e as Error).message;
            }
        }
    }
</script>

<BackLink href="/">Back to dashboard</BackLink>

<h2>Move a folder</h2>
<p>
    Enter a patient's NHS Number, pick the folder you're moving, then pick the
    destination cabinet (or mark it in transit).
</p>

{#if success}
    <Alert type="success" heading="Move recorded">{success}</Alert>
{/if}

<div class="split">
    <Form label="Move folder" onsubmit={handleSubmit}>
        <Field label="Patient NHS Number" required error={nhsError}>
            <UnitedKingdomNationalHealthServiceNumberInput
                label="NHS Number" bind:value={nhsNumber} oninput={onNhsInput} required
            />
        </Field>

        <Field label="Folder" required error={folderError} description={patientFolders.length ? 'Pick which of this patient\'s folders to move.' : 'Enter an NHS Number to see folders.'}>
            <select bind:value={folderId} required disabled={patientFolders.length === 0}>
                <option value="">— Select folder —</option>
                {#each patientFolders as f (f.id)}
                    <option value={f.id}>{f.title} — {f.cabinetLabel} · {f.status}</option>
                {/each}
            </select>
        </Field>

        <Field label="Destination" required error={cabinetError}>
            <select bind:value={toCabinetId} required>
                <option value="">— Select cabinet —</option>
                <option value="__transit">In transit (porter carrying)</option>
                {#each cache.cabinets as c (c.id)}
                    <option value={c.id}>{c.label} ({c.containerPath})</option>
                {/each}
            </select>
        </Field>

        <Field label="Worker (from Main Worker Service)" description="Pick a registered worker, or leave blank to use the free-text field below.">
            <select bind:value={workerId}>
                <option value="">— Free-text only —</option>
                {#each cache.workers as w (w.id)}
                    <option value={w.id}>{w.name}{w.role ? ` — ${w.role}` : ''}</option>
                {/each}
            </select>
        </Field>

        <Field label="Moved by (free text)" description="Used when no worker is selected.">
            <input bind:value={movedBy} placeholder="e.g. Alice (porter)" />
        </Field>

        <Field label="Reason">
            <input bind:value={reason} placeholder="e.g. Outpatient appointment" />
        </Field>

        <div class="actions">
            <a href="/" class="button secondary">Cancel</a>
            <Button type="submit" disabled={!folderId}>Record move</Button>
        </div>
    </Form>

    <aside class="panel" aria-labelledby="patient-folders">
        <h3 id="patient-folders">Patient folders</h3>
        {#if patientFolders.length > 0}
            <ul style="list-style: none; padding: 0; margin: 0;">
                {#each patientFolders as f (f.id)}
                    <li style="padding: var(--nhs-space-1) 0; border-bottom: 1px solid var(--nhs-pale-grey);">
                        <strong>{f.title}</strong>
                        <Badge type={badgeType(f.status)}>{f.status}</Badge><br />
                        <small>{f.cabinetLabel}</small>
                    </li>
                {/each}
            </ul>
        {:else}
            <p>Enter a valid NHS Number to see this patient's folders.</p>
        {/if}
    </aside>
</div>
