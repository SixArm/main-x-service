<!--
  Tour (/tour) — a public, in-depth walkthrough of the Person registry.
  Copy lives in the i18n catalog (`tour.*`); this file only declares which
  app screen each workflow section links to, in order.
-->
<script lang="ts">
    import Tour from "$lib/components/Tour.svelte";

    const sections = [
        { href: "/persons/new" },
        { href: "/persons" },
        { href: "/persons/match" },
        { href: "/review" },
        { href: "/persons/bulk" },
        { href: "/persons" },
    ];
</script>

<Tour {sections} />
