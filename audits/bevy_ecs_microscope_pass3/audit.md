# Bevy ECS Microscope Pass 3 Audit

## Scope

Pass 3 audits archetype, table, and sparse storage pressure. This pass intentionally stops at telemetry, randomized correctness coverage, and focused benchmarks. It does not attempt archetype compaction or sparse representation changes.

## Baseline

| Field | Value |
| --- | --- |
| Branch | `codex-bevy-ecs-storage-pass3` |
| Baseline commit | `a30625a0c Add Bevy ECS microscope audit harness` |
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
| `cargo test -p bevy_ecs --lib` | Pass, baseline | Before edits: `880 passed; 0 failed; 2 ignored`. |
| `cargo bench -p benches --bench ecs_microscope --features ecs_audit --no-run` | Pass, baseline | Existing microscope bench compiled before edits. |
| `cargo test -p bevy_ecs storage::audit_tests --lib` | Pass | New storage audit tests: `4 passed`. |
| `cargo test -p bevy_ecs --lib` | Pass | Post-patch default library lane: `884 passed; 0 failed; 2 ignored`. |
| `cargo test -p bevy_ecs --features bevy_ecs_audit storage::audit_tests --lib` | Pass | Audit-feature storage lane: `5 passed`. |
| `cargo test -p bevy_ecs --features bevy_ecs_audit storage::audit_tests::storage_metrics_report_archetype_and_sparse_pressure --lib -- --nocapture` | Pass | Printed retained archetype/table and high-index sparse metrics below. |
| `cargo bench -p benches --bench ecs_microscope --features ecs_audit --no-run` | Pass | Expanded microscope bench compiles. |
| `cargo bench -p benches --bench ecs_microscope --features ecs_audit` | Pass | Focused Criterion run completed. |
| `cargo miri test -p bevy_ecs --lib` | Blocked | `cargo-miri.exe` exists, but `miri` is not installed for `nightly-x86_64-pc-windows-msvc`. |
| `Get-Command cargo-flamegraph,samply,heaptrack,valgrind,perf` | Blocked | No local profiler command found. |
| `cargo fmt --package bevy_ecs --package benches` | Pass | Formatting applied cleanly. |
| `git diff --check` | Pass | No whitespace errors. |

## Patch Surface

- Added audit storage metrics under `bevy_ecs_audit`: archetype counts, empty archetypes, edge cache entries/slots, table counts, retained table capacity, table columns, sparse set count, sparse entity capacity, and sparse slot count.
- Added internal invariant helpers for tables, archetypes, and component sparse sets.
- Added default storage randomized model tests covering spawn, spawn batch, insert, remove, replace, take, despawn, clear, required components, ZST, high alignment, sparse components, drop counters, panic during drop, and panic during command application.
- Extended `ecs_microscope` with row-move, table growth, archetype churn, world clear, and high sparse-index benchmarks.

## Correctness Evidence

The randomized model runs four deterministic seeds with 768 operations per seed. It checks:

- live entity membership for model-live entities
- table and sparse component presence and values
- required component insertion
- `EntityLocation` coherence against archetype row and table row
- table entity row reverse mapping
- sparse set dense/sparse reverse mapping
- drop counter equality after replacement, removal, despawn, clear, and world drop

Separate panic tests verify:

- a panic during component `Drop` while clearing does not become a double-drop on later world drop
- a panic during command application leaves sparse storage checkable and allows a later command application

## Telemetry Evidence

Output from the audit metrics report:

| Scenario | Archetypes | Empty Archetypes | Edge Entries | Edge Slots | Tables | Empty Tables | Table Capacity | Table Columns |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1,000 transient combinations, then clear | 1002 | 1002 | 1001 | 5379 | 1002 | 1002 | 4008 | 4939 |
| 10,000 transient combinations, then clear | 4097 | 4097 | 4096 | 26624 | 4097 | 4097 | 16388 | 24577 |

High sparse-index output:

| Scenario | Sparse Sets | Sparse Entities | Sparse Capacity | Sparse Slots |
| --- | ---: | ---: | ---: | ---: |
| sparse component at entity index 1,000,000 | 2 | 2 | 128 | 1,000,002 |

The sparse-set values are aggregate ECS sparse storage metrics, so they include internal sparse component storage in addition to the test component.

## Benchmark Summary

Criterion estimates from the successful focused run:

| Case | Estimate |
| --- | --- |
| Table insert/remove, 1,000 entities | `79.079 us .. 80.226 us` |
| Sparse insert/remove, 1,000 entities | `70.152 us .. 71.194 us` |
| Table insert/remove, 10,000 entities | `775.47 us .. 801.47 us` |
| Sparse insert/remove, 10,000 entities | `683.13 us .. 708.48 us` |
| Row move insert/remove, width 1, 1,000 entities | `90.205 us .. 93.726 us` |
| Row move insert/remove, width 4, 1,000 entities | `119.71 us .. 122.70 us` |
| Row move insert/remove, width 16, 1,000 entities | `305.41 us .. 389.71 us` |
| Spawn growth, width 1, 10,000 entities | `268.99 us .. 287.87 us` |
| Spawn growth, width 4, 10,000 entities | `510.93 us .. 649.39 us` |
| Spawn growth, width 16, 10,000 entities | `1.0853 ms .. 1.1486 ms` |
| Create/clear 1,000 transient combinations | `1.9396 ms .. 2.0889 ms` |
| Query update after 1,000 empty-churn archetypes | `305.51 us .. 324.76 us` |
| World clear after 1,000 churn entities | `278.31 us .. 284.00 us` |
| Create/clear 10,000 transient combinations | `40.823 ms .. 43.506 ms` |
| Query update after 10,000 empty-churn archetypes | `10.383 ms .. 11.098 ms` |
| World clear after 10,000 churn entities | `9.2530 ms .. 9.9225 ms` |
| Sparse insert/get/clear at entity index 10,000 | `7.0413 us .. 7.1122 us` |
| Sparse insert/get/clear at entity index 1,000,000 | `4.3841 ms .. 4.4318 ms` |

## Triage

| Severity | Finding | Status |
| --- | --- | --- |
| S2 | Empty archetype/table churn is measurable: 10,000 transient combinations retained 4,097 empty archetypes and 4,097 empty tables, and query update over that empty-churn world measured about 10.4-11.1 ms. | Evidence captured; no compaction attempted. |
| S2 | High sparse entity indices remain memory-proportional to index: one sparse component at index 1,000,000 produced 1,000,002 aggregate sparse slots and a 4.38-4.43 ms insert/get/clear case. | Evidence captured; representation change deferred. |
| S3 | Randomized model coverage now guards table rows, entity locations, sparse reverse maps, required components, drop counts, and panic paths. | Addressed with tests. |
| S3 | Miri coverage is still unavailable until the active nightly has the `miri` component installed. | Blocked. |
| S3 | Allocation, cache-miss, and flamegraph/callgrind proof is still missing because no local profiler command was available. | Blocked; no optimization should land from this pass alone. |

## Next Patch Candidates

- Add a non-invasive debug warning or metric consumer for high empty-archetype ratios and high sparse slot-to-entity ratios.
- Split query update over empty archetypes into a query-level mitigation pass before considering archetype compaction.
- Prototype sparse paging behind an internal feature only after allocator/cache profiles exist.

