<div align="center">
	
# SVAR Svelte Kanban

</div>

<div align="center">

[Homepage](https://svar.dev/svelte/kanban/) • [Getting Started](https://docs.svar.dev/svelte/kanban/getting-started/quick-start/) • [Demos](https://docs.svar.dev/svelte/kanban/samples/)

</div>

<div align="center">

[![npm](https://img.shields.io/npm/v/@svar-ui/svelte-kanban.svg)](https://www.npmjs.com/package/@svar-ui/svelte-kanban)
[![License](https://img.shields.io/github/license/svar-widgets/kanban)](https://github.com/svar-widgets/kanban/blob/main/license.txt)
[![npm downloads](https://img.shields.io/npm/dm/@svar-ui/svelte-kanban.svg)](https://www.npmjs.com/package/@svar-ui/svelte-kanban)

</div>

[SVAR Svelte Kanban](https://svar.dev/svelte/kanban/) is a customizable, interactive Kanban board component for Svelte and SvelteKit apps. It supports drag-and-drop, card editing, filtering, sorting, flexible layouts, and rich customization options. Use it to add clear visualization of tasks and project workflows. 

The kanban component comes with full TypeScript support, rich API, and easy CSS styling. The PRO Edition offers extra features for enterprise projects (see below).

<div align="center">
<img src="https://svar.dev/images/github/github_kanban.gif" alt="SVAR Svelte Kanban Preview">
</div>

### ✨ Key Features

- Drag-and-drop cards between columns and rows
- Built-in card editor
- Context menu and toolbar
- Card filtering, sorting, grouping
- REST data provider for backend integration
- Custom card templates
- Localization
- Light and dark themes
- Full TypeScript support

### 🚀 PRO Edition

SVAR Svelte Kanban is available in open-source and [PRO Editions](https://svar.dev/svelte/kanban/#pro). The PRO Edition offers extra features for enterprise projects:

- Export to PDF/PNG/Excel
- Dynamic data loading
- Undo/redo

Visit the [pricing page](https://svar.dev/svelte/kanban/pricing/) for full feature comparison, licensing details, and **free trial**.

Or [see the live demo](https://svar.dev/demos/kanban/).

### 🛠️ How to Use

To use SVAR Svelte Kanban, simply import the package and include the component in your Svelte file:

```svelte
<script>
	import { Kanban } from "@svar-ui/svelte-kanban";

	const cards = [
		{ id: 1, label: "Design", column: "todo" },
		{ id: 2, label: "Implement", column: "doing" },
	];
	const columns = [
		{ id: "todo", label: "To Do" },
		{ id: "doing", label: "Doing" },
		{ id: "done", label: "Done" },
	];
</script>

<Kanban {cards} {columns} />
```

For further instructions, follow the detailed [quick start guide](https://docs.svar.dev/svelte/kanban/getting-started/quick-start/).

### How to Modify

Typically, you don't need to modify the code. However, if you wish to do so, follow these steps:

1. Install [vite-plus](https://vite.plus) (`curl -fsSL https://vite.plus | bash` on Mac/Linux, `irm https://vite.plus/ps1 | iex` on Windows). The project uses `pnpm` workspaces under the hood, so plain `npm` will not work.
2. Run `vp i` from the project root to install dependencies.
3. Run `vp run build` to build all packages.
4. Start the demo app in development mode with `vp run start`.

### Run Tests

To run the tests:

```sh
vp test
```

### ⭐ Show Your Support

If SVAR Svelte Kanban helps your project, [give us a star](https://github.com/svar-widgets/kanban/) to support us! 

### :speech_balloon: Need Help?

[Post an Issue](https://github.com/svar-widgets/kanban/issues/) or use our [community forum](https://forum.svar.dev).
