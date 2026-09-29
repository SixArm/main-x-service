<!--
  Tour (/tour) — a public, in-depth walkthrough of the Thing registry.
  Copy lives in the i18n catalog (`tour.*`); this file only declares which
  app screen each workflow section links to, in order.
-->
<script lang="ts">
    import Tour from "$lib/components/Tour.svelte";

    const sections = [
        { href: "/things/new" },
        { href: "/things" },
        { href: "/things/match" },
        { href: "/review" },
        { href: "/things/merge" },
        { href: "/things" },
    ];
</script>

<Tour {sections} />
