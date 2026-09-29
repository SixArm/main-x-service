<script>
	import Willow from "./Willow.svelte";
	let { fonts = true, children } = $props();
</script>

{#if children}
	<Willow {fonts}>
		<div class="wx-bw-theme">
			{@render children()}
		</div>
	</Willow>
{:else}
	<Willow {fonts} />
{/if}

<style>
	.wx-bw-theme {
		height: 100%;
		width: 100%;
		box-sizing: border-box;

		/* base palette - black with gray shades for print legibility */
		--wx-background: #ffffff;
		--wx-background-alt: #ffffff;
		--wx-background-hover: transparent;
		--wx-color-font: #000000;
		--wx-color-font-alt: #555555;
		--wx-color-font-disabled: #888888;
		--wx-color-primary: #000000;
		--wx-color-primary-selected: transparent;
		--wx-color-primary-font: #000000;
		--wx-color-link: #000000;
		--wx-icon-color: #000000;

		/* kanban surfaces */
		--wx-kanban-bg: #ffffff;
		--wx-kanban-column-bg: #ffffff;
		--wx-kanban-column-over-limit-bg: #ffffff;
		--wx-kanban-card-bg: #ffffff;
		--wx-kanban-tag-bg: #ffffff;
		--wx-kanban-avatar-bg: #ffffff;

		/* kanban progress */
		--wx-kanban-progress-bg: #ffffff;
		--wx-kanban-progress-fill: #777777;

		/* kanban borders and shadows */
		--wx-kanban-border-color: #999999;
		--wx-kanban-card-shadow: none;
		--wx-kanban-card-shadow-hover: none;
		--wx-kanban-card-shadow-drag: none;
		--wx-kanban-drop-placeholder-bg: transparent;

		/* priority badges - gray/black borders instead of color fills */
		--wx-kanban-priority-low-bg: #ffffff;
		--wx-kanban-priority-low-color: #555555;
		--wx-kanban-priority-medium-bg: #ffffff;
		--wx-kanban-priority-medium-color: #333333;
		--wx-kanban-priority-high-bg: #ffffff;
		--wx-kanban-priority-high-color: #000000;
	}

	.wx-bw-theme :global(.wx-kanban),
	.wx-bw-theme :global(.wx-board),
	.wx-bw-theme :global(.wx-scroll),
	.wx-bw-theme :global(.wx-content) {
		background: #ffffff;
	}

	.wx-bw-theme :global(.wx-column) {
		background: #ffffff;
		border-right: 1px solid #fff;
		border-bottom: 1px solid #fff;
	}

	.wx-bw-theme :global(.wx-column-header),
	.wx-bw-theme :global(.wx-scroll-board .wx-column-header),
	.wx-bw-theme :global(.wx-over-limit .wx-column-header),
	.wx-bw-theme :global(.wx-over-limit .wx-body) {
		background: #ffffff;
	}

	.wx-bw-theme :global(.wx-over-limit.wx-column) {
		border-color: #000000;
	}

	.wx-bw-theme :global(.wx-card) {
		background: #ffffff;
		border: 1px solid var(--wx-kanban-border-color);
		box-shadow: none;
		break-inside: avoid;
	}

	.wx-bw-theme :global(.wx-card:hover) {
		box-shadow: none;
	}

	.wx-bw-theme :global(.wx-cover),
	.wx-bw-theme :global(.wx-image) {
		filter: grayscale(1) contrast(1.08);
	}

	.wx-bw-theme :global(.wx-cover) {
		box-shadow: inset 0 0 0 1px var(--wx-kanban-border-color);
	}

	.wx-bw-theme :global(.wx-priority),
	.wx-bw-theme :global(.wx-tag) {
		background: #ffffff;
		border: 1px solid currentColor;
	}

	.wx-bw-theme :global(.wx-card-priority-medium) {
		border-style: dashed;
	}

	.wx-bw-theme :global(.wx-card-priority-high) {
		border-width: 2px;
	}

	.wx-bw-theme :global(.wx-progress) {
		height: 6px;
		background: #ffffff;
		border: 1px solid #777777;
	}

	/* diagonal stripes so progress reads on paper without color */
	.wx-bw-theme :global(.wx-progress-fill) {
		background: repeating-linear-gradient(
			45deg,
			#777777 0,
			#777777 1px,
			transparent 1px,
			transparent 6px
		);
	}

	.wx-bw-theme :global(.wx-avatar) {
		background: #ffffff;
		border: 1px solid var(--wx-kanban-border-color);
	}

	.wx-bw-theme :global(.wx-drop-placeholder) {
		background: transparent;
	}

	.wx-bw-theme :global(.wx-add:hover),
	.wx-bw-theme :global(.wx-toggle:hover),
	.wx-bw-theme :global(.wx-expand:hover),
	.wx-bw-theme :global(.wx-menu:hover) {
		background: transparent;
	}
</style>
