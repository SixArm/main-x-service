<!--
  Tour (/tour) — a public, in-depth walkthrough of Case Tracking.
  Copy lives in the i18n catalog (`tour.*`); this file only declares which
  app screen each workflow section links to, in order.
-->
<script lang="ts">
    import Tour from '$lib/components/Tour.svelte';

    const sections = [
        { href: '/folders/new' },
        { href: '/folders' },
        { href: '/move' },
        { href: '/scan' },
        { href: '/volumes' },
        { href: '/history' },
    ];
</script>

<Tour {sections} />
