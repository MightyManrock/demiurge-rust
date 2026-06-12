# Climate Map Design

**Date:** 2026-06-11
**Status:** Decided — not yet implemented

---

## Overview

Two heat maps: **temperature** and **precipitation**. They are related but distinct — a polar desert is cold and dry, a tropical rainforest is hot and wet, a temperate rainforest is moderate and very wet. Both are generated from the elevation map and ocean classification. Both feed into hydrology: temperature sets the snow/ice line and evaporation rate; precipitation sets where rivers actually start.

---

## Temperature Map

**Value range:** 0.0 = extreme cold (polar peak), 1.0 = extreme heat (equatorial lowland)

### Inputs

**Latitude** — primary driver. Temperature falls off from equator to pole following a cosine curve:

```
abs_lat = |y - height/2| / (height/2)    # 0 at equator, 1 at poles
lat_base = cos(abs_lat × π/2)            # 1.0 at equator, 0.0 at poles
```

**Elevation lapse rate** — temperature drops with altitude. A configurable `lapse_factor` scales how much the highest terrain cools relative to sea level. Default: 0.3 (meaning the absolute highest point is 0.3 cooler than its latitude base).

```
temperature = (lat_base - elevation × lapse_factor).clamp(0, 1)
```

**Ocean proximity** *(deferred)* — maritime climates moderate temperature extremes (coastal areas are cooler in summer, warmer in winter). Requires a distance-to-ocean map. Deferred to a later pass.

### Planet parameters (future injection points)

- `stellar_luminosity` — scales base equatorial temperature
- `axial_tilt` — affects seasonal variation and shifts the effective latitude of peak warmth away from the geometric equator; stronger tilt can push the warmest zone toward the tropics at certain orbital positions
- Ocean coverage (sea level) — higher oceans = more maritime moderation globally

---

## Precipitation Map

**Value range:** 0.0 = extreme arid, 1.0 = extreme wet

### Mechanism 1: Atmospheric circulation bands

Earth's general circulation creates predictable precipitation bands by latitude. These are modeled as a function of `abs_lat`:

| Latitude range | Band | Effect |
|---|---|---|
| 0–27° (abs_lat 0–0.30) | Intertropical Convergence Zone (ITCZ) | High precipitation; peaks at equator |
| 27–45° (abs_lat 0.30–0.50) | Subtropical high (Hadley cell descending) | Low precipitation; desert belt |
| 45–65° (abs_lat 0.50–0.72) | Mid-latitude cyclone belt | Moderate-high precipitation |
| 65–90° (abs_lat 0.72–1.00) | Polar high | Low precipitation; polar desert |

Implemented as a composite function of gaussian peaks and troughs over abs_lat.

### Mechanism 2: Prevailing winds and moisture advection

Moisture originates at ocean cells and is advected inland by prevailing winds. Wind direction varies by latitude:

| Latitude | Wind regime | Direction |
|---|---|---|
| Tropics (< 30°) | Trade winds | From east |
| Mid-latitudes (30–60°) | Westerlies | From west |
| Polar (> 60°) | Polar easterlies | From east |

**Implementation:** row sweeps. For westerlies, scan each latitude row west→east, carrying moisture forward from ocean cells. For easterlies, scan east→west. Blend the two moisture fields based on a latitude-dependent westerly weight function. Each sweep step:

1. If the current cell is ocean: reset carry moisture to 1.0
2. Otherwise: reduce carry by any upslope elevation gain (moisture is lost crossing a ridge)
3. Store cell moisture = carry

This naturally produces rain shadows: a mountain range blocks moisture from reaching its leeward side.

### Combining the mechanisms

```
moisture = moisture_west × westerly_weight(abs_lat)
         + moisture_east × (1 - westerly_weight(abs_lat))

precipitation = lat_band_factor(abs_lat) × (base_arid + moisture × moisture_weight)
```

The `base_arid` term ensures even fully inland areas get a minimum precipitation from local convection (not purely zero).

### Planet parameters (future injection points)

- `axial_tilt` — shifts latitude band positions and their seasonality; high tilt planets have extreme seasonal wet/dry cycles
- `rotation_rate` — affects Coriolis strength; slow rotators (Venus-like) have weaker circulation bands
- `wind_strength` — scales how far moisture penetrates inland before being depleted
- `moisture_loss_rate` — how aggressively elevation barriers strip moisture

---

## Downstream effects on hydrology

Once climate maps exist:

- **River sources:** precipitation replaces the current implicit "every high cell contributes equally" model. Flow accumulation is weighted by precipitation — dry cells contribute little or nothing.
- **Snow/ice:** cells where temperature < snow threshold and precipitation > 0 accumulate snowpack. Snowmelt (modeled as a function of temperature gradient across the year) becomes a seasonal river source.
- **Endorheic basins:** basins in rain shadows naturally receive low precipitation and may not accumulate enough inflow to overflow — endorheic behavior emerges organically rather than being imposed by a fill threshold.
- **Evaporation:** high-temperature cells lose water faster, affecting lake persistence and river reach in arid zones.

---

## Output

Two heat maps generated together (they share inputs):

- `temperature.png` — color gradient TBD (blue=cold → white=snow → yellow=temperate → red=hot)
- `precipitation.png` — color gradient TBD (tan=arid → light green=semi-arid → deep green=wet → blue=extremely wet)
