# Bevy ECS Microscope Pass 7 Audit

## Scope

Pass 7 audits observers, hooks, and lifecycle dispatch: observer reentrancy, observer cache mutation during dispatch, duplicate route behavior, lifecycle no-observer overhead, and unregister pressure across many archetypes.

No new branch was created for this pass because the operator explicitly requested that work continue on the current branch.

## Baseline

| Field | Value |
| --- | --- |
| Branch | `codex-bevy-ecs-command-pass4` |
| Baseline commit | `6f97fa2401cbb29acbf94ba30b5a1a5085e7353e` |
| Rust | `rustc 1.97.0-nightly (4b0c9d76a 2026-05-10)` |
| Cargo | `cargo 1.97.0-nightly (a343accce 2026-05-08)` |
| Host | `x86_64-pc-windows-msvc` |
| OS | `Microsoft Windows [Version 10.0.26200.8457]` |
| CPU | `AMD Ryzen 9 7900X3D 12-Core Processor` |
| Logical threads | `24` |
| Power plan | High performance, `8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c` |

`Get-CimInstance` was blocked by access denial in this shell. CPU came from `reg query HKLM\HARDWARE\DESCRIPTION\System\CentralProcessor\0 /v ProcessorNameString`, OS from `cmd /c ver`, logical thread count from `[Environment]::ProcessorCount`, and the power plan from `powercfg /getactivescheme`.

## Commands

| Command | Status | Evidence |
| --- | --- | --- |
| `cargo fmt -p bevy_ecs -p benches` | Pass | Formatting applied cleanly after edits. |
| `cargo test -p bevy_ecs observer --lib` | Pass | `62 passed; 0 failed`, including new reentrancy and duplicate-route regressions. |
| `cargo test -p bevy_ecs --features bevy_ecs_audit audit_counters_record_representative_ecs_paths --lib` | Pass | Audit counter smoke test covers observer dispatch and no-observer counters. |
| `cargo bench -p benches --bench ecs_microscope --features ecs_audit --no-run` | Pass | Expanded microscope benchmark compiles. |
| `cargo bench -p benches --bench ecs_microscope --features ecs_audit observer_dispatch -- --quiet` | Pass | Focused observer dispatch, lifecycle, irrelevant observer, and unregister benchmarks ran cleanly. |
| `cargo test -p bevy_ecs --lib` | Pass | `907 passed; 0 failed; 2 ignored`. |
| `cargo test -p bevy_ecs --features multi_threaded --lib` | Pass | `909 passed; 0 failed; 2 ignored`. |
| `cargo test -p bevy_ecs --features bevy_ecs_audit --lib` | Pass | `909 passed; 0 failed; 2 ignored`. |
| `cargo test -p bevy_ecs --no-default-features --lib` | Blocked | Existing test modules import `std`, `bevy_reflect`, and `MultiThreadedExecutor` while those features are disabled. |
| `cargo test -p bevy_ecs --all-features --lib` | Blocked | `bevy_reflect` emits `compile_error!`: automatic reflect registration needs `auto_register_inventory` or `auto_register_static`. |
| `cargo miri test -p bevy_ecs observer --lib` | Blocked | `cargo-miri.exe` is not installed for `nightly-x86_64-pc-windows-msvc`. |
| `Get-Command cargo-flamegraph`, `Get-Command valgrind`, `Get-Command heaptrack`, `Get-Command samply` | Blocked | No local profiler command found. |

## Patch Surface

- Added `CachedObservers::is_empty`, `CachedComponentObservers::is_empty`, and non-lifecycle empty-cache cleanup after observer unregister.
- Added fast no-observer exits for dynamic `World` and `DeferredWorld` trigger paths that return before constructing dispatch work when the event cache is missing or empty.
- Added `bevy_ecs_audit` counters for no-observer triggers, route-specific dispatches, and trigger-id dedupe skips.
- Routed global, entity, component, and entity-component observer dispatches through route-specific audit hooks while preserving the aggregate observer dispatch counter.
- Counted duplicate observer runner skips when the same observer is reached through overlapping route paths in one trigger.
- Added observer regression tests for self-despawn during dispatch, despawning another observer during dispatch, adding an observer during dispatch, recursive same-event trigger, recursive different-event trigger, and overlapping component-route dedupe.
- Expanded `ecs_microscope` with observer dispatch benchmarks for no-observer, global, entity, component, entity-component, irrelevant entity observers, lifecycle add, and unregister pressure across 1k/10k archetypes.

## Correctness Evidence

New tests cover:

- An observer despawning itself during dispatch fires for the active trigger, is removed after `flush`, and does not fire on the next trigger.
- An observer despawning another observer does not invalidate the active dispatch list; the victim fires once and is removed before the next trigger.
- An observer added through a queued command during dispatch waits until the next trigger.
- Recursive same-event and different-event triggers remain supported through deferred command application.
- One observer registered through overlapping component routes for one entity-component trigger fires exactly once, locking in trigger-id dedupe behavior.
- Unregistering the last non-lifecycle global observer now removes the empty event cache while leaving the observer entity alive when only the `Observer` component is removed.

No public observer, hook, lifecycle, or event API was intentionally changed.

## Benchmark Summary

Criterion estimates for `ecs_audit`:

| Case | Estimate |
| --- | --- |
| Global event, no observers | `6.8550 ns .. 6.8946 ns` |
| Global observer dispatch | `32.425 ns .. 37.099 ns` |
| Entity observer dispatch | `32.908 ns .. 34.976 ns` |
| Component observer dispatch | `34.889 ns .. 37.717 ns` |
| Entity-component observer dispatch | `44.838 ns .. 52.110 ns` |
| 1k irrelevant entity observers | `17.591 ns .. 17.934 ns` |
| 10k irrelevant entity observers | `21.118 ns .. 24.836 ns` |
| Lifecycle add, no observers | `4.8370 us .. 5.1707 us` |
| Lifecycle add, observer flags present | `6.7179 us .. 7.2550 us` |
| Unregister add observer, 1k archetypes | `444.17 us .. 597.43 us` |
| Unregister add observer, 10k archetypes | `10.995 ms .. 12.660 ms` |

The no-observer path is now explicit in code and instrumented. The unregister benchmark shows the current lifecycle flag cleanup still scales materially with archetype count when a component observer is removed.

No local allocation profile, cache-miss profile, flamegraph, callgrind, or Miri evidence is available on this machine yet.

## Triage

| Severity | Finding | Status |
| --- | --- | --- |
| S1 | Duplicate observer execution is easy to regress when one observer is reachable through overlapping entity/component routes. | Covered by a named regression; observed behavior is exactly-once per trigger via trigger-id dedupe. |
| S1 | Observer cache mutation during dispatch can invalidate assumptions about active dispatch lists. | Covered for self-despawn, despawning another observer, and adding a new observer during dispatch. |
| S1 | Recursive observer triggers need explicit coverage because deferred command application can hide ordering bugs. | Covered for same-event and different-event recursion. |
| S2 | Lifecycle observer unregister across many archetypes has measurable cost. | Quantified; no deeper flag-index rewrite attempted in this pass. |
| S2 | No-observer dispatch is common enough to deserve a measured fast path. | Patched for dynamic World and DeferredWorld trigger split paths and covered by benchmark plus audit counter. |
| S3 | Observer dispatch counters were aggregate-only and did not expose route mix or dedupe. | Addressed under `bevy_ecs_audit`. |
| S3 | Non-lifecycle observer caches could remain allocated after the last runner was removed. | Patched with empty-cache cleanup. |
| S3 | Local profiler and Miri evidence are missing. | Blocked by missing tools. |

## Rejected or Deferred

- Observer dispatch precompiled flat lists: deferred. The current pass first locks behavior and measures route pressure.
- Per-archetype observer mask cache: deferred. It needs a wider lifecycle and component-hook benchmark matrix.
- Dedupe scratch set reuse: deferred. Trigger-id dedupe already handles duplicate runner execution; scratch reuse needs allocation profiling first.
- Event batching for lifecycle hooks: deferred as high-risk semantic work around ordering, observers, and hooks.

## Next Patch Candidates

- Reduce lifecycle unregister cost by maintaining a per-component observed-archetype or remaining-observer count, avoiding broad archetype flag scans when removing the last component observer.
- Add allocation profiling for observer registration/unregistration once a Windows-compatible profiler lane is installed.
- Add no-std-compatible gating for std-only tests so `cargo test -p bevy_ecs --no-default-features --lib` can become useful audit evidence.
- Install Miri and one profiler lane, then rerun observer reentrancy tests and the observer dispatch benchmark with call stacks and allocation data.
