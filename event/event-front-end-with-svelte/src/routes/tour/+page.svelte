<!--
  Tour (/tour) — a public, in-depth walkthrough of the Event registry.
  Copy lives in the i18n catalog (`tour.*`); this file only declares which
  app screen each workflow section links to, in order.
-->
<script lang="ts">
    import Tour from "#lib/components/Tour.svelte";

    const sections = [
        { href: "/events/new" },
        { href: "/events" },
        { href: "/events/match" },
        { href: "/events/merge" },
        { href: "/calendar" },
        { href: "/events" },
    ];
</script>

<Tour {sections} />
