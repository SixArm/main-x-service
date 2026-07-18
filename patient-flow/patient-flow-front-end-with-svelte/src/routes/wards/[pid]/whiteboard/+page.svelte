<script lang="ts">
  import WardBoard from "$lib/components/WardBoard.svelte";

  let { data } = $props();
</script>

<h1>{data.board.ward_name} — whiteboard</h1>
<WardBoard wardPid={data.board.ward_pid} initial={data.board} />
