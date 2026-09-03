<!--
  FieldError — renders a single inline validation error for a form field.

  Purpose: a tiny presentational primitive; shows nothing when there is no
  error, and an assertive `role="alert"` message when there is.

  $props:
    - error (string | null) — the message to show; falsy ⇒ render nothing.
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
