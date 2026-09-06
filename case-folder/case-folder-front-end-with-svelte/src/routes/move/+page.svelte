<script lang="ts">
    // Move folder (`/move`) — record a folder placement / hand-off.
    //
    // The operator enters a patient's NHS Number; the page debounce-looks
    // up that patient's folders, they pick one folder plus a destination
    // (a cabinet, or "in transit" while a porter carries it), optionally a
    // worker and reason, and submit. The move round-trips through
    // `cache.recordMove`, which also patches the cached folder location.
    //
    // The form can be deep-linked: `?folder=<id>` (from the scan page) or
    // `?nhs=<number>` pre-fills the relevant fields via the $effect below.
    //
    // State: form fields (nhsNumber/folderId/toCabinetId/workerId/movedBy/
    // reason), per-field error strings, a success message, the live
    // patientFolders pane, and the debounce timer handle.

    import { page } from '$app/state';
    import { cache } from '$lib/store/cache.svelte';
    import { api, ApiError } from '$lib/api/client';
    import type { Folder } from '$lib/store/types';
    import {
        formatNhsNumber,
        isValidNhsNumber,
        normaliseNhsNumber,
    } from '$lib/store/nhs';

    import BackLink from '$lib/components/BackLink/BackLink.svelte';
    import Alert from '$lib/components/Alert/Alert.svelte';
    import Badge from '$lib/components/Badge/Badge.svelte';
    import Form from '$lib/components/Form/Form.svelte';
    import Field from '$lib/components/Field/Field.svelte';
    import Button from '$lib/components/Button/Button.svelte';
    import UnitedKingdomNationalHealthServiceNumberInput from '$lib/components/UnitedKingdomNationalHealthServiceNumberInput/UnitedKingdomNationalHealthServiceNumberInput.svelte';
    import { t, tf, statusLabel } from '$lib/i18n.svelte';

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

    // Fetch this patient's folders into the side pane / folder picker.
    // Only queries once the NHS Number is a complete 10 digits; otherwise
    // clears the pane.
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

    // Debounce the lookup so we don't hit the API on every keystroke
    // while the porter types/scans the number.
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

    // Map a folder status to a Badge colour for the side pane.
    function badgeType(
        status: string,
    ): 'success' | 'warning' | 'info' | 'default' {
        if (status === 'in-cabinet') return 'success';
        if (status === 'in-transit') return 'warning';
        return 'default';
    }

    // Validate, record the move, and report success/failure inline.
    async function handleSubmit() {
        nhsError = '';
        folderError = '';
        cabinetError = '';
        success = null;

        const formatted = formatNhsNumber(nhsNumber);
        if (!isValidNhsNumber(formatted)) {
            nhsError = t('move.invalidNhs');
        }
        if (!folderId) {
            folderError = t('move.selectFolderError');
        }
        if (nhsError || folderError) return;

        // The sentinel "__transit" and the empty option both mean "no
        // cabinet" (folder in transit); send null in both cases.
        const target =
            toCabinetId === '__transit' || toCabinetId === ''
                ? null
                : toCabinetId;
        try {
            const event = await cache.recordMove({
                folderId,
                toCabinetId: target,
                workerId: workerId || null,
                movedBy: movedBy.trim() || undefined,
                reason: reason.trim() || undefined,
            });
            success = tf('move.recordedSummary', {
                patient: event.patientName,
                folder: event.folderTitle,
                from: event.fromCabinetLabel,
                to: event.toCabinetLabel,
            });
            reason = '';
            // Refresh the patient folders pane to reflect the new location.
            lookupFolders(nhsNumber);
        } catch (e) {
            if (e instanceof ApiError && e.status === 422) {
                const body = e.body as {
                    errors?: Record<string, string>;
                } | null;
                const errs = body?.errors ?? {};
                folderError = errs.folder_id ?? folderError;
                cabinetError = errs.to_cabinet_id ?? cabinetError;
                if (!folderError && !cabinetError) {
                    folderError = e.message;
                }
            } else if (e instanceof ApiError && e.status === 404) {
                folderError = t('move.folderNotFound');
            } else {
                folderError = (e as Error).message;
            }
        }
    }
</script>

<BackLink href="/">{t('common.backToDashboard')}</BackLink>

<h2>{t('move.heading')}</h2>
<p>{t('move.intro')}</p>

{#if success}
    <Alert type="success" heading={t('move.recorded')}>{success}</Alert>
{/if}

<div class="split">
    <Form label={t('move.formLabel')} onsubmit={handleSubmit}>
        <Field label={t('move.patientNhs')} required error={nhsError}>
            <UnitedKingdomNationalHealthServiceNumberInput
                label={t('common.nhsNumber')}
                bind:value={nhsNumber}
                oninput={onNhsInput}
                required
            />
        </Field>

        <Field
            label={t('move.folder')}
            required
            error={folderError}
            description={patientFolders.length
                ? t('move.pickFolderDescription')
                : t('move.enterNhsDescription')}
        >
            <select
                bind:value={folderId}
                required
                disabled={patientFolders.length === 0}
            >
                <option value="">{t('move.selectFolderOption')}</option>
                {#each patientFolders as f (f.id)}
                    <option value={f.id}
                        >{f.title} — {f.cabinetLabel} · {statusLabel(
                            f.status,
                        )}</option
                    >
                {/each}
            </select>
        </Field>

        <Field label={t('move.destination')} required error={cabinetError}>
            <select bind:value={toCabinetId} required>
                <option value="">{t('common.selectCabinetOption')}</option>
                <option value="__transit">{t('common.inTransitPorter')}</option>
                {#each cache.cabinets as c (c.id)}
                    <option value={c.id}>{c.label} ({c.containerPath})</option>
                {/each}
            </select>
        </Field>

        <Field
            label={t('move.workerLabel')}
            description={t('move.workerDescription')}
        >
            <select bind:value={workerId}>
                <option value="">{t('move.freeTextOnly')}</option>
                {#each cache.workers as w (w.id)}
                    <option value={w.id}
                        >{w.name}{w.role ? ` — ${w.role}` : ''}</option
                    >
                {/each}
            </select>
        </Field>

        <Field
            label={t('move.movedByLabel')}
            description={t('move.movedByDescription')}
        >
            <input
                bind:value={movedBy}
                placeholder={t('move.movedByPlaceholder')}
            />
        </Field>

        <Field label={t('common.reason')}>
            <input
                bind:value={reason}
                placeholder={t('move.reasonPlaceholder')}
            />
        </Field>

        <div class="actions">
            <a href="/" class="button secondary">{t('common.cancel')}</a>
            <Button type="submit" disabled={!folderId}
                >{t('move.recordMove')}</Button
            >
        </div>
    </Form>

    <aside class="panel" aria-labelledby="patient-folders">
        <h3 id="patient-folders">{t('move.patientFolders')}</h3>
        {#if patientFolders.length > 0}
            <ul style="list-style: none; padding: 0; margin: 0;">
                {#each patientFolders as f (f.id)}
                    <li
                        style="padding: var(--nhs-space-1) 0; border-bottom: 1px solid var(--nhs-pale-grey);"
                    >
                        <strong>{f.title}</strong>
                        <Badge type={badgeType(f.status)}
                            >{statusLabel(f.status)}</Badge
                        ><br />
                        <small>{f.cabinetLabel}</small>
                    </li>
                {/each}
            </ul>
        {:else}
            <p>{t('move.enterValidNhs')}</p>
        {/if}
    </aside>
</div>
