<!--
  Tour (/tour) — a public, in-depth walkthrough of the Course registry.
  Copy lives in the i18n catalog (`tour.*`); this file only declares which
  app screen each workflow section links to, in order.
-->
<script lang="ts">
    import Tour from "$lib/components/Tour.svelte";

    const sections = [
        { href: "/courses/new" },
        { href: "/courses" },
        { href: "/courses/match" },
        { href: "/courses/merge" },
        { href: "/board" },
        { href: "/calendar" },
    ];
</script>

<Tour {sections} />
