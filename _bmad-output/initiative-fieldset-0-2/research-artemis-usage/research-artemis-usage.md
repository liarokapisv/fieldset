---
type: research
title: How artemis uses fieldset 0.1 (usage flows and constraints)
status: reference
sources: dreadbox/artemis @ 92b89e7, liarokapisv/fieldset @ 85f1a62
---

# Purpose

Reference material for the fieldset 0.2 design. It records how the one real
consumer (the Artemis synthesizer firmware, Cortex-M4 "master" core) uses the
crate today, what it worked around, and the flows the new design must serve.
Artemis itself is not being migrated in this initiative; this document exists so
the spec and architecture can cite concrete usage without the artemis checkout.

Paths below are relative to the artemis repository root.

# The model

- `core/artemis_core/src/parameters.rs` is the canonical parameter tree:
  `PresetParameters` (root) containing `BaseParameters`, which contains effect
  groups (`DistortionsParameters`, `ModulationsParameters`, `DelaysParameters`,
  `ReverbsParameters`, `SeqParameters`), each containing per-effect structs.
  Three to four levels of `#[fieldset]` nesting. `GlobalSettings` and
  `UserInputs` are two further, smaller roots.
- Leaves are overwhelmingly `Norm` and `SNorm` (bounded `f32` newtypes from
  `xbounded`), plus `bool`, small enums (`Wave`, `SyncMode`, `TrackMode`) and
  a few typed scalars (`Bpm`, `u8`).
- Every struct derives `Clone, Copy, Debug, Serialize, Deserialize`, most
  derive `Default`, all are `repr(C)` because `SharedParameters` is pushed to the
  audio core over shared-memory IPC (`core/artemis_core/src/ipc.rs:71`). The
  model type must stay plain data; nothing may be embedded in it.
- Measured on a synthetic replica of the same shape (401 `f32` leaves, 3
  levels): struct 1604 B; `FieldType` enum 16 B for a 4 B payload (nested tags
  plus `f32` alignment); `Option<FieldType>` 16 B (niche); OptFieldSet 3204 B;
  BitFieldSet 6480 B; PerfFieldSet 7232 B.

# What artemis actually uses from 0.1

| Feature | Used in production | Where |
|---|---|---|
| `FieldType` event enums as the event vocabulary | yes, everywhere | `artemis_master/src/lib.rs`, `page_service.rs`, `pages/*`, `cc_in_service.rs` |
| `apply(event)` on the plain struct | yes | `lib.rs:734`, `lib.rs:757` |
| `IntoIterator for PresetParameters` (replay every field) | yes | `lib.rs:721` (`notify_all` after preset load / init) |
| `OptFieldSet` | yes, one instance | `user_inputs_service.rs:90`, drained with `mem::take(..).into_iter()` each poll |
| `BitFieldSet`, `PerfFieldSet` | tests only | `parameters.rs:1293-1330` |
| `*FieldSetter` traits as controller interfaces | no | imported in `lib.rs:46-48`, never used as a parameter type |

The setter traits were abandoned. Likely causes, inferred from the code: the
sets could not be reused (no `clear`, consuming `into_iter`), so a controller
would construct a 7 KB `PerfFieldSet` per call; returning one through
`Option<Action>` is a large stack copy, which this codebase avoids on principle
(`dbx_init`); and the derive only compiles with `FieldSetter` separately
imported.

# Write flows today (modification points)

All controllers return events instead of writing through a setter.

1. **Panel / pages.** `page_service.rs:341 handle_user_input_event(event,
   &parameters, ...) -> Option<Action>`. A page gets `UserInputsFieldType`
   plus `&BaseParameters` for reads, and returns one event. Because the return
   is a single value, `Action` grew `PresetParameterChanges(ev1, ev2)` and
   `PanelLoadAction::PresetParameterChanges` (`page_service.rs:252-287`), and
   `pages/panel.rs:470` produces the pair for the EG "Both" mode (same value to
   `eg_*` and `vca_eg_*`; order independent). The main loop unrolls them with a
   `for event in [event1, event2]` (`lib.rs:850`).
2. **MIDI CC in.** `cc_in_service.rs` maps CC number to a variant constructor
   stored as `Fn(u7) -> BaseParametersFieldType` (lines 69-76 and the
   `slider_to_*` functions), returns `HandleCCAction::PresetParameter(event)`.
3. **Dispatcher BPM.** `lib.rs:662-667` reads the clock, builds the event by
   hand, writes `parameters.preset.base.bpm` directly, and calls
   `pages.handle_parameter_change` itself, bypassing the main handler.
4. **Modulation clear.** `Action::Clear(source)` (`lib.rs:945`) resets a whole
   sub-struct with `Default::default()` and notifies through a bespoke
   `pages.notify_modulation_cleared`. Comment there notes CC-out will force a
   refactor.
5. **Sequencer steps.** Owned by the dispatcher as a second source of truth;
   synced back once per loop pass via `dispatcher.if_sequence_changed`
   (`lib.rs:680`) and replayed on load via `import_sequence` (`lib.rs:719`).
   A save handled earlier in the same pass serializes the previous pass's
   steps (minor ordering issue).
6. **Preset load / init.** `persistency.load_preset` then `notify_all`, which
   replays every non-skipped field through `handle_parameter_change`.
   `name` is notified separately (`handle_preset_change`).

Controllers need read access to the model while deciding what to write
(jump-catch, increments, dependent fields). Today they get `&BaseParameters`
or `&PresetParameters` alongside the input event.

# Notification flow today (reaction points)

`lib.rs:727 handle_parameter_change(parameters, pages, dispatcher, event,
preset_id, preview)`: applies the event to the model, matches a few variants
for dispatcher side effects (`PlayMode`, `Bpm`), then forwards `&event` and
`&parameters.preset` to `pages.handle_parameter_change`, which forwards to
each page. Pages match on the nested enum and redraw. `handle_settings_change`
(`lib.rs:753`) snapshots the settings, applies, and saves when `prev !=
new`. The whole `SharedParameters` is pushed to the audio core every loop pass
(`lib.rs:687`), independent of events.

Consumers of events: pages (display), dispatcher (clock, play mode),
persistency (settings), and in future CC-out. All run in one embassy task;
the loop is single-writer and single-threaded.

# Skipped fields and why

| Field | Reason | Handled by |
|---|---|---|
| `_lfo_1_rate`, `_lfo_2_rate`, `_left_time`, `_right_time`, `_time` | serde v1 migration sidecars (`preset_migrations.rs:94-118`), intentionally stale after edits | never notified, correct |
| `steps: [Step<6>; 64]` | would make every event and every Perf/Bit slot the size of the array (largest-field problem) | dispatcher ownership and sync-back |
| `name: Option<Name>`, `format_schema: u8` | metadata | explicit calls |

Only `steps` is a memory-driven skip. It is a collection, and the natural unit
of change is one step, so the design needs indexed fields rather than a
whole-array event.

# Modelling idioms that matter for the design

- **Sum types are flattened.** `DelaysParameters` is a `typ: DelaysType` tag
  plus every effect's parameters as sibling structs, retained across type
  switches. This is both a workaround for the derive's lack of enum support
  and a UX feature (settings survive switching). Enums with payloads can be
  leaves; nested enum editing is not required.
- **Bounded value types do the clamping.** `Norm`, `SNorm`, `u7` cannot hold
  out-of-range values, so the write path needs no value validation hook.
- **Policy is per controller.** Jump-catch lives in pages; modal states are
  handled by the page service deciding what to dispatch. No central gate is
  needed; routing in the loop covers origin and echo suppression.

# Flows the 0.2 design must support

1. A controller receives an input event and a writer over its subtree, reads
   current values through the writer, and writes one or several fields in one
   call. Narrowed writers for sub-controllers (a delays page cannot touch
   anything outside delays). Cross-subtree reads passed by copy by the caller.
2. The loop drains after each controller call and routes the batch of changed
   paths to reactors of its choosing (for example, everyone except CC-out
   after a CC-in batch). Reactors receive the path and `&model` (or the
   narrowed subtree), never `&mut`.
3. Bulk writes: reset a sub-struct to defaults with one call, emitting one
   path per field.
4. Replay: after a preset load, mark every field dirty so the next drain
   notifies everything, replacing `IntoIterator for PresetParameters`.
5. Indexed fields: per-step events for the sequencer so it can rejoin the
   model instead of living in the dispatcher.
6. Small ad-hoc roots: `UserInputs` is polled every tick and drained
   immediately; the set must be cheap to clear and must not copy on drain.
7. Settings persistence: "save if anything changed" from the drain, with
   `set` skipping writes equal to the current value.
8. Long-lived sets in statics, constructed in place (`const fn new`), never
   staged on the stack.

# Numbers to design against

- Preset: 401 leaves, 1604 B. UI batches are a handful of changes at up to
  ~1 kHz polling; the only 400-change batch is the preset replay.
- Bench (x86, release, LTO; 401 leaves; ns per "set k fields then drain"):
  ideal reusable slot-array (Perf-like) 9 / 50 / 191 / 1359 for k = 1 / 16
  / 64 / 400; shadow plus mask with index-tree dispatch 12 / 139 / 541 /
  3516; the 0.1 sets 157-5860 because of per-batch construction and consuming
  iteration. Dirty-list designs with one indirect call per item sit between
  the first two.
- Flash is plentiful (STM32H745, 2 MB); RAM on the M4 side is the constraint.
