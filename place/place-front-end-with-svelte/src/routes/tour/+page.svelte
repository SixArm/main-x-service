<!--
  Tour (/tour) — a public, in-depth walkthrough of the Place registry.
  Copy lives in the i18n catalog (`tour.*`); this file only declares which
  app screen each workflow section links to, in order.
-->
<script lang="ts">
    import Tour from "#lib/components/Tour.svelte";

    const sections = [
        { href: "/places/new" },
        { href: "/places" },
        { href: "/places/match" },
        { href: "/review" },
        { href: "/places/merge" },
        { href: "/places" },
    ];
</script>

<Tour {sections} />
