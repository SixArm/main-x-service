<!--
  Tour (/tour) — a public, in-depth walkthrough of the Cases registry.
  Copy lives in the i18n catalog (`tour.*`); this file only declares which
  app screen each workflow section links to, in order.
-->
<script lang="ts">
  import Tour from "#lib/components/Tour.svelte";

  const sections = [
    { href: "/new" },
    { href: "/" },
    { href: "/board" },
    { href: "/merge" },
    { href: "/cases" },
    { href: "/audit" },
  ];
</script>

<Tour {sections} />
