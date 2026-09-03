<!--
  LabeledField — label + input wrapper with optional hint and error.

  Purpose: standardises a labelled form control. Wraps an arbitrary input
  (passed as `children`) with a <label>, a required marker, an optional
  hint line, and a FieldError. Keeps spacing/ARIA consistent across forms.

  $props:
    - label (string): visible field label text.
    - for (string, aliased to htmlFor): id of the control the label targets;
      callers must give the child input the same id for label association.
    - required (boolean, default false): renders a "*" marker.
    - error (string | null, default null): validation message; when set the
      field gets the `has-error` class and the hint is suppressed.
    - hint (string, optional): helper text shown only when there is no error.
    - children (Snippet): the actual input/select/textarea element.
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
        {label}{#if required}<span class="required" aria-hidden="true">*</span
            >{/if}
    </label>
    <!-- The caller-provided control (input/select/textarea). -->
    {@render children()}
    <!-- Hint shows only when there is no error, to avoid competing text. -->
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
