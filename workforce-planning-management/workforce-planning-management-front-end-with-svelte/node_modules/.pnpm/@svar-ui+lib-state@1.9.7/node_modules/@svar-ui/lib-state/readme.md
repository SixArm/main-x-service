# @wx/lib-state

Framework-agnostic reactive state management with event processing.

![NPM package](https://img.shields.io/npm/v/@svar-ui/lib-state)

## Installation

```
npm install @wx/lib-state
```

## Core Modules

### Store

Reactive state container. Wraps values into subscribable observables via a framework adapter (`writable`). [Spec](.specs/store.md)

```ts
const store = new Store({ writable, async: false });
store.setState({ count: 0, name: "Alice" });

const reactive = store.getReactive();
reactive.count.subscribe(v => console.log(v));

store.getState(); // { count: 0, name: "Alice" }
```

| Method                  | Returns         |
| ----------------------- | --------------- |
| `setState(data, mode?)` | `TAsyncSignals` |
| `getState()`            | `T`             |
| `getReactive()`         | `TState<T>`     |

### DataRouter

Adds computed property recalculation and input normalization on top of Store.

```ts
const router = new DataRouter(store, routes, parsers);
router.init({ items: [1, 2, 3], filter: null });
router.setState({ filter: v => v > 2 });
```

| Method                    | Returns         |
| ------------------------- | --------------- |
| `init(cfg)`               | `void`          |
| `setState(updates, ctx?)` | `TAsyncSignals` |
| `setStateAsync(updates)`  | `TAsyncSignals` |

Computed dependencies are declared as `TDataBlock` objects with `in`, `out`, and `exec` fields. DataRouter resolves execution order by dependency depth automatically.

### EventBus

Chainable event processing pipeline. Handlers registered via `on()` run in order; `intercept()` prepends a handler. Return `false` from a handler to stop propagation. [Spec](.specs/events.md)

```ts
const bus = new EventBus();
bus.on("update-task", ev => {
	/* handle */
});
bus.intercept("update-task", ev => {
	if (!ev.valid) return false; // cancel
});
await bus.exec("update-task", { id: 1 });
```

| Method                              | Returns               |
| ----------------------------------- | --------------------- |
| `on(name, handler, config?)`        | `void`                |
| `intercept(name, handler, config?)` | `void`                |
| `detach(tag)`                       | `void`                |
| `exec(name, ev)`                    | `Promise<T[keyof T]>` |
| `setNext(next)`                     | `IEventBus<T>`        |

Chain links via `setNext()` - any object with `exec()` and `setNext()` qualifies.

### EventBusRouter

Adapts a plain dispatch function into an event chain link. Always forwards (can't stop propagation).

```ts
const router = new EventBusRouter((name, data) => store.dispatch(name, data));
bus.setNext(router);
```

### EventResolver

Calls a callback embedded in the event object when the event reaches this chain link. Used for optimistic update confirmation flows.

```ts
const resolver = new EventResolver("resolve");
router.setNext(resolver);

await bus.exec("add-task", {
	task: { name: "Design" },
	resolve: ev => {
		/* server confirmed */
	},
});
```

## Data Collections

### DataArray

Flat indexed collection with O(1) ID lookup. [Spec](.specs/data.md)

```ts
const items = new DataArray([
	{ id: 1, name: "A" },
	{ id: 2, name: "B" },
]);
items.byId(1); // { id: 1, name: "A" }
items.add({ name: "C" }); // auto-assigns ID
items.update(1, { name: "A+" });
items.remove(2);
```

| Method               | Returns |
| -------------------- | ------- |
| `add(item)`          | `void`  |
| `update(id, values)` | `void`  |
| `remove(id)`         | `void`  |
| `filter(predicate)`  | `void`  |
| `byId(id)`           | `T`     |
| `map(cb)`            | `D[]`   |
| `forEach(cb)`        | `void`  |

### DataTree

Hierarchical indexed collection. Extends flat operations with tree traversal, branch management, and nesting levels. [Spec](.specs/data.md)

```ts
const tree = new DataTree([
	{ id: 1, parent: 0, text: "Root" },
	{ id: 2, parent: 1, text: "Child" },
]);
tree.byId(2); // { id: 2, parent: 1, $level: 2, ... }
tree.getBranch(1); // [node2]
tree.toArray(); // flattened by open/closed state
tree.move(2, "after", 1);
```

| Method                    | Returns |
| ------------------------- | ------- |
| `parse(items, parent)`    | `void`  |
| `add(item, index)`        | `void`  |
| `addAfter(item, afterId)` | `void`  |
| `remove(id)`              | `void`  |
| `update(id, values)`      | `void`  |
| `move(id, mode, target)`  | `void`  |
| `toArray()`               | `T[]`   |
| `byId(id)`                | `T`     |
| `getBranch(id)`           | `T[]`   |
| `forEach(cb)`             | `void`  |
| `eachChild(cb, parent)`   | `void`  |

## Helpers [Spec](.specs/helpers.md)

| Function                     | Purpose                                              |
| ---------------------------- | ---------------------------------------------------- |
| `uid()`                      | Sequential numeric ID                                |
| `tempID()`                   | Temporary string ID (`"temp://..."`)                 |
| `isTempID(v)`                | Check if value is a temp ID                          |
| `isSame(a, b)`               | Deep structural equality                             |
| `deepCopy(obj)`              | Deep clone (primitives, Date, arrays, plain objects) |
| `markReactive(obj, nested?)` | Tag object for nested reactive wrapping in Store     |

## License

MIT
