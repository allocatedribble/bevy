# Bevy ECS Microscope Pass 1 Audit

## Scope

Pass 1 establishes a measurement floor for the ECS audit campaign. This patch does not optimize ECS behavior. It adds feature-gated internal counters and a focused benchmark target so later query, command, scheduler, storage, observer, and relationship changes can be measured before they are proposed.

## Baseline

| Field | Value |
| --- | --- |
| Branch | `codex-bevy-ecs-microscope-pass1` |
| Baseline commit | `f55891a4ee2eaa11fd63da2023fc01854465c6a6` |
| Rust | `rustc 1.97.0-nightly (4b0c9d76a 2026-05-10)` |
| Cargo | `cargo 1.97.0-nightly (a343accce 2026-05-08)` |
| Host | `x86_64-pc-windows-msvc` |
| OS | `Microsoft Windows 10.0.26200` |
| CPU | `AMD Ryzen 9 7900X3D 12-Core Processor` |
| Logical threads | `24` |
| Power plan | High performance, `8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c` |

## Commands

| Command | Status | Evidence |
| --- | --- | --- |
| `cargo test -p bevy_ecs` | Pass | Baseline full lane passed before patch: `471 passed; 0 failed; 3 ignored`; doctests passed. |
| `cargo test -p bevy_ecs --all-features` | Fail, baseline config issue | After fetching `subsecond`, compilation failed in `bevy_reflect`: automatic reflect registration requires either `auto_register_inventory` or `auto_register_static`. |
| `cargo test -p bevy_ecs --no-default-features` | Fail, baseline config issue | Test build hit unresolved `reflect`/`bevy_reflect`, `std`-gated test code, and missing `MultiThreadedExecutor` surfaces. |
| `cargo test -p bevy_ecs --features multi_threaded` | Pass | Baseline full lane passed before patch: `473 passed; 0 failed; 3 ignored`; doctests passed. |
| `cargo miri test -p bevy_ecs --lib` | Blocked | `cargo-miri.exe` exists, but `miri` is not installed for `nightly-x86_64-pc-windows-msvc`. |
| `cargo bench --bench bevy_ecs` | Fail, target discovery | No bench target named `bevy_ecs`; the existing ECS bench is `cargo bench -p benches --bench ecs`. |
| `cargo bench -p benches --bench ecs` | Partial | Existing full ECS Criterion suite ran for about 47 minutes and timed out before completing. |
| `cargo test -p bevy_ecs --lib` | Pass | Post-patch default library lane: `880 passed; 0 failed; 2 ignored`. |
| `cargo test -p bevy_ecs --features multi_threaded --lib` | Pass | Post-patch multi-threaded library lane: `882 passed; 0 failed; 2 ignored`. |
| `cargo test -p bevy_ecs --features bevy_ecs_audit audit::tests::audit_counters_record_representative_ecs_paths --lib` | Pass | Audit counter representative-path regression passed. |
| `cargo bench -p benches --bench ecs_microscope --features ecs_audit --no-run` | Pass | Focused microscope bench target compiled in release bench profile. |
| `cargo bench -p benches --bench ecs_microscope --features ecs_audit` | Pass | Focused benchmark completed in about 72 seconds. |
| `cargo fmt --package bevy_ecs --package benches` | Pass | Formatting applied cleanly. |
| `git diff --check` | Pass | No whitespace errors. |

## Patch Surface

- Added private crate feature `bevy_ecs_audit`.
- Added `bevy_ecs::audit` counters behind `bevy_ecs_audit`; without the feature these calls compile to no-op functions in a crate-private module.
- Instrumented query archetype updates, table row moves, sparse set activity, command queue push/apply, multi-threaded executor scans and contention, ApplyDeferred timing, observer dispatch depth, and relationship collection mutations.
- Added focused bench target `ecs_microscope` under the `benches` package, enabled with `--features ecs_audit`.

## Benchmark Summary

Criterion estimates from `cargo bench -p benches --bench ecs_microscope --features ecs_audit`:

| Case | Estimate |
| --- | --- |
| Query update, 10 archetypes | `3.6297 us .. 4.1458 us` |
| Query update, 100 archetypes | `24.792 us .. 26.620 us` |
| Query update, 1000 archetypes | `349.71 us .. 377.44 us` |
| Optional sparse query, 1% density | `18.209 us .. 20.022 us` |
| Optional sparse query, 10% density | `21.456 us .. 22.358 us` |
| Optional sparse query, 50% density | `34.199 us .. 35.019 us` |
| Optional sparse query, 90% density | `46.384 us .. 47.423 us` |
| Command storm, 100 commands | `849.21 ns .. 879.54 ns` |
| Command storm, 1000 commands | `13.919 us .. 16.424 us` |
| Command storm, 10000 commands | `137.45 us .. 149.13 us` |
| Table insert/remove, 1000 entities | `167.71 us .. 193.61 us` |
| Sparse insert/remove, 1000 entities | `144.03 us .. 156.33 us` |
| Table insert/remove, 10000 entities | `1.8976 ms .. 2.1518 ms` |
| Sparse insert/remove, 10000 entities | `1.5673 ms .. 1.8177 ms` |
| Single-thread schedule, 10 systems | `156.59 ns .. 173.07 ns` |
| Multi-thread schedule, 10 systems | `24.413 us .. 27.858 us` |
| Single-thread schedule, 100 systems | `1.0815 us .. 1.1938 us` |
| Multi-thread schedule, 100 systems | `69.215 us .. 71.891 us` |
| Single-thread schedule, 1000 systems | `11.498 us .. 12.021 us` |
| Multi-thread schedule, 1000 systems | `787.02 us .. 800.47 us` |
| Global observer trigger storm, 10000 triggers | `321.90 us .. 359.87 us` |
| One parent with 1000 children | `495.42 us .. 529.93 us` |
| One parent with 10000 children | `24.097 ms .. 25.404 ms` |

## Correctness Proof Surface

- Existing default and multi-threaded `bevy_ecs` library suites pass after patch.
- New audit regression verifies representative query, sparse set, command queue, and observer counters are wired through real ECS paths.
- Slow-oracle query differential tests are not part of this pass; they are the next pass before any query planner change.
- Miri coverage is blocked by a missing `miri` component for the active nightly toolchain.

## Performance Proof Surface

- Focused Criterion benchmark now covers query archetype update, optional sparse iteration, command storms, storage churn, scheduler pressure, observer storms, and relationship fanout.
- The existing full ECS bench suite is too broad for a quick pass and timed out locally; future passes should either narrow it or run it in CI with a larger timeout.
- Allocation profiling, cache-miss profiling, and flamegraph/callgrind evidence were not captured in this environment. Local probes did not find `cargo-flamegraph`, `samply`, `heaptrack`, `valgrind`, or `perf`.

## Triage

| Severity | Finding | Status |
| --- | --- | --- |
| S3 | ECS lacked a focused microscope benchmark target with audit counters for pass-by-pass work. | Addressed by `ecs_microscope` and `bevy_ecs_audit`. |
| S3 | `--all-features` lane currently needs an explicit reflect auto-registration backend. | Baseline issue recorded; not changed in this pass. |
| S3 | `--no-default-features` test lane currently compiles std/reflect/multi-threaded test surfaces. | Baseline issue recorded; not changed in this pass. |
| S3 | Miri lane is unavailable until the nightly toolchain has the `miri` component installed. | Blocked, recorded. |
| S3 | Flamegraph, allocation, and cache-miss profiles are still missing from the proof surface. | Blocked by missing local profilers; no optimization should land on this evidence alone. |

