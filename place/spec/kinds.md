# Kinds of Place

A Place ([schema.org/Place](https://schema.org/Place)) can represent many
**kinds** of location, spanning scales from a single storage shelf up to an
administrative region. A place's kind is carried by its classification
(`place_type` → `category`, see [§5 Domain Model](05-domain-model.md)), and
places nest through the containment hierarchy
(`contained_in_place` / `contains_place`).

## Some kinds of place

Ordered roughly smallest (most contained) to largest (most containing):

| Kind                      | Scale                  | Notes                                                         |
| ------------------------- | ---------------------- | ------------------------------------------------------------- |
| **Shelf on a rack**       | storage unit           | A single shelf within a rack; a leaf storage location.        |
| **Filing cabinet drawer** | storage unit           | A drawer within a filing cabinet; a leaf storage location.    |
| **Drawer**                | storage unit           | A drawer (e.g. in a desk or cabinet); a leaf storage location.|
| **Closet**                | storage unit           | A small enclosed storage space / cupboard.                    |
| **Desk**                  | furniture              | A work surface a folder may rest on; a leaf location.         |
| **Table**                 | furniture              | A work surface a folder may rest on; a leaf location.         |
| **Room**                  | interior space         | A room within a floor / building.                             |
| **Library**               | facility               | A room or building used to store and lend materials.          |
| **Floor of a building**   | building subdivision   | A storey within a building.                                   |
| **Wing of a building**    | building subdivision   | A wing / section of a building.                               |
| **Building**              | structure              | A whole building.                                             |
| **Campus**                | site                   | A grouped set of buildings on one site.                       |
| **Town**                  | settlement             | A populated settlement.                                       |
| **City**                  | settlement             | A larger populated settlement.                                |
| **County**                | administrative region  | An administrative subdivision of a province / nation.         |
| **Province**              | administrative region  | A larger administrative region.                               |

## Containment

These kinds nest, smallest within largest, via the place hierarchy
(`contained_in_place` / `contains_place`; cycles are rejected):

```
Shelf on a rack │ Filing cabinet drawer │ Drawer │ Closet │ Desk │ Table   (leaf storage / furniture)
      └─ Room │ Library
            └─ Floor of a building │ Wing of a building
                  └─ Building
                        └─ Campus
                              └─ Town │ City
                                    └─ County
                                          └─ Province
```

Containment is not strict — a place may be contained directly in any larger
kind (e.g. a building directly in a city, with no campus). The hierarchy
records whatever parent links actually hold; this list only fixes the
**relative scale** of each kind.

## Facility & civic kinds

The scale list above orders kinds by **size / containment**. A place also
has a **function** — what it is *for* — which is the other axis. Functional
("facility" / civic) kinds are classified by `place_type` (and map to a
[schema.org/Place](https://schema.org/Place) subtype); they typically occupy
the building / site scale and nest within a settlement (town / city).

| Kind                   | Classification (`place_type` → schema.org)      | Notes                                          |
| ---------------------- | ----------------------------------------------- | ---------------------------------------------- |
| **Hospital**           | `Hospital` → Hospital                           | A hospital — a building or a whole campus.     |
| **Clinic**             | `Hospital` → MedicalClinic                      | An outpatient clinic; a room, suite, or building. |
| **Fire department**    | `CivicStructure` → FireStation                  | A fire station.                                |
| **Police department**  | `CivicStructure` → PoliceStation                | A police station.                              |
| **Train station**      | `CivicStructure` → TrainStation                 | A railway station.                             |
| **Airport**            | `CivicStructure` → Airport                      | An airport (itself a site containing buildings). |
| **Bus stop**           | `CivicStructure` → BusStop                      | A bus stop — typically a single point.         |

…and so on — this list is **open-ended**. Any schema.org/Place subtype
(school, library, pharmacy, place of worship, park, …) is a valid kind;
record the function in `place_type` / `category` and the size/nesting via the
containment hierarchy. The two axes are independent: e.g. a **hospital** is
*function = Hospital*, *scale = building or campus*.

## Relationship to the model

- **Classification.** Each kind maps onto the place classification
  (`place_type` enum → `category`, [§5 Domain Model](05-domain-model.md)).
  This list is the human-facing catalogue of kinds; the enum/category values
  are the machine-facing encoding. New kinds are added by extending the
  classification (a [§13 Tasks](13-tasks.md) item) and recording the mapping
  here.
- **Hierarchy.** Kind fixes a place's typical scale; the actual parent/child
  relationships are recorded per place via `contained_in_place` /
  `contains_place`.
- **Matching.** Two places of different kinds at very different scales (a
  drawer vs a county) should not match; kind/category is one signal the
  matcher weighs (see the matcher spec).
