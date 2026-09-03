<!--
  FieldError — inline validation message for a single form field.

  Renders nothing when `error` is empty, so callers can pass an optional
  error straight through. Uses role="alert" so assistive tech announces
  the message when it appears.

  Props:
    - error?: string | null — the message to show; null/empty renders nothing.
-->
<script lang="ts">
    let { error = null }: { error?: string | null } = $props();
</script>

{#if error}
    <small class="error" role="alert">{error}</small>
{/if}

<style>
    .error {
        color: var(--mxi-color-danger);
        font-size: 0.75rem;
    }
</style>
