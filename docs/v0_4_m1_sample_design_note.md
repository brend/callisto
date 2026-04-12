# Callisto v0.4 M1 Sample Design Note

This note defines the `M1` sample-depth target implemented in `playdate_auto_bootstrap`.

## Goal

Evolve the sample from hold-driven scene selection to a longer-lived state machine with explicit transitions and persisted session state.

## State Machine

- Scenes: `Splash`, `Pilot`, `Telemetry`
- Transition events:
  - `A` just pressed -> advance scene (`Splash -> Pilot -> Telemetry -> Splash`)
  - `B` just pressed -> rewind scene (`Splash <- Pilot <- Telemetry <- Splash`)
  - no event -> remain in current scene

The active scene is no longer derived from button-hold state each frame; it persists until an explicit transition event occurs.

## Persisted Model Values

The sample model tracks session-level values across frames:

- `ticks`: total update count
- `scene_changes`: count of explicit transitions
- `pilot_frames`: total frames spent in `Pilot`
- `telemetry_frames`: total frames spent in `Telemetry`
- `last_transition`: last transition event (`Stayed`, `Advanced`, `Rewound`)

HUD labels are derived from these values to make persistence visible in runtime behavior.

## SDK Calls Required

No new binding surfaces were required.

The scenario uses existing bindings in `playdate.cal`:
- `buttonJustPressed`
- `getCrankChange`
- `getCrankPosition`

And existing graphics rendering calls from `playdate.graphics`.
