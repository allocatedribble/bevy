# Bevy ECS Microscope Pass 6 Audit

## Scope

Pass 6 audits change detection: tick wraparound, long-running tick maintenance, table and sparse tick storage scans, custom schedule tick maintenance, `World::last_change_tick_scope`, and `Changed<T>` filter pressure.

No new branch was created for this pass because the operator explicitly requested that work continue on the current branch.

## Baseline

| Field | Value |
| --- | --- |
| Branch | `codex-bevy-ecs-command-pass4` |
| Baseline commit | `7394cea714320c1472ad24192297d0ab74a55bc1` |
| Rust | `rustc 1.97.0-nightly (4b0c9d76a 2026-05-10)` |
| Cargo | `cargo 1.97.0-nightly (a343accce 2026-05-08)` |
| Host | `x86_64-pc-windows-msvc` |
| OS | `Microsoft Windows [Version 10.0.26200.8457]` |
| CPU | `AMD Ryzen 9 7900X3D 12-Core Processor` |
| Logical threads | `24` |
| Power plan | High performance, `8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c` |

`Get-CimInstance` was blocked by access denial in this shell, so CPU came from the processor registry key and logical thread count came from `NUMBER_OF_PROCESSORS`.

## Commands

| Command | Status | Evidence |
| --- | --- | --- |
| `cargo fmt -p bevy_ecs -p benches` | Pass | Formatting applied cleanly after edits. |
| `cargo test -p bevy_ecs change_detection --lib` | Pass | `23 passed; 0 failed`. |
| `cargo test -p bevy_ecs tick --lib` | Pass | `9 passed; 0 failed`, including custom schedule tick tests. |
| `cargo test -p bevy_ecs --features bevy_ecs_audit audit_counters_record_representative_ecs_paths --lib` | Pass | Audit counters include change-tick scans, empty storage scans, and component-tick scan counts. |
| `cargo bench -p benches --bench ecs_microscope --features ecs_audit --no-run` | Pass | Expanded microscope benchmark compiles. |
| `cargo bench -p benches --bench ecs_microscope --features ecs_audit change_detection -- --quiet` | Pass | Focused scan and filter benchmarks ran cleanly. |
| `cargo bench -p benches --bench ecs_microscope --features ecs_audit,bevy_ecs/track_location change_detection -- --quiet` | Pass | Focused scan and filter benchmarks ran with caller-location tracking enabled. |
| `cargo test -p bevy_ecs --lib` | Pass | `901 passed; 0 failed; 2 ignored`. |
| `cargo test -p bevy_ecs --features multi_threaded --lib` | Pass | `903 passed; 0 failed; 2 ignored`. |
| `cargo test -p bevy_ecs --features track_location --lib` | Pass | `903 passed; 0 failed; 2 ignored`. |
| `cargo test -p bevy_ecs --features bevy_ecs_audit --lib` | Pass | `903 passed; 0 failed; 2 ignored`. |
| `cargo test -p bevy_ecs --no-default-features --lib` | Blocked | Existing test modules import `std`, `bevy_reflect`, and `MultiThreadedExecutor` while those features are disabled. |
| `cargo test -p bevy_ecs --all-features --lib` | Blocked | `bevy_reflect` emits `compile_error!`: automatic reflect registration needs `auto_register_inventory` or `auto_register_static`. |
| `cargo miri test -p bevy_ecs change_detection --lib` | Blocked | `cargo-miri.exe` is not installed for `nightly-x86_64-pc-windows-msvc`. |
| `cargo flamegraph --version`, `valgrind --version`, `heaptrack --version` | Blocked | No local profiler command found. |

## Patch Surface

- Added `bevy_ecs_audit` counters for change-tick full scans, threshold skips, scan nanoseconds, tables scanned, empty tables scanned, sparse sets scanned, empty sparse sets scanned, and component tick cells visited.
- Added an audit-only forced scan helper so Criterion can measure `World::check_change_ticks` without performing hundreds of millions of tick increments.
- Skipped empty tables before walking their columns during tick maintenance.
- Skipped empty sparse sets before dispatching to the dense tick column.
- Added randomized tick-window tests comparing table-stored and sparse-stored `Added<T>` and `Changed<T>` filters against a slow arithmetic oracle across wraparound and expiration edges.
- Added deterministic coverage for add/change/remove sequences, resource mutation through `set_if_neq`, interior mutability plus manual `set_changed`, and `World::last_change_tick_scope`.
- Added custom schedule tick maintenance regressions for rare systems and schedules removed then reinserted into `Schedules`.
- Expanded `ecs_microscope` with change-tick scan and `Changed<T>` filter benchmarks for table and sparse storage, including a `track_location` lane.

## Correctness Evidence

New tests cover:

- Tick wraparound and expiration edges using randomized event ages and system ages.
- Table-stored and sparse-stored `Added<T>` and `Changed<T>` filters against an explicit oracle.
- Components added, changed, removed, and reinserted across table and sparse storage.
- Resource change detection through `ResMut`-style mutation and `set_if_neq`.
- Interior mutability that does not automatically mark a resource changed until `set_changed` is called.
- `World::last_change_tick_scope` both changes detection inside the scope and restores the previous tick afterward.
- `Schedule::check_change_ticks` clamping a rare system's `last_run`.
- `Schedules::check_change_ticks` clamping a schedule after remove/reinsert.

No public query or change-detection semantics were intentionally changed.

## Benchmark Summary

Criterion estimates for `ecs_audit`:

| Case | Estimate |
| --- | --- |
| Check ticks, 1M table components | `782.92 us .. 814.90 us` |
| Check ticks, 1k tables | `7.4673 us .. 7.6697 us` |
| Check ticks, 10k tables | `103.97 us .. 107.95 us` |
| Check ticks, 10k sparse components across sparse sets | `6.8176 us .. 6.9301 us` |
| Check ticks, 100k sparse components across sparse sets | `67.314 us .. 67.919 us` |
| `Changed<TableOnly>`, 1 percent dirty, 10k entities | `2.7960 us .. 2.8997 us` |
| `Changed<TableOnly>`, all dirty, 10k entities | `4.9742 us .. 5.1357 us` |
| `Changed<Sparse>`, 1 percent dirty, 10k entities | `19.315 us .. 19.440 us` |
| `Changed<Sparse>`, all dirty, 10k entities | `32.922 us .. 33.152 us` |
| `Changed<TableOnly>`, 1 percent dirty, 100k entities | `26.737 us .. 27.119 us` |
| `Changed<TableOnly>`, all dirty, 100k entities | `47.045 us .. 48.856 us` |
| `Changed<Sparse>`, 1 percent dirty, 100k entities | `193.54 us .. 195.04 us` |
| `Changed<Sparse>`, all dirty, 100k entities | `326.07 us .. 327.72 us` |

Criterion estimates for `ecs_audit,bevy_ecs/track_location`:

| Case | Estimate |
| --- | --- |
| Check ticks, 1M table components | `1.0881 ms .. 1.1357 ms` |
| Check ticks, 1k tables | `8.1022 us .. 8.8148 us` |
| Check ticks, 10k tables | `112.40 us .. 127.31 us` |
| Check ticks, 10k sparse components across sparse sets | `8.9077 us .. 9.7356 us` |
| Check ticks, 100k sparse components across sparse sets | `83.921 us .. 90.533 us` |
| `Changed<TableOnly>`, 1 percent dirty, 10k entities | `2.8221 us .. 2.8930 us` |
| `Changed<TableOnly>`, all dirty, 10k entities | `4.7926 us .. 4.9368 us` |
| `Changed<Sparse>`, 1 percent dirty, 10k entities | `19.137 us .. 19.518 us` |
| `Changed<Sparse>`, all dirty, 10k entities | `32.745 us .. 32.998 us` |
| `Changed<TableOnly>`, 1 percent dirty, 100k entities | `27.198 us .. 28.968 us` |
| `Changed<TableOnly>`, all dirty, 100k entities | `46.889 us .. 48.460 us` |
| `Changed<Sparse>`, 1 percent dirty, 100k entities | `197.92 us .. 199.97 us` |
| `Changed<Sparse>`, all dirty, 100k entities | `331.12 us .. 334.31 us` |

The empty-storage skip is intentionally small. It removes column iteration for empty tables and avoids no-op dense sparse tick checks, which matters most after archetype or sparse-set churn leaves persistent empty storage. It is not expected to change populated-storage scan costs.

No local allocation profile, cache-miss profile, flamegraph, callgrind, or Miri evidence is available on this machine yet.

## Triage

| Severity | Finding | Status |
| --- | --- | --- |
| S1 | Custom schedules outside the `Schedules` resource need explicit `CheckChangeTicks` maintenance or rare-system `last_run` ticks can become stale. | Documented upstream behavior now has regression coverage for schedule clamping and remove/reinsert through `Schedules`. |
| S1 | Tick wraparound and expiration semantics are easy to regress for sparse storage because table and sparse tick arrays are maintained separately. | Covered by randomized table/sparse oracle tests. |
| S2 | `World::check_change_ticks` scans all persistent storage once the threshold is crossed, including empty tables and sparse sets created by churn. | Partially patched with empty table/sparse skip and quantified with scan benchmarks. |
| S2 | Sparse `Changed<T>` filter iteration is materially slower than table storage at the same entity counts in this matrix. | Quantified; no dirty index or chunk summary attempted in this pass. |
| S3 | Change-tick scan cost was not visible through the audit counters. | Addressed under `bevy_ecs_audit`. |
| S3 | Local profiler and Miri evidence are missing. | Blocked by missing tools. |

## Rejected or Deferred

- Per-storage `last tick scan` cache: deferred. The current scan only runs at the threshold and the safety proof for skipping a storage needs per-column min/max or equivalent metadata.
- Dirty component index and per-table chunk dirty summaries: deferred as medium-risk planner/storage work requiring a wider benchmark matrix.
- Event-style change journals: deferred as high-risk semantic and memory-model work.

## Next Patch Candidates

- Add column-level tick min/max metadata to make "cannot overflow yet" checks O(number of columns) instead of O(number of components).
- Add table-chunk dirty summaries for `Changed<T>` filters, starting with table storage only.
- Add a no-std-compatible test lane or split std-dependent tests behind `#[cfg(feature = "std")]`.
- Install Miri and one Windows-compatible profiler lane, then rerun this pass with Miri and flamegraph evidence.
