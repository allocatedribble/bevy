# Bevy ECS Microscope Pass 10: Memory And Allocation Audit

## Baseline

| Field | Value |
| --- | --- |
| Baseline commit | `f50ff4d9d70daffc055a5e5eecb6bd9b79939442` |
| Branch | `codex-bevy-ecs-command-pass4` |
| Branch policy | No new branch created, per operator instruction. |
| Rust | `rustc 1.97.0-nightly (4b0c9d76a 2026-05-10)` |
| Cargo | `cargo 1.97.0-nightly (a343accce 2026-05-08)` |
| Host | `x86_64-pc-windows-msvc` |
| OS | `Microsoft Windows [Version 10.0.26200.8457]` |
| CPU | `AMD Ryzen 9 7900X3D 12-Core Processor` |
| Threads | 24 |
| Power plan | High performance, `8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c` |

## Scope

Pass 10 measures memory retained after high-churn ECS workloads. The patch stays behind the private `bevy_ecs_audit` feature and adds in-process retained-capacity telemetry instead of changing storage behavior.

The retained-byte estimate is a lower-bound and attribution tool, not a replacement for an allocator profiler. It covers collection capacities directly visible from `World`, including entity metadata, archetypes, table entities, table columns, sparse sets, command queue buffers, and observer caches.

## Patch Surface

- Added `MemoryMetrics` and `memory_metrics(&World)` under `bevy_ecs_audit`.
- Extended `StorageMetrics` with sparse-slot and archetype-edge capacity fields.
- Added audit-only retained-capacity helpers for:
  - `Entities`
  - `Archetypes`
  - tables and columns
  - sparse sets
  - observer caches
- Added `memory_metrics_record_representative_retained_capacity` as a fast regression test.
- Added ignored heavy scenario coverage in `memory_audit_heavy_churn_scenarios`, which prints stable `memory_audit label=...` rows for follow-up tooling.

No production code path changes when `bevy_ecs_audit` is disabled.

## External Profiler Availability

`dhat`, `heaptrack`, and `valgrind` were not available in this Windows environment. Jemalloc profiling was not configured for this target. The pass therefore uses audit-feature allocator counters and retained-capacity estimates. A Linux or provisioned profiling runner should still run heaptrack, dhat, or jemalloc profiling before any allocator-policy change lands.

## Heavy Scenario Measurements

Command:

```powershell
cargo test -p bevy_ecs memory_audit_heavy_churn_scenarios --features bevy_ecs_audit --lib -- --ignored --nocapture
```

Result: pass, 54.26s.

| Scenario | Estimated retained | Entity meta | Archetypes | Table entities | Table columns | Sparse sets | Command queue | Observer cache | Shape |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `spawn_despawn_1m` | 57,171,745 B, 54.5 MiB | 25,165,824 B | 16,001,645 B | 8,000,032 B | 8,000,064 B | 4,168 B | 12 B | 0 B | 3 empty archetypes, 3 empty tables |
| `archetype_churn_100k` | 191,419,494 B, 182.6 MiB | 3,145,728 B | 158,988,722 B | 3,200,032 B | 26,080,832 B | 4,168 B | 12 B | 0 B | 100,001 empty archetypes, 100,001 empty tables |
| `sparse_high_index_low_density` | 560,011,217 B, 534.1 MiB | 480,000,048 B | 3,757 B | 1,056 B | 64 B | 80,006,280 B | 12 B | 0 B | sparse slot capacity 20,000,006 |
| `command_storm_repeated` | 8,918,805 B, 8.5 MiB | 3,145,728 B | 2,098,797 B | 1,048,608 B | 1,048,640 B | 4,168 B | 1,572,864 B | 0 B | command queue retained 1.5 MiB |
| `observer_register_unregister_storm` | 3,479,773 B, 3.3 MiB | 393,216 B | 263,789 B | 131,104 B | 64 B | 2,691,200 B | 88 B | 312 B | observer cache capacity 3, runner capacity 0 |
| `relationship_add_remove_storm` | 11,541,183 B, 11.0 MiB | 3,145,728 B | 4,196,583 B | 2,097,216 B | 2,097,344 B | 4,168 B | 144 B | 0 B | 4 empty archetypes, 4 empty tables |
| `schedule_rebuild_storm` | 14,898 B, 0.014 MiB | 96 B | 1,750 B | 32 B | 64 B | 12,944 B | 12 B | 0 B | schedule allocations dropped with schedules |
| `large_alignment_table_churn` | 31,951,649 B, 30.5 MiB | 3,145,728 B | 1,601,645 B | 800,032 B | 26,400,064 B | 4,168 B | 12 B | 0 B | 100k aligned table components |
| `wide_table_component_removal` | 9,547,566 B, 9.1 MiB | 393,216 B | 613,498 B | 263,200 B | 8,273,472 B | 4,168 B | 12 B | 0 B | 35 empty archetypes, 35 empty tables |

## Top Retainers

1. `sparse_high_index_low_density`: 534.1 MiB retained. The top costs are entity metadata at 480,000,048 B and sparse-set backing storage at 80,006,280 B, driven by high entity indices and a sparse slot capacity of 20,000,006.
2. `archetype_churn_100k`: 182.6 MiB retained. The dominant cost is 158,988,722 B in archetype metadata and edge/index structures after 100,001 archetypes and tables become empty.
3. Table capacity after churn: `spawn_despawn_1m` retains 54.5 MiB, while `large_alignment_table_churn` retains 30.5 MiB with 26,400,064 B in table columns. These are expected retained capacities, but they are now quantifiable.

The command storm retained 1,572,864 B in command queue buffers after repeated application. Observer register/unregister churn did not leave a large observer cache; most of that scenario's retained bytes came from sparse component storage used by observer entities.

## Patch Candidates Evaluated

| Candidate | Status | Notes |
| --- | --- | --- |
| Audit-only memory report | Prepared | `memory_metrics` gives repeatable retained-capacity attribution without affecting production builds. |
| Observer cache cleanup | Rejected for now | Heavy scenario retained only 312 B in observer cache storage after unregister. |
| Command queue capacity policy | Deferred | Retention is visible at 1.5 MiB after storm; changing reuse/shrink policy needs separate latency and allocation benchmarks. |
| `World::shrink_to_fit` | Deferred | Broad API and invariant surface. Needs a dedicated design pass across entities, tables, sparse sets, archetypes, command queues, and observer caches. |
| Paged sparse-set representation | Deferred | Strong candidate for high-index sparse workloads, but it is a storage redesign and needs query/storage benchmarks plus migration-risk review. |
| Archetype churn diagnostics | Supported | Empty-archetype telemetry from this pass makes diagnostics practical without compaction risk. |

## Validation Commands

| Command | Result |
| --- | --- |
| `git rev-parse HEAD` | `f50ff4d9d70daffc055a5e5eecb6bd9b79939442` |
| `rustc -Vv` | `rustc 1.97.0-nightly (4b0c9d76a 2026-05-10)`, host `x86_64-pc-windows-msvc` |
| `cargo -V` | `cargo 1.97.0-nightly (a343accce 2026-05-08)` |
| `cmd /c ver` | `Microsoft Windows [Version 10.0.26200.8457]` |
| `reg query "HKLM\HARDWARE\DESCRIPTION\System\CentralProcessor\0" /v ProcessorNameString` | `AMD Ryzen 9 7900X3D 12-Core Processor` |
| `powercfg /getactivescheme` | High performance, `8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c` |
| `Get-Command dhat,heaptrack,valgrind -ErrorAction SilentlyContinue` | no commands found |
| `cargo fmt -p bevy_ecs` | pass |
| `cargo test -p bevy_ecs audit_counters_record_representative_ecs_paths --features bevy_ecs_audit --lib` | pass |
| `cargo test -p bevy_ecs memory_metrics_record_representative_retained_capacity --features bevy_ecs_audit --lib` | pass |
| `cargo test -p bevy_ecs memory_audit_heavy_churn_scenarios --features bevy_ecs_audit --lib -- --ignored --nocapture` | pass, 54.26s |
| `cargo test -p bevy_ecs --lib` | pass, 917 passed, 2 ignored |
| `cargo test -p bevy_ecs --features bevy_ecs_audit --lib` | pass, 920 passed, 3 ignored |

## Exit Criteria

- Retained memory is quantified for every requested scenario.
- The top three visible retainers are identified.
- A low-risk telemetry patch is prepared under `bevy_ecs_audit`.
- Cleanup and shrink policies are deliberately deferred until allocator-profiled numbers are available.

## Deferred Work

- Run heaptrack, dhat, or jemalloc profiling on a provisioned Linux runner.
- Add Criterion or iai-callgrind memory scenarios around command queue capacity reuse and archetype churn diagnostics.
- Design a narrow `World::shrink_to_fit` proposal if retained capacity becomes a supported operational control.
- Prototype paged sparse-set backing storage behind an experiment gate and compare query/storage hot paths before considering it for production.
