<!--
  LabeledField — wraps an input control with its <label>, an optional
  required marker, an optional hint, and a FieldError. The input itself is
  supplied as the `children` snippet so this stays control-agnostic.

  Props:
    - label (string): visible field label text.
    - for (string): id of the control the label points at (bound as `htmlFor`).
    - required (boolean, default false): show the "*" required marker.
    - error (string | null, default null): validation error to display.
    - hint (string, optional): helper text; hidden while an error is shown.
    - children (Snippet): the actual input/select control.
-->
<script lang="ts">
    import type { Snippet } from "svelte";
    import FieldError from "./FieldError.svelte";

    // `for` is a reserved word, so alias the prop to `htmlFor` for use below.
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
    <!-- Hint and error are mutually exclusive: the error takes precedence. -->
    {#if hint && !error}<small class="hint">{hint}</small>{/if}
    <FieldError {error} />
</div>

<style>
    .field { display: flex; flex-direction: column; gap: 0.25rem; margin-bottom: 0.75rem; }
    label { font-weight: 600; font-size: 0.875rem; }
    .required { color: var(--mxi-color-danger); margin-left: 0.125rem; }
    .hint { color: var(--mxi-color-muted); font-size: 0.75rem; }
</style>
