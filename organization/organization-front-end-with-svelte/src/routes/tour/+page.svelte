<!--
  Tour (/tour) — a public, in-depth walkthrough of the Organizations
  registry. Copy lives in the i18n catalog (`tour.*`); this file only
  declares which app screen each workflow section links to, in order.
-->
<script lang="ts">
  import Tour from "$lib/components/Tour.svelte";

  const sections = [
    { href: "/new" },
    { href: "/organizations" },
    { href: "/organizations" },
    { href: "/organizations" },
    { href: "/review" },
    { href: "/merge" },
  ];
</script>

<Tour {sections} />
