<script lang="ts">
  import WardBoard from "$lib/components/WardBoard.svelte";

  let { data } = $props();
</script>

<WardBoard
  wardPid={data.board.ward_pid}
  initial={data.board}
  masked={data.masked}
/>
