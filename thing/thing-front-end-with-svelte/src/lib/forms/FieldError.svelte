<!--
  FieldError — renders a single inline validation message for a form field.

  Purpose: a tiny, reusable error slot used by LabeledField and bespoke
  forms so error styling/ARIA stay consistent.

  $props:
    - error (string | null, default null): the message to show; when
      null/empty nothing is rendered.

  Accessibility: uses role="alert" so assistive tech announces the message
  when it appears.
-->
<script lang="ts">
    let { error = null }: { error?: string | null } = $props();
</script>

{#if error}
    <small class="error" role="alert">{error}</small>
{/if}

<style>
    .error { color: var(--mxi-color-danger); font-size: 0.75rem; }
</style>
