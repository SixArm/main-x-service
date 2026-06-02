<script lang="ts">
    import type { Snippet } from "svelte";
    import FieldError from "./FieldError.svelte";

    let {
        label,
        for: htmlFor,
        required = false,
        error = null,
        hint,
        children,
    }: {
        label: string;
        for: string;
        required?: boolean;
        error?: string | null;
        hint?: string;
        children: Snippet;
    } = $props();
</script>

<div class="field" class:has-error={Boolean(error)}>
    <label for={htmlFor}>
        {label}{#if required}<span class="required" aria-hidden="true">*</span>{/if}
    </label>
    {@render children()}
    {#if hint && !error}<small class="hint">{hint}</small>{/if}
    <FieldError {error} />
</div>

<style>
    .field { display: flex; flex-direction: column; gap: 0.25rem; margin-bottom: 0.75rem; }
    label { font-weight: 600; font-size: 0.875rem; }
    .required { color: var(--mxi-color-danger); margin-left: 0.125rem; }
    .hint { color: var(--mxi-color-muted); font-size: 0.75rem; }
</style>
