# Pop Redesign — Design Spec
*2026-06-16*

## Context

The original Pop model defined a Pop by species + civilization + social stratum + a single occupation + domain beliefs + cultural traits. That model was designed for a simulation where Civilizations were the primary societal container and social class was a first-class identity axis.

Three changes to the broader design motivate a rethink:

1. **Civs as institutional containers** — Civs are now better understood as containers for institutions (Religion, Govs, Factions, etc.) rather than as monolithic cultural blocs with intrinsic class structures.
2. **Amoeba-style Pop movement** — Pops are no longer point entities that teleport; they extrude portions of themselves into adjacent terrain and travel networks.
3. **Map and location mechanics** — Geographic context (terrain, resources, travel networks) is now load-bearing for what a Pop can do and become.

---

## Core Changes

### 1. Occupation as Distribution

A Pop no longer has a single occupation field. Instead it carries two paired `HashMap<Occupation, f32>` fields representing its **full** state and its **current** (at-home) state:

```
full_occupations:    HashMap<Occupation, f32>  // stable identity; sums to 1.0
current_occupations: HashMap<Occupation, f32>  // what's physically present at core
```

The difference between `full_occupations` and `current_occupations` represents the occupation character currently out on extrusions.

**What it drives:**
- What resources a Pop can collect and how efficiently
- What kinds of extrusions it can send and with what composition
- What actions/behaviors are available to the Pop

### 2. Size as Dual Field

Similarly, Pop size splits into two paired fields:

```
full_size:    f32  // total population including all extruded portions
current_size: f32  // population physically present at core location
```

`full_size - current_size` = the total population currently distributed across active extrusions.

`full_size` changes through growth, attrition, budding, and splinting. `current_size` fluctuates constantly as extrusions depart and return.

### 3. Stratum Removed from Pop

Social stratum is no longer an inherent property of a Pop. Instead, stratum is an **institutional derivation**: Govs (and other Civ institutions) map occupation distributions onto social hierarchies based on their own structure and values.

This allows emergent class structures that don't need to be pre-specified:
- A Civ with Confucian-influenced institutions might rank `producer` Pops above `merchant` Pops despite merchants' economic power
- A militocratic Gov might elevate a `bonded` Pop that primarily functions as `soldier` to high status (cf. Janissaries)
- The same occupation distribution can carry very different social status under different institutional regimes

The Demiurge can observe the gap between a Pop's self-perceived identity (beliefs, culture tags) and the stratum assigned to it by governing institutions. That gap is a potential lever for indirect influence.

---

## Extrusion System

### Extrusion as Separate Entity

An `Extrusion` is a lightweight struct representing a temporary projection of a Pop's presence into the world. It is not a Pop — it has no independent beliefs, no culture tags, no notable mortals. It is a portion of the parent Pop that is currently elsewhere.

**Extrusion fields (tentative):**
```
parent_pop_id:  Uuid
size:           f32                       // bled from parent's current_size
occupations:    HashMap<Occupation, f32>  // bled from parent's current_occupations
location:       Uuid                      // current cell or travel-network node
purpose:        ExtrusionPurpose          // Foraging, Scouting, Trading, Raiding, Colonizing, ...
status:         ExtrusionStatus           // Outbound, Arrived, Returning, Lost
```

When an extrusion departs:
- `parent.current_size -= extrusion.size`
- `parent.current_occupations` adjusted to remove the bled portion

When an extrusion returns successfully:
- `parent.current_size += extrusion.size` (plus any resources collected)
- `parent.current_occupations` restored

When an extrusion is lost (hostile terrain, conflict, disaster):
- `parent.full_size -= extrusion.size`
- `parent.full_occupations` renormalized

### Extrusion Composition

Extrusions are **prescribed in purpose** but carry **noise**. A foraging extrusion is composed primarily of `forager` and `hunter` occupation weight — but at lower civilizational scales in particular, a small random proportion of other occupation types tags along (2–5% clergy, laborers, etc. are realistic). This noise:

- Decreases with civilizational scale and tech level (a planetary-scale Civ can send precisely composed survey teams)
- Seeds occupation diversity in any settlement that buds from the extrusion
- Is generally too small to meaningfully change the extrusion's behavior, but can matter over long timescales

### Extrusion Reach

What locations a Pop can extrude into is determined by:
- **Species mobility** (Keth pack-hunters range farther than settled agrarians)
- **Terrain costs** (mountains, ocean, hostile atmosphere impose friction)
- **Travel networks** (a port opens ocean reach; a shuttle service extends reach to orbital infrastructure; a travel network chain can extend extrusion reach across multiple worlds)

A Pop with sailing capability can extrude across an ocean because the port opens a travel network that reaches those cells. This replaces the old "Linked Pops" concept: there is no need to maintain a copy of a Pop at a remote location if the Pop can simply extrude there through the travel network.

### Discovery Layer

Extrusions may encounter resources that the Pop has **no current capacity to exploit**. These are recorded as known-but-inaccessible discoveries on the Pop. When the Civ later unlocks the relevant technology (surface mining, metallurgy, etc.), the Pop gains the *capacity* to shift occupation weight toward exploiting that resource — the discovery converts from latent knowledge into an actionable opportunity.

Example: foraging extrusions encounter surface copper deposits. Nothing happens yet. When the Civ unlocks basic metallurgy, the Pop can begin training miners and smiths as part of its occupation distribution and send targeted extrusions to the copper site.

---

## Pop Budding

### Emergent Budding

When an extrusion reaches a location and becomes sufficiently dense and self-sustaining over time, it crosses a **permanence threshold** and is promoted to a full Pop (a **bud**). The bud:

- Inherits the extrusion's size and occupation distribution as its `full_size` and `full_occupations`
- Starts with beliefs and culture tags inherited from the parent (with some drift from the extrusion's compositional noise)
- Reduces the parent's `full_size` and `full_occupations` by the budded amount
- Maintains an initial cooperative relationship with the parent (supply lines, resource exchange)

The permanence threshold is a function of:
- Extrusion density at the location
- Resource availability at the location (can it sustain a permanent presence?)
- Logistics capacity (can the parent maintain a supply relationship?)
- Time (durable presence matters)

Emergent buds tend to produce **cooperative sibling pairs**: the parent and bud are ideologically similar (shared beliefs, shared culture) but occupationally specialized for their respective geographic contexts.

### Directed Extrusions and Directed Buds

Notable Mortals and Factions with sufficient authority can **bypass the emergent threshold** and direct a Pop to send a prescribed extrusion or immediately formalize a bud.

**Authority scoping**: the authority to direct is scoped to the actor's institutional role.
- A warchief can direct warriors and hunters on a raid; they cannot compel the clergy to join without additional religious authority
- A militarist Faction can direct garrison placements; a mercantile Faction is better positioned to direct trade extrusions
- A religious Notable Mortal might direct a missionary extrusion toward a specific location

**Examples:**
- *Asha Keln, warchief of Taem's Oasis*, decides to peel off a band of warriors and hunters for a plains raid. He has the authority to prescribe the occupational composition (heavy `soldier`/`hunter`, minimal noise) and dispatch the extrusion immediately, bypassing the Pop's natural extrusion logic for that tick.
- *The "turtle" faction of the Hiparunites* decides to place a permanent garrison at Hiparun's Rift and directs the core Pop to keep it supplied. This is a directed bud: the garrison Pop is formalized immediately at the chosen location with a prescribed military occupation profile, and a directed supply relationship is established with the parent.

Directed extrusions and buds differ from emergent ones only in their **trigger** — their mechanics (size bleed, occupation bleed, cooperative relationship) are the same.

### Bud Dissolution

A bud relationship can dissolve:
- **Naturally**: if the resource that motivated the bud runs out, or logistics capacity collapses, the bud may retract — its population drifting back toward the parent
- **By decision**: a Gov or Notable Mortal can direct the dissolution of a garrison, colony, or outpost (the reverse of directed budding)
- **By loss**: the bud Pop is destroyed by hostile action or environmental catastrophe

---

## Pop Splinting (Largely Unchanged)

Splinting remains belief-divergence-driven. When a Pop's belief profile diverges far enough from the Civ's `established_beliefs`, a faction breaks away as a splinter Pop.

Key difference from budding: splint siblings tend toward **similar occupation distributions but divergent beliefs**. They are not cooperative — they may be rivals or outright hostile, competing for the same economic niche.

The splint mechanic from the Python implementation (divergence threshold, probabilistic trigger, size transfer, identity anchor, reabsorption when beliefs reconverge) carries over largely intact. The main adaptation is that splint eligibility no longer requires matching stratum+occupation (since stratum is removed), only species and sufficient size.

---

## Demiurge Agency

The Demiurge has **no direct control** over Pop budding, splinting, or extrusion behavior. Influence is indirect:

- **Whisper to a Notable Mortal** with authority over a Pop → that mortal may direct extrusions or buds in response
- **Support a Faction** that advocates for expansion or consolidation → Faction gains institutional influence → Govs act → Pops move
- **Shape beliefs** toward domain values that favor certain occupation shifts (a Pop deeply committed to a domain of industry may shift occupation weight toward crafting and building)

This is consistent with the broader design principle: the Demiurge acts through people and institutions, not by puppeting simulation entities directly.

---

## Open Questions

- **Extrusion granularity**: Can a Pop maintain many simultaneous extrusions, or is there a practical limit? (Probably constrained by `current_size` — a Pop can't extrude more than it has.)
- **Bud cooperative relationship**: Should bud-sibling bonds be explicit data (a `BudBond` struct tracking resource flow and bond strength) or inferred from proximity and belief similarity? Explicit bonds allow the simulation to model bond weakening and dissolution as a crisis signal.
- **Occupation unlock mechanic**: Is tech-gating on new occupation types a property of the Civ's tech tree, or does it emerge from the presence of relevant Govs and institutions? Probably both — tech tree sets the ceiling, institutions determine whether a Pop actually has access.
- **Wild/non-sapient Pops**: The occupation distribution model applies naturally to non-sapient species (a predator Pop with `raider`-equivalent occupations, a herd Pop with `forager` occupations). The discovery layer and directed bud mechanics probably don't apply below a sapience threshold.
