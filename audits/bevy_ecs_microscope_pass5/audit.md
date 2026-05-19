# Bevy ECS Microscope Pass 5 Audit

## Scope

Pass 5 audits the scheduler and executor surface, especially `MultiThreadedExecutor`, `SingleThreadedExecutor`, run-condition skip semantics, explicit `ApplyDeferred`, and schedule build pressure. The patch is intentionally conservative: it adds audit-only telemetry, strengthens executor correctness tests, expands the microscope scheduler benchmarks, and removes one avoidable `FixedBitSet` clone from the multi-threaded explicit `ApplyDeferred` path.

No new branch was created for this pass because the operator explicitly requested that work continue on the current branch.

## Baseline

| Field | Value |
| --- | --- |
| Branch | `codex-bevy-ecs-command-pass4` |
| Baseline commit | `a6cb6213e Audit Bevy ECS command queue deferred application` |
| Rust | `rustc 1.97.0-nightly (4b0c9d76a 2026-05-10)` |
| Cargo | `cargo 1.97.0-nightly (a343accce 2026-05-08)` |
| Host | `x86_64-pc-windows-msvc` |
| OS | `Microsoft Windows NT 10.0.26200.0` |
| CPU | `AMD Ryzen 9 7900X3D 12-Core Processor` |
| Logical threads | `24` |
| Power plan | High performance, `8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c` |

`Get-CimInstance` was blocked by access denial in this shell, so CPU and thread count came from the processor registry key plus `[System.Environment]::ProcessorCount`.

## Commands

| Command | Status | Evidence |
| --- | --- | --- |
| `cargo fmt --package bevy_ecs --package benches` | Pass | Formatting applied cleanly after edits. |
| `cargo test -p bevy_ecs schedule::executor --lib` | Pass | `39 passed; 0 failed`. |
| `cargo test -p bevy_ecs --features multi_threaded schedule::executor --lib` | Pass | `39 passed; 0 failed`. |
| `cargo test -p bevy_ecs --features bevy_ecs_audit audit::tests::audit_counters_record_representative_ecs_paths --lib` | Pass | Audit counters cover scheduler spawn, lock, ready delay, bitset reuse, and deferred duration. |
| `cargo bench -p benches --bench ecs_microscope --features ecs_audit --no-run` | Pass | Expanded microscope bench compiles. |
| `cargo bench -p benches --bench ecs_microscope --features ecs_audit scheduler_pressure/multi_threaded/10000` | Pass | 10k multi-threaded executor pressure ran cleanly. |
| `cargo bench -p benches --bench ecs_microscope --features ecs_audit scheduler_pressure/multi_threaded_medium_query` | Pass | Medium query executor pressure ran cleanly. |
| `cargo bench -p benches --bench ecs_microscope --features ecs_audit scheduler_pressure/single_threaded/10000` | Pass | 10k single-threaded executor pressure ran cleanly. |
| `cargo bench -p benches --bench ecs_microscope --features ecs_audit scheduler_build_pressure` | Pass | Build-pressure group ran cleanly after removing routine 10k fully-conflicting cases. |
| `cargo bench -p benches --bench ecs_microscope --features ecs_audit scheduler_apply_deferred_frequency` | Pass | ApplyDeferred frequency group ran cleanly. |
| `cargo test -p bevy_ecs --lib` | Pass | `895 passed; 0 failed; 2 ignored`. |
| `cargo test -p bevy_ecs --features multi_threaded --lib` | Pass | `897 passed; 0 failed; 2 ignored`. |
| `cargo test -p bevy_ecs --features bevy_ecs_audit --lib` | Pass | `897 passed; 0 failed; 2 ignored`. |
| `cargo bench -p benches --bench ecs_microscope --features ecs_audit scheduler` | Superseded | The first combined run timed out after the initial harness sampled 10k fully-conflicting build cases at roughly 33 seconds per sample. The harness was adjusted and the scheduler groups were run separately. |
| `cargo test -p bevy_ecs --no-default-features --lib` | Blocked | Existing no-std test modules import `std`, `bevy_reflect`, and `MultiThreadedExecutor` without those features. |
| `cargo test -p bevy_ecs --all-features --lib` | Blocked | `bevy_reflect` emits `compile_error!`: automatic reflect registration needs `auto_register_inventory` or `auto_register_static`. |
| `cargo miri test -p bevy_ecs --lib` | Blocked | `cargo-miri.exe` is not installed for `nightly-x86_64-pc-windows-msvc`. |
| `Get-Command cargo-flamegraph,samply,heaptrack,valgrind,perf` | Blocked | No local profiler command found. |

## Patch Surface

- Added `bevy_ecs_audit` scheduler counters for spawned tasks, exclusive spawned tasks, non-Send spawned tasks, ready-to-run wait time, ready samples, idle-ready wait time, executor lock hold time, lock samples, and `ApplyDeferred` bitset reuse.
- Added audit-only `ApplyDeferred` duration and system-count telemetry for both single-threaded and multi-threaded executors.
- Kept timing and ready-queue bookkeeping fully behind `bevy_ecs_audit`; non-audit builds do not create `Instant` timers for these paths.
- Recycled the multi-threaded explicit `ApplyDeferred` `unapplied_systems` bitset with `mem::take` instead of cloning it for every explicit barrier.
- Added executor tests comparing single-threaded and multi-threaded constrained chain, diamond, skipped-system dependent, and skipped-set dependent semantics.
- Added a deferred-application panic test that proves panics propagate through both executors.
- Expanded `ecs_microscope` scheduler benches for 10k no-op systems, medium query systems, schedule build cost, conflict precomputation, condition-conflict precomputation, and `ApplyDeferred` frequency.

## Correctness Evidence

New tests cover:

- Single-threaded and multi-threaded execution of a strict chain, including exact log order.
- Single-threaded and multi-threaded execution of a diamond DAG, comparing allowed final-state equivalence instead of ambiguous internal ordering.
- Run-condition skips on a system and on a system set, proving dependents still run consistently across executors.
- Panic during deferred command application through both executor paths.
- Audit counter coverage for multi-threaded scheduler spawning, explicit `ApplyDeferred`, lock hold samples, ready-to-run samples, and recycled bitset tracking.

The patch does not add randomized DAG property tests yet. That remains the next correctness step.

## Benchmark Summary

Criterion estimates from the focused scheduler runs:

| Case | Estimate |
| --- | --- |
| 10k no-op systems, single-threaded run | `124.39 us .. 176.17 us` |
| 10k no-op systems, multi-threaded run | `6.1867 ms .. 7.2330 ms` |
| Medium query systems, multi-threaded, 10 systems | `29.585 us .. 35.256 us` |
| Medium query systems, multi-threaded, 100 systems | `591.55 us .. 661.30 us` |
| Medium query systems, multi-threaded, 1,000 systems | `19.002 ms .. 21.809 ms` |
| Schedule build, no-op single-threaded, 100 systems | `195.89 us .. 211.98 us` |
| Schedule build, no-op multi-threaded, 100 systems | `315.56 us .. 333.73 us` |
| Schedule build, no-op single-threaded, 1,000 systems | `22.640 ms .. 24.057 ms` |
| Schedule build, no-op multi-threaded, 1,000 systems | `27.813 ms .. 29.540 ms` |
| Schedule build, no-op single-threaded, 10,000 systems | `2.0927 s .. 2.4136 s` |
| Schedule build, no-op multi-threaded, 10,000 systems | `2.2364 s .. 2.3251 s` |
| Conflict precompute, 100 `ResMut` systems | `1.7736 ms .. 2.0178 ms` |
| Condition-conflict precompute, 100 conditioned `ResMut` systems | `1.8060 ms .. 1.9334 ms` |
| Conflict precompute, 1,000 `ResMut` systems | `170.72 ms .. 181.85 ms` |
| Condition-conflict precompute, 1,000 conditioned `ResMut` systems | `182.64 ms .. 190.55 ms` |
| Deferred frequency, final-only, 100 command systems | `71.830 us .. 117.60 us` |
| Deferred frequency, every 10 systems, 100 command systems | `86.621 us .. 147.77 us` |
| Deferred frequency, every system, 100 command systems | `84.338 us .. 161.56 us` |
| Deferred frequency, final-only, 1,000 command systems | `434.84 us .. 601.83 us` |
| Deferred frequency, every 10 systems, 1,000 command systems | `495.32 us .. 581.79 us` |
| Deferred frequency, every system, 1,000 command systems | `950.79 us .. 985.70 us` |

The attempted 10k fully-conflicting build cases measured roughly `32.574 s .. 34.025 s` per sample before the run timed out. Those cases were intentionally removed from routine Criterion sampling; the 100 and 1k conflict cases retain the O(n^2) pressure signal without making the standard microscope run impractical.

No local allocation profiler, cache-miss profiler, flamegraph, callgrind, or Miri evidence is available on this machine yet.

## Triage

| Severity | Finding | Status |
| --- | --- | --- |
| S2 | Explicit multi-threaded `ApplyDeferred` cloned `unapplied_systems` for every barrier. This is avoidable allocation/copy pressure on barrier-heavy schedules. | Patched by taking, clearing, and recycling the bitset after the barrier task completes. |
| S2 | Schedule build pressure is steep with many mutually-conflicting systems: 1k conflicting `ResMut` systems already costs around 171-182 ms, and the attempted 10k case costs over 32 seconds per sample. | Quantified; no high-risk planner rewrite attempted. |
| S2 | Multi-threaded no-op executor overhead is much higher than single-threaded for 10k tiny systems. | Quantified; no adaptive executor policy change attempted in this pass. |
| S3 | Scheduler telemetry did not expose ready delay, lock hold time, spawned task mix, or explicit barrier bitset reuse. | Addressed under `bevy_ecs_audit`. |
| S3 | Executor tests did not directly compare constrained single-threaded and multi-threaded DAG outcomes. | Addressed with chain, diamond, skip, and deferred-panic tests. |
| S3 | No local Miri, flamegraph, cache-miss, callgrind, or allocation-profile evidence is available. | Blocked by missing tools. |

## Next Patch Candidates

- Add a randomized schedule DAG model comparing single-threaded and multi-threaded final world state for constrained ordering only.
- Add a fast path for schedules with no run conditions and no exclusive or non-Send systems.
- Investigate conflict-matrix compression for sparse conflict graphs before attempting any task graph rewrite.
- Add optional audit dumps for ready-to-run delay and executor lock hold histograms so real applications can correlate tiny-system cliffs with schedule shape.
