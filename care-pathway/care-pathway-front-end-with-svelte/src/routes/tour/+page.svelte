<!--
  Tour (/tour) — a public, in-depth walkthrough of the Care Pathways
  registry. Copy lives in the i18n catalog (`tour.*`); this file only
  declares which app screen each workflow section links to, in order.
-->
<script lang="ts">
    import Tour from "#lib/components/Tour.svelte";

    const sections = [
        { href: "/new" },
        { href: "/" },
        { href: "/" },
        { href: "/board" },
        { href: "/time" },
        { href: "/bulk" },
    ];
</script>

<Tour {sections} />
