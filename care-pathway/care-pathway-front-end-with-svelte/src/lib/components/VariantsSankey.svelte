<!--
  Journey variants as a Sankey diagram (spec T-14c/T-14l): one column
  per position in the pathway string, node height proportional to how
  many instances reach that (position, stage) pair, and every flow
  eventually reaching the synthetic Stopped terminal — the same
  convention the sunburst uses, drawn as a second, complementary view
  of the same variants payload (a Sankey reads flow *between*
  positions more easily than a sunburst does).
-->
<script lang="ts">
  import type { VariantsReport } from "$lib/api/tba";
  import { STOPPED, sankeyFromVariants } from "$lib/analytics-transforms";

  interface Props {
    report: VariantsReport;
  }
  let { report }: Props = $props();

  const graph = $derived(sankeyFromVariants(report));
  const maxPosition = $derived(
    Math.max(0, ...graph.nodes.map((n) => n.position)),
  );

  const COLUMN_WIDTH = 140;
  const NODE_WIDTH = 18;
  const PIXELS_PER_INSTANCE = 6;
  const MARGIN = 20;

  interface Placed {
    id: string;
    label: string;
    position: number;
    value: number;
    x: number;
    y: number;
    height: number;
  }

  const placed = $derived.by((): Map<string, Placed> => {
    const map = new Map<string, Placed>();
    for (let position = 0; position <= maxPosition; position += 1) {
      let y = MARGIN;
      const atPosition = graph.nodes
        .filter((n) => n.position === position)
        .sort((a, b) =>
          a.label === STOPPED
            ? 1
            : b.label === STOPPED
              ? -1
              : a.label.localeCompare(b.label),
        );
      for (const node of atPosition) {
        const nodeHeight = Math.max(4, node.value * PIXELS_PER_INSTANCE);
        map.set(node.id, {
          id: node.id,
          label: node.label,
          position: node.position,
          value: node.value,
          x: MARGIN + position * COLUMN_WIDTH,
          y,
          height: nodeHeight,
        });
        y += nodeHeight + 6;
      }
    }
    return map;
  });

  const height = $derived(
    Math.max(
      120,
      MARGIN * 2 +
        Math.max(
          ...Array.from({ length: maxPosition + 1 }, (_, position) =>
            [...placed.values()]
              .filter((n) => n.position === position)
              .reduce((sum, n) => sum + n.height + 6, 0),
          ),
          0,
        ),
    ),
  );
  const width = $derived(MARGIN * 2 + (maxPosition + 1) * COLUMN_WIDTH);

  function linkPath(source: Placed, target: Placed): string {
    const x1 = source.x + NODE_WIDTH;
    const x2 = target.x;
    const y1 = source.y + source.height / 2;
    const y2 = target.y + target.height / 2;
    const midX = (x1 + x2) / 2;
    return `M ${x1} ${y1} C ${midX} ${y1}, ${midX} ${y2}, ${x2} ${y2}`;
  }
</script>

<div class="sankey-wrap" data-testid="variants-sankey">
  <svg
    viewBox={`0 0 ${width} ${height}`}
    role="img"
    aria-label="Journey variants Sankey diagram"
  >
    {#each graph.links as link (link.source + "->" + link.target)}
      {@const source = placed.get(link.source)}
      {@const target = placed.get(link.target)}
      {#if source && target}
        <path
          d={linkPath(source, target)}
          class="link"
          style={`stroke-width: ${Math.max(1, link.value * PIXELS_PER_INSTANCE)}`}
        />
      {/if}
    {/each}
    {#each [...placed.values()] as node (node.id)}
      <rect
        x={node.x}
        y={node.y}
        width={NODE_WIDTH}
        height={node.height}
        class="node"
        class:stopped={node.label === STOPPED}
      >
        <title
          >{node.label} — {node.value} instance{node.value === 1
            ? ""
            : "s"}</title
        >
      </rect>
      <text
        x={node.x + NODE_WIDTH + 4}
        y={node.y + node.height / 2}
        class="node-label"
      >
        {node.label}
      </text>
    {/each}
  </svg>
</div>

<style>
  .sankey-wrap {
    overflow-x: auto;
    margin: 1rem 0;
  }
  svg {
    display: block;
    min-width: 100%;
  }
  .node {
    fill: rgb(60 120 200 / 0.6);
  }
  .node.stopped {
    fill: rgb(128 128 128 / 0.5);
  }
  .node-label {
    font-size: 0.7rem;
    dominant-baseline: middle;
    fill: currentColor;
  }
  .link {
    fill: none;
    stroke: rgb(60 120 200 / 0.25);
  }
</style>
