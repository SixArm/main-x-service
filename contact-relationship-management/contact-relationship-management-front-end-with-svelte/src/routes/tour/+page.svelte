<!--
  Tour (/tour) — a public, in-depth walkthrough of the CRM. Copy lives in
  the i18n catalog (`tour.*`); this file only declares which app screen
  each workflow section links to, in order.
-->
<script lang="ts">
  import Tour from "#lib/components/Tour.svelte";

  const sections = [
    { href: "/contacts" },
    { href: "/leads" },
    { href: "/deals" },
    { href: "/campaigns" },
    { href: "/tickets" },
    { href: "/executive" },
  ];
</script>

<Tour {sections} />
