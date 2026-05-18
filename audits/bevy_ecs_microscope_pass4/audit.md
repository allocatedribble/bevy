# Bevy ECS Microscope Pass 4 Audit

## Scope

Pass 4 audits `CommandQueue`, `Commands::append`, world command flushing, `ParallelCommands`, and explicit deferred application pressure. The patch is intentionally low risk: it fixes the partially-consumed append invariant, adds regression tests, extends audit counters, and expands the existing microscope benchmark harness. It does not attempt typed command batching or command reordering.

## Baseline

| Field | Value |
| --- | --- |
| Branch | `codex-bevy-ecs-command-pass4` |
| Baseline commit | `bfd8b466e Add Bevy ECS storage audit pass` |
| Rust | `rustc 1.97.0-nightly (4b0c9d76a 2026-05-10)` |
| Cargo | `cargo 1.97.0-nightly (a343accce 2026-05-08)` |
| Host | `x86_64-pc-windows-msvc` |
| OS | `Microsoft Windows 10.0.26200.8457` |
| CPU | `AMD Ryzen 9 7900X3D 12-Core Processor` |
| Logical threads | `24` |
| Power plan | High performance, `8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c` |

## Commands

| Command | Status | Evidence |
| --- | --- | --- |
| `cargo test -p bevy_ecs world::command_queue --lib` | Pass, baseline | Before edits: `6 passed; 0 failed`. |
| `cargo bench -p benches --bench ecs_microscope --features ecs_audit --no-run` | Pass, baseline | Existing microscope bench compiled before edits. |
| `cargo fmt --package bevy_ecs --package benches` | Pass | Formatting applied cleanly. |
| `cargo test -p bevy_ecs world::command_queue --lib` | Pass | New command queue tests: `13 passed; 0 failed`. |
| `cargo test -p bevy_ecs system::commands::tests::append --lib` | Pass | Existing append coverage still passes. |
| `cargo test -p bevy_ecs --features bevy_ecs_audit audit::tests::audit_counters_record_representative_ecs_paths --lib` | Pass | Audit counters cover append, realloc, and world flush. |
| `cargo bench -p benches --bench ecs_microscope --features ecs_audit --no-run` | Pass | Expanded microscope bench compiles. |
| `cargo bench -p benches --bench ecs_microscope --features ecs_audit command` | Pass | Focused command/deferred Criterion run completed. |
| `cargo test -p bevy_ecs --lib` | Pass | Post-patch default library lane: `891 passed; 0 failed; 2 ignored`. |
| `cargo test -p bevy_ecs --features multi_threaded --lib` | Pass | Post-patch multi-threaded lane: `893 passed; 0 failed; 2 ignored`. |
| `cargo test -p bevy_ecs --features bevy_ecs_audit --lib` | Pass | Audit feature library lane: `893 passed; 0 failed; 2 ignored`. |
| `cargo test -p bevy_ecs --no-default-features --lib` | Blocked | Existing test harness uses `std`, `bevy_reflect`, and `MultiThreadedExecutor` in test modules with those features disabled. |
| `cargo test -p bevy_ecs --all-features --lib` | Blocked | `bevy_reflect` emits `compile_error!`: automatic reflect registration needs `auto_register_inventory` or `auto_register_static`. |
| `cargo miri test -p bevy_ecs --lib` | Blocked | `cargo-miri.exe` is not installed for `nightly-x86_64-pc-windows-msvc`. |
| `Get-Command cargo-flamegraph,samply,heaptrack,valgrind,perf` | Blocked | No local profiler command found. |
| `git diff --check` | Pass | No whitespace errors. |

## Patch Surface

- `CommandQueue::append` now drains only `other.bytes[other.cursor..]`, clears the already-consumed prefix, resets `other.cursor`, and documents the cursor invariant with debug assertions.
- `Commands::append` and raw world-queue append now delegate to the same cursor-aware helper, so `SystemBuffer::queue` and `ParallelCommandQueue::queue` inherit the same behavior.
- `RawCommandQueue::push` records queue reallocations under `bevy_ecs_audit`.
- `World::flush_commands` records world command flushes under `bevy_ecs_audit`.
- Audit counters now include command append calls, appended bytes, reallocations, and world flush count.
- `ecs_microscope` now covers command structural patterns, payload sizes, nested commands, append fan-in, `ParallelCommands`, and explicit `ApplyDeferred` barriers.

## Correctness Evidence

New and strengthened tests cover:

- `CommandQueue::append` with a partially-consumed source queue.
- Raw `Commands::append` into the world's command queue with a partially-consumed source queue.
- Fully consumed source queue append, proving no already-consumed command is replayed.
- Commands that enqueue commands and recursively call `world.flush`.
- Panic recovery when a command panics after enqueueing more commands.
- Panic recovery when a command panics before enqueueing more commands.
- ZST command payloads.
- Large command payloads.
- Alignment-sensitive command payloads.
- Command payload drop counters.

The append caller trace was:

| Caller | Resolution |
| --- | --- |
| `CommandQueue::append` | Calls `append_unapplied_command_bytes`. |
| `Commands::append` with `InternalQueue::CommandQueue` | Calls `CommandQueue::append`. |
| `Commands::append` with `InternalQueue::RawCommandQueue` | Calls `RawCommandQueue::append`. |
| `SystemBuffer for CommandQueue::queue` | Calls `world.commands().append(self)`. |
| `ParallelCommandQueue::queue` | Calls `world.commands().append(cq)` for each thread queue. |

## Benchmark Summary

Criterion estimates from the focused command run:

| Case | Estimate |
| --- | --- |
| Existing fake command storm, 100 | `844.77 ns .. 855.42 ns` |
| Existing fake command storm, 1,000 | `8.3008 us .. 8.4150 us` |
| Existing fake command storm, 10,000 | `86.015 us .. 88.154 us` |
| `commands.spawn` one-by-one, 100 | `7.9162 us .. 8.0212 us` |
| `commands.spawn_batch`, 100 | `3.7882 us .. 3.8275 us` |
| Mixed spawn/insert/remove/despawn, 100 | `19.595 us .. 19.892 us` |
| `commands.spawn` one-by-one, 1,000 | `44.418 us .. 45.203 us` |
| `commands.spawn_batch`, 1,000 | `17.291 us .. 17.810 us` |
| Mixed spawn/insert/remove/despawn, 1,000 | `143.22 us .. 147.90 us` |
| `commands.spawn` one-by-one, 10,000 | `455.93 us .. 462.57 us` |
| `commands.spawn_batch`, 10,000 | `161.06 us .. 164.54 us` |
| Mixed spawn/insert/remove/despawn, 10,000 | `1.4257 ms .. 1.4522 ms` |
| Many tiny commands, 100 | `1.2021 us .. 1.2185 us` |
| Nested commands, 100 | `2.0516 us .. 2.1001 us` |
| Many tiny commands, 1,000 | `9.8917 us .. 10.410 us` |
| Nested commands, 1,000 | `18.354 us .. 18.700 us` |
| Many tiny commands, 10,000 | `97.445 us .. 98.334 us` |
| Nested commands, 10,000 | `180.84 us .. 183.72 us` |
| Large 4 KiB commands, 10 | `2.0881 us .. 2.1175 us` |
| Large 4 KiB commands, 100 | `21.149 us .. 21.556 us` |
| Append 10 queues | `1.3561 us .. 1.4230 us` |
| Append 100 queues | `3.6974 us .. 3.7318 us` |
| Append 1,000 queues | `24.952 us .. 25.163 us` |
| `ParallelCommands` insert, 1,000 entities | `75.071 us .. 79.448 us` |
| `ParallelCommands` insert, 10,000 entities | `707.93 us .. 726.63 us` |
| Explicit `ApplyDeferred` barriers, 1 | `4.6766 us .. 5.7578 us` |
| Explicit `ApplyDeferred` barriers, 4 | `16.670 us .. 18.059 us` |
| Explicit `ApplyDeferred` barriers, 16 | `26.161 us .. 29.140 us` |

The focused run reused Criterion's prior baseline for the existing `command_storm` cases and reported no material regression: 100 commands stayed within noise, 1,000 improved, and 10,000 had no statistically significant change.

## Triage

| Severity | Finding | Status |
| --- | --- | --- |
| S1 | `CommandQueue::append` did not encode what happens when `other.cursor > 0`. If a partially-consumed queue is appended, replaying consumed bytes is possible from the visible implementation. | Patched to append only unapplied bytes; regression tests cover normal and raw-world append paths. |
| S1 | Panic recovery around nested command enqueue must preserve unapplied command bytes after both before-enqueue and after-enqueue panics. | Strengthened tests; no runtime patch needed beyond append invariant. |
| S3 | Command queue telemetry did not expose append fan-in, reallocations, or world flush count. | Addressed under `bevy_ecs_audit`. |
| S3 | Command/deferred benchmark harness lacked spawn, spawn_batch, mixed structural, nested, append fan-in, `ParallelCommands`, and explicit `ApplyDeferred` cases. | Addressed in `ecs_microscope`. |
| S3 | No local Miri, flamegraph, callgrind, cache-miss, or allocation profiler evidence is available on this machine. | Blocked; audit counters and Criterion are present, but deeper optimization must wait for tools. |
| S3 | `--no-default-features` and `--all-features` library test lanes are not currently runnable as written. | Documented blockers; not caused by the command queue patch. |

## Next Patch Candidates

- Prototype homogeneous command batching behind an internal benchmark-only feature and compare against the new structural command baselines.
- Add a typed audit dump for command bytes per apply and flushes per frame so applications can correlate schedule shape with command pressure.
- Investigate `ApplyDeferred` barrier placement and unapplied bitset handling with a profiler installed; do not optimize from wall time alone.
