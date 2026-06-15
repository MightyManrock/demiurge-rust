# Diurnal Temperature Variation

## The problem

The current `generate_temperature` output is effectively a diurnal mean — useful for seasonal climate modeling but it conceals what may be the most ecologically significant feature of certain worlds: the gap between the daytime peak and the predawn trough.

Low atmospheric humidity is the key variable. Water vapor acts as a thermal blanket, absorbing outgoing longwave radiation at night and re-emitting it downward. Strip that blanket away and the surface radiates freely into space after dark, dropping sharply. During the day, the same dry air offers little buffer against solar heating. The result is a large diurnal swing even at moderate latitudes — the same effect that makes Earth's deserts cold at night despite their daytime heat.

Oros makes this extreme:
- **Low liquid coverage (33%)** → weak moisture cycle, limited cloud cover
- **Long rotation period (1.25 standard days / ~30 hours)** → more time to heat under direct radiation, more time to radiate before dawn
- **Axial tilt (22°)** contributes seasonal variation but the diurnal swing likely exceeds it

A rough analogy: the Sahara swings ~30–40°C between midday and predawn. Oros, drier and with a longer day, probably does worse.

## What this means for the Keth

The Keth are nocturnal bipeds. The physical grounding for this is now quite specific: the Oros night is probably the only thermally tolerable window for their metabolism. This isn't a preference — it's a constraint. The long night (~15 standard hours) offers a sustained comfortable window before the long day becomes punishing.

This has a direct implication for their **temperature comfort range** in the `Species` definition. Currently their range is calibrated against the annual mean temperature field, which doesn't capture the day/night split. Their actual comfort zone is probably:

- **Lower bound**: somewhere above the predawn trough (they need to function, not just survive)
- **Upper bound**: well below the daytime peak (they're sheltering during this period)

The mean temperature at a habitable Oros location might look fine, but if the diurnal swing is large, a significant portion of each day is outside tolerance. Nocturnal species should score against the *nighttime trough*, not the mean.

## Proposed modeling approach

Add a `diurnal_swing` HeatMap derived from:
- The existing temperature field (mean)
- Atmospheric humidity / `precip_moisture` (low humidity → high swing)
- `rotation_period` from `PlanetParams` (longer day → more time to heat and cool)

Something like:

```
swing(cell) = base_swing * (1 - humidity_factor) * rotation_factor
```

Where:
- `base_swing` scales with the mean temperature (hotter places swing more in absolute terms)
- `humidity_factor` ∝ `precip_moisture` (more moisture = more buffering)
- `rotation_factor` ∝ `sqrt(rotation_period)` (diminishing returns; a 10× longer day doesn't give 10× the swing due to thermal mass)

Then expose two derived fields:
- `temp_day(cell) = mean + swing / 2`
- `temp_night(cell) = mean - swing / 2`

## Habitability implications

Species habitability scoring should be aware of activity pattern:

- **Diurnal species**: score temperature comfort against `temp_day`
- **Nocturnal species**: score temperature comfort against `temp_night`
- **Crepuscular / flexible**: score against `mean` or a weighted blend

The Keth are nocturnal, so their effective temperature range should be re-evaluated against `temp_night` rather than the mean. Their current comfort range probably needs to shift — a location that looks marginal on the mean map might be quite comfortable at night, and a location that looks comfortable on the mean might be uninhabitable during the day (relevant if the Keth ever need to travel or have any daytime exposure).

## Priority note

This is worth implementing before tectonic/underground work because:
1. It's relatively self-contained (one new HeatMap, a tweak to habitability scoring)
2. It makes the Keth's defining trait mechanically meaningful rather than just lore
3. It will likely change which regions on Oros look habitable, which may inform other design decisions about where Keth civilizations form

Tectonic dynamics would fundamentally change how land is generated. Diurnal swing touches only the climate and habitability layers, which are downstream and easier to adjust.
