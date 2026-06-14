# Pop Mechanics — Design Notes

## Core Model: Amoeba-Like Movement

Pops are not points that teleport between locations. They are rooted to a coordinate but can **extrude** portions of themselves outward to search for or collect resources. If an extrusion reaches a location with significantly better conditions, it can pull the rest of the Pop toward it — a gradual drift rather than a discrete move.

This mirrors how pre-modern populations actually relocate: scouts range ahead, foragers find better ground, and the group follows over time rather than all at once.

## Extrusion

An extrusion is a temporary projection of the Pop's presence into an adjacent or nearby cell. It represents scouts, foragers, raiding parties, or seasonal migrants — the leading edge of the population's activity.

**Extrusion radius** determines how far a Pop can sense and reach. This is probably a function of:
- Species mobility (a nomadic pack-hunting species like the Keth would have a longer reach than a settled agrarian one)
- Technology tier (better tools and transport extend the effective radius)
- Terrain (movement costs vary by geography)

**What extrusions sense** is effectively the same signal as region suitability scoring: resource density, climate comfort, water access, danger. The existing `score_region_for_species` infrastructure is load-bearing here — not just a display metric.

## Migration Threshold

An extrusion triggers migration when the destination's conditions exceed the origin's by enough to justify the cost of moving. The threshold should account for:
- The suitability delta (how much better is it over there?)
- Distance (farther moves cost more)
- Pop size and cohesion (larger Pops may be slower to move)

A small persistent delta might cause slow drift; a large sudden delta (a disaster at home, a resource windfall ahead) could cause rapid relocation.

## Friction and Conflict

When two Pops' extrusions reach the same cell, the resulting overlap is where the interesting simulation emerges:
- **Competition**: both Pops want the same resource; one or both retract or are depleted
- **Conflict**: extrusions make contact and trigger an encounter
- **Merging**: compatible Pops (same species, same faction) may consolidate
- **Retraction**: if the contested cell isn't worth fighting for, the weaker extrusion pulls back

This friction is the natural emergence point for **Faction** and **Civilization** mechanics — those structures don't need to be designed top-down if the Pop-level interactions generate them organically.

## Open Questions

- Does a Pop maintain multiple simultaneous extrusions, or one at a time?
- What happens to the "extruded" portion while the main body remains — is it a separate entity, or a temporary state?
- How do extrusions interact with terrain that the Pop's species finds hostile (wrong atmosphere, temperature out of range)?
- At what scale does a "Pop" operate — a band of dozens, a tribe of hundreds, a city of thousands? This probably affects extrusion radius and migration threshold significantly.
