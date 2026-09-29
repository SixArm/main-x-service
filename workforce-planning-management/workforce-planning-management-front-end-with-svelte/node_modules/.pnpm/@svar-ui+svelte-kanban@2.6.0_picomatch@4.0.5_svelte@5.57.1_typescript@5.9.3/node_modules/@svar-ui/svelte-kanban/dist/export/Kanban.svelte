<script>
	import Willow from "../themes/Willow.svelte";
	import WillowDark from "../themes/WillowDark.svelte";
	import Print from "../themes/Print.svelte";
	import PrintBW from "../themes/PrintBW.svelte";
	import Kanban from "../components/Kanban.svelte";
	import Card from "./Card.svelte";

	const { skin, config = {}, htmlMode } = $props();
	const cardContent = $derived(htmlMode ? Card : undefined);

</script>

{#if skin === "willow-dark" || skin === "dark"}
	<WillowDark>
		<Kanban {...config} {cardContent} />
	</WillowDark>
{:else if skin === "print"}
	<Print>
		<Kanban {...config} {cardContent} />
	</Print>
{:else if skin === "bw"}
	<PrintBW>
		<Kanban {...config} {cardContent} />
	</PrintBW>
{:else}
	<Willow>
		<Kanban {...config} {cardContent} />
	</Willow>
{/if}
