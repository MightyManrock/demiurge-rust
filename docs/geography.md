# Geography and the Heatmap Model

## Core Principle

Physical location is defined by heatmap layers rather than discrete tiles or authored nodes. Every point in a planetary system is described by overlapping continuous fields (elevation, temperature, precipitation, Domain expression, etc.). All location-dependent mechanics derive from sampling these fields.

`PopLocation` and `TravelLocation` as discrete node types are eliminated. Pops exist at coordinates, and movement is governed by navigability imposed by the heatmaps themselves.

## Pop Location and Density

A Pop's location is a center coordinate plus a **spread radius** and a mutable **density scalar**, not a single point.

- **High density**: the Pop is concentrated — settled agriculture, urban clusters.
- **Low density**: the Pop is dispersed — nomadic foragers, pastoralists splitting to cover ground.

Density is not directly player-controlled. It shifts as a consequence of cultural values and environmental pressure. Promoting `VirtueTag::Negative(VirtueTrait::Solidarity)` (i.e., Autonomy) through the Imāgō system makes Pops more inclined to increase their spread over time.

### Mechanical consequences of spread

A dispersed Pop samples the heatmaps across a wider area. This means:

- Exposure to more varied terrain and resource nodes.
- Exposure to multiple regional Domain gradients simultaneously.
- Increased cultural drift risk: divergent environmental pressures on different parts of the Pop drive divergent belief and trait development.
- A Pop spread thin for an extended period accumulates **cohesion pressure** toward splitting.

## Emergent Regions

Regions are not authored. They emerge from clustering on two axes:

1. **Terrain similarity** — elevation, temperature, precipitation, biome character.
2. **Domain expression** — the accumulated Domain profile of the area.

A region is a contiguous area where both dimensions are sufficiently homogeneous. Two physically similar areas (e.g., two grasslands) can be distinct regions if their Domain expression profiles differ significantly.

Regions grow, shrink, and are subsumed by neighboring regions as the underlying heatmaps evolve. They have no hard boundaries in the data — boundary is an emergent property of the clustering.

### Regional character has two timescales

- **Terrain-paced** (geological): driven by the underlying heatmap values. Slow and semi-permanent. Outlasts the Pops who shaped it — a region can carry the imprint of a long-extinct civilization in its terrain long after they are gone ("ghost regions").
- **History-paced** (cultural/political): driven by accumulated Pop presence, POI influence, and Demiurge intervention. Faster and more reversible.

These are presented as a single regional profile but should be tracked separately in the data model.

## Cross-Regional Pop Spread

Pops straddling a regional boundary is intentional and mechanically significant:

- The Pop experiences a weighted blend of both regions' Domain expression and terrain character, weighted by how much of the spread falls in each region.
- Divergent regional pulls reduce Pop cohesion, accelerating split risk.
- When a split occurs, each daughter Pop reinforces its respective region's character.
- Conversely, Pop activity and Domain beliefs **leak back into** regional profiles: physical actions (deforestation, agriculture, settlement) alter the underlying heatmaps; Domain belief concentration shifts the history-paced layer of regional character.

This bidirectional feedback — regions shaping Pops, Pops reshaping regions — is the core engine of cultural and geographic evolution in the simulation.

## Points of Interest (POIs)

POIs exist at coordinates within regions and influence regional character. A POI with strong Domain expression pushes the history-paced regional layer toward that Domain over time. See `underreal.md` and `essence-economy.md` for the Domain-pushing monolith as a specific Demiurge-created POI type.
