<!--
  LabeledField — wraps a form control with a `<label>`, an optional
  required marker, an optional hint, and an inline error.

  The control itself is passed as the `children` snippet; this component
  owns only the surrounding label/hint/error chrome and the error styling.

  $props:
    - label (string)       — visible label text.
    - for (string)         — id of the control the label points at (aliased
                             to `htmlFor` since `for` is a reserved word).
    - required (boolean)   — show the `*` required marker. Default false.
    - error (string|null)  — inline error; also toggles `.has-error`.
    - hint (string?)       — helper text, hidden while an error is shown.
    - children (Snippet)   — the actual input/select control.
-->
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
        <!-- Required marker is decorative; `*` is hidden from assistive tech. -->
        {label}{#if required}<span class="required" aria-hidden="true">*</span
            >{/if}
    </label>
    {@render children()}
    <!-- Hint and error are mutually exclusive: error wins when both exist. -->
    {#if hint && !error}<small class="hint">{hint}</small>{/if}
    <FieldError {error} />
</div>

<style>
    .field {
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
        margin-bottom: 0.75rem;
    }
    label {
        font-weight: 600;
        font-size: 0.875rem;
    }
    .required {
        color: var(--mxi-color-danger);
        margin-left: 0.125rem;
    }
    .hint {
        color: var(--mxi-color-muted);
        font-size: 0.75rem;
    }
</style>
