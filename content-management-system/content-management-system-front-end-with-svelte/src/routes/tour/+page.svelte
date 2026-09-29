<!--
  Tour (/tour) — a public, in-depth walkthrough of the CMS. Copy lives in
  the i18n catalog (`tour.*`); this file only declares which app screen
  each workflow section links to, in order.
-->
<script lang="ts">
  import Tour from "$lib/components/Tour.svelte";

  const sections = [
    { href: "/entries" },
    { href: "/entries" },
    { href: "/entries" },
    { href: "/workflow" },
    { href: "/translations" },
    { href: "/assets" },
  ];
</script>

<Tour {sections} />
