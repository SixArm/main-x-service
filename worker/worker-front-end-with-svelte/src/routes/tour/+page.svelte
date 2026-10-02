<!--
  Tour (/tour) — a public, in-depth walkthrough of the Worker registry.
  Copy lives in the i18n catalog (`tour.*`); this file only declares which
  app screen each workflow section links to, in order.
-->
<script lang="ts">
    import Tour from "#lib/components/Tour.svelte";

    const sections = [
        { href: "/workers/new" },
        { href: "/workers" },
        { href: "/workers/match" },
        { href: "/review" },
        { href: "/expiry" },
        { href: "/workers" },
    ];
</script>

<Tour {sections} />
