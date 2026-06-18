# Places

The physical **place hierarchy** for paper case-note folders — the nested
locations the system walks to answer _"where is the folder for NHS Number
`XXX XXX XXXX` right now?"_

> A case folder can be **on a shelf** or **in a cabinet**, in a **room**,
> on a **building floor**, in a **building**, on a **campus**.

## Containment hierarchy

From innermost (the folder) outward to the campus. A folder sits in
exactly one **leaf container** — a cabinet **or** a shelf — and that
container nests upward through the estate:

```
Case folder
  └─ in a Cabinet ┐
  └─ on a Shelf   ┴─ leaf container (exactly one)
        └─ in a Room
              └─ on a Floor          (building floor / storey)
                    └─ in a Building
                          └─ on a Campus   (top of the tree)
```

## Place levels

| Level        | Contained in | Notes                                                              |
| ------------ | ------------ | ------------------------------------------------------------------ |
| **Campus**   | — (root)     | A hospital site / campus; the top of the place tree.              |
| **Building** | Campus       | A named building on the campus.                                    |
| **Floor**    | Building     | A storey (building floor) within the building.                     |
| **Room**     | Floor        | A room on the floor.                                               |
| **Cabinet**  | Room         | A filing cabinet; a leaf container that holds folders (`capacity`).|
| **Shelf**    | Room         | A shelf; the alternative leaf container that holds folders.        |

A **case folder** — and a **volume** (a bundle of folders) — is located in
exactly one **leaf container** (a **Cabinet** or a **Shelf**), never
directly in a room, floor, building, or campus. The leaf container's parent
chain (room → floor → building → campus) yields the folder's full location,
and a move re-points the folder/volume to a new leaf container.

## Location path

Every folder location resolves to a human-readable **container path** that
spells out the full chain from campus down to the leaf, for example:

```
Riverside Campus › Tower Block › Floor 3 › Records Room 3B › Cabinet C-12
Riverside Campus › Tower Block › Floor 3 › Records Room 3B › Shelf S-04
```

The leaf container's label and the path are **snapshotted** onto folder and
move records at write time (see [domain-model.md](domain-model.md)), so the
audit trail survives an upstream rename, deletion, or outage.

## Relationship to the current model

Today the implemented place chain (see [domain-model.md](domain-model.md),
sourced from the Main Place Service) is **Building → Room → Cabinet**. This
spec defines the full physical estate, which extends that chain with three
levels:

- **Campus** — above Building (a building belongs to a campus).
- **Floor** — between Building and Room (a room is on a building floor).
- **Shelf** — a sibling leaf container to Cabinet (a folder may sit on a
  shelf rather than in a cabinet).

Folder/volume location, move events, and the "where is folder X?" query all
operate on the **leaf container** (cabinet or shelf); the remaining levels
are resolved by walking the parent chain upward to the campus.
