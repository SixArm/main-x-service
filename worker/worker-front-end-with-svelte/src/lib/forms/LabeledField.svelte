<!--
  LabeledField — wraps a single form control with a <label>, an optional
  required marker, an optional hint, and a FieldError. The caller supplies
  the actual input element via `children`, wiring its `id` to match `for`.

  $props:
    - label: string — visible label text.
    - for: string — id of the wrapped control (mapped to local `htmlFor`
      because `for` is a reserved word).
    - required?: boolean — show the "*" required marker (default false).
    - error?: string | null — validation message; also toggles .has-error.
    - hint?: string — helper text shown only when there is no error.
    - children: Snippet — the input/select control to label.
-->
<script lang="ts">
    import type { Snippet } from "svelte";
    import FieldError from "./FieldError.svelte";

    let {
        label,
        // Rename `for` (reserved word) to a usable local identifier.
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
    <!-- Show the hint only when there's no error, so the error takes priority. -->
    {#if hint && !error}<small class="hint">{hint}</small>{/if}
    <FieldError {error} />
</div>

<style>
    .field { display: flex; flex-direction: column; gap: 0.25rem; margin-bottom: 0.75rem; }
    label { font-weight: 600; font-size: 0.875rem; }
    .required { color: var(--mxi-color-danger); margin-left: 0.125rem; }
    .hint { color: var(--mxi-color-muted); font-size: 0.75rem; }
</style>
