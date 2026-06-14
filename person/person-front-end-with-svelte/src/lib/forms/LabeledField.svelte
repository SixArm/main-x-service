<!--
  LabeledField — wraps a form control with an accessible <label>, an
  optional required marker, an optional hint, and a FieldError. The hint is
  hidden once an error is present so the two never compete for the slot.

  Props:
    - label: string — visible label text.
    - for: string — id of the control the label points at (bound to htmlFor).
    - required?: boolean — show the "*" required marker.
    - error?: string | null — validation message (toggles .has-error).
    - hint?: string — helper text shown only when there is no error.
    - children: Snippet — the actual input/select/control.
-->
<script lang="ts">
    import type { Snippet } from "svelte";
    import FieldError from "./FieldError.svelte";

    // `for` is a reserved word, so the prop is renamed to `htmlFor` locally.
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
