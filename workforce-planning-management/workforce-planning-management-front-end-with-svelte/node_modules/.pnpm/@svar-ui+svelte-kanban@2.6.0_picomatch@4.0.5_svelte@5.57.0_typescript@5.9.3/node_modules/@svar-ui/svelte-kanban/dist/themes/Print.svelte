<script>
	import Willow from "./Willow.svelte";
	let { fonts = true, children } = $props();
</script>

{#if children}
	<Willow {fonts}>
		<div class="wx-print-theme">
			{@render children()}
		</div>
	</Willow>
{:else}
	<Willow {fonts} />
{/if}

<style>
	.wx-print-theme {
		height: 100%;
		width: 100%;
		box-sizing: border-box;

		/* surfaces - strip filled backgrounds, keep neutral page */
		--wx-background: #ffffff;
		--wx-background-alt: #ffffff;
		--wx-background-hover: transparent;
		--wx-color-primary: #1f6bd9;
		--wx-color-primary-selected: transparent;
		--wx-color-font-alt: #555d68;
		--wx-icon-color: #555d68;

		/* kanban surfaces */
		--wx-kanban-bg: #ffffff;
		--wx-kanban-column-bg: #ffffff;
		--wx-kanban-column-over-limit-bg: #ffffff;
		--wx-kanban-card-bg: #ffffff;
		--wx-kanban-tag-bg: #ffffff;
		--wx-kanban-avatar-bg: #ffffff;

		/* kanban progress */
		--wx-kanban-progress-bg: #ffffff;
		--wx-kanban-progress-fill: #1f6bd9;

		/* kanban borders and shadows */
		--wx-kanban-border-color: #b8bec8;
		--wx-kanban-card-shadow: none;
		--wx-kanban-card-shadow-hover: none;
		--wx-kanban-card-shadow-drag: none;
		--wx-kanban-drop-placeholder-bg: transparent;

		/* priority badges - keep color as text/border only */
		--wx-kanban-priority-low-bg: #ffffff;
		--wx-kanban-priority-low-color: #166534;
		--wx-kanban-priority-medium-bg: #ffffff;
		--wx-kanban-priority-medium-color: #92400e;
		--wx-kanban-priority-high-bg: #ffffff;
		--wx-kanban-priority-high-color: #991b1b;
	}

	.wx-print-theme :global(.wx-kanban),
	.wx-print-theme :global(.wx-board),
	.wx-print-theme :global(.wx-scroll),
	.wx-print-theme :global(.wx-content) {
		background: #ffffff;
	}

	.wx-print-theme :global(.wx-column) {
		background: #ffffff;
		border-right: 1px solid #fff;
		border-bottom: 1px solid #fff;
	}

	.wx-print-theme :global(.wx-column-header),
	.wx-print-theme :global(.wx-scroll-board .wx-column-header),
	.wx-print-theme :global(.wx-over-limit .wx-column-header),
	.wx-print-theme :global(.wx-over-limit .wx-body) {
		background: #ffffff;
	}

	.wx-print-theme :global(.wx-over-limit.wx-column) {
		border-color: var(--wx-color-danger);
	}

	.wx-print-theme :global(.wx-card) {
		background: #ffffff;
		border: 1px solid var(--wx-kanban-border-color);
		box-shadow: none;
		break-inside: avoid;
	}

	.wx-print-theme :global(.wx-card:hover) {
		box-shadow: none;
	}

	.wx-print-theme :global(.wx-cover) {
		box-shadow: inset 0 0 0 1px var(--wx-kanban-border-color);
		filter: saturate(0.75) contrast(1.05);
	}

	.wx-print-theme :global(.wx-priority),
	.wx-print-theme :global(.wx-tag) {
		background: #ffffff;
		border: 1px solid currentColor;
	}

	.wx-print-theme :global(.wx-progress) {
		height: 6px;
		background: #ffffff;
		border: 1px solid #6e777d;
	}

	.wx-print-theme :global(.wx-avatar) {
		background: #ffffff;
		border: 1px solid var(--wx-kanban-border-color);
	}

	.wx-print-theme :global(.wx-drop-placeholder) {
		background: transparent;
	}

	.wx-print-theme :global(.wx-add:hover),
	.wx-print-theme :global(.wx-toggle:hover),
	.wx-print-theme :global(.wx-expand:hover),
	.wx-print-theme :global(.wx-menu:hover) {
		background: transparent;
	}
</style>
