# Bevy ECS Microscope Pass 9: Unsafe-Code Ledger

## Baseline

- Baseline commit: `cc32aaa64af8ee0692c15920f052bfddf097ee25`
- Branch: `codex-bevy-ecs-command-pass4`
- Branch policy: no new branch created, per operator instruction.
- Rust: `rustc 1.97.0-nightly (4b0c9d76a 2026-05-10)`
- Cargo: `cargo 1.97.0-nightly (a343accce 2026-05-08)`
- Host: `x86_64-pc-windows-msvc`
- OS: `Microsoft Windows [Version 10.0.26200.8457]`
- CPU: `AMD Ryzen 9 7900X3D 12-Core Processor`
- Threads: 24

## Patch Surface

- Added `crates/bevy_ecs/unsafe_audit.md`, a stable machine-readable and human-readable ledger for prioritized unsafe islands.
- Added red-team tests for:
  - stale entity location after table row swap-remove
  - sparse-set `remove_and_forget` ownership transfer through `EntityWorldMut::take`
  - unchecked query fetch after archetype update
  - untyped mutable component access through readonly `UnsafeWorldCell`
  - observer reentrancy mutating table and sparse storage
  - multi-threaded executor run-condition read access against a running mutable system conflict

No production unsafe code was added in this pass.

## Correctness Evidence

The ledger records S0/S1 unsafe invariants for query fetch/state, `UnsafeWorldCell`, `CommandQueue`, blob arrays, table storage, sparse sets, archetypes, the multi-threaded executor, and bundle insertion/removal/spawning/writing.

New S0/S1 regression coverage:

- `storage::audit_tests::table_row_move_updates_swapped_entity_location`
- `storage::audit_tests::sparse_set_take_forgets_storage_but_transfers_drop_ownership`
- `query::state::tests::unchecked_query_fetch_after_archetype_update_sees_moved_entity`
- `world::unsafe_world_cell::tests::as_unsafe_world_cell_readonly_component_mut_by_id_forbidden`
- `observer::tests::observer_reentrant_storage_mutation_updates_table_and_sparse_storage`
- `schedule::executor::multi_threaded::tests::run_condition_access_conflicting_with_running_systems_is_serialized`

Existing tests cited by the ledger continue to cover command queue panic recovery, bundle behavior, storage randomized operations, observer reentrancy, query transmutation, and executor exclusive-system Miri regressions.

## Validation Commands

| Command | Result |
| --- | --- |
| `cargo fmt -p bevy_ecs` | pass |
| `cargo test -p bevy_ecs storage::audit_tests::table_row_move_updates_swapped_entity_location --lib` | pass |
| `cargo test -p bevy_ecs storage::audit_tests::sparse_set_take_forgets_storage_but_transfers_drop_ownership --lib` | pass |
| `cargo test -p bevy_ecs unchecked_query_fetch_after_archetype_update_sees_moved_entity --lib` | pass |
| `cargo test -p bevy_ecs as_unsafe_world_cell_readonly_component_mut_by_id_forbidden --lib` | pass |
| `cargo test -p bevy_ecs observer_reentrant_storage_mutation_updates_table_and_sparse_storage --lib` | pass |
| `cargo test -p bevy_ecs run_condition_access_conflicting_with_running_systems_is_serialized --features multi_threaded --lib` | pass |
| `cargo test -p bevy_ecs --lib` | pass, 917 passed, 2 ignored |
| `cargo test -p bevy_ecs --features multi_threaded --lib` | pass, 919 passed, 2 ignored |
| `cargo miri test -p bevy_ecs --lib` | blocked: `cargo-miri.exe` is not installed for `nightly-x86_64-pc-windows-msvc` |
| `MIRIFLAGS="-Zmiri-strict-provenance" cargo miri test -p bevy_ecs --lib` | blocked: `cargo-miri.exe` is not installed for `nightly-x86_64-pc-windows-msvc` |
| `RUSTFLAGS="-Z sanitizer=address" cargo +nightly test -p bevy_ecs` | blocked before Bevy compilation: `serde` build script exits with `STATUS_DLL_NOT_FOUND` |
| `RUSTFLAGS="-Z sanitizer=leak" cargo +nightly test -p bevy_ecs` | unsupported: leak sanitizer is not supported for `x86_64-pc-windows-msvc` |
| `RUSTFLAGS="-Z sanitizer=thread" cargo +nightly test -p bevy_ecs --features multi_threaded` | unsupported: thread sanitizer is not supported for `x86_64-pc-windows-msvc` |

## Miri And Sanitizer Status

Miri could not run because the component is not installed for the active nightly. The ledger still identifies major Miri target groups so a Linux or fully provisioned nightly runner can execute them without rediscovering scope.

Sanitizers are best-effort on this machine:

- ASan starts compilation but fails in a dependency build script due to missing runtime DLLs under sanitizer flags.
- LSan and TSan are rejected by rustc as unsupported for the current Windows MSVC target.

## Performance Evidence

No performance patch was made in this pass. No benchmark was required beyond keeping the unsafe-ledger work out of hot production paths.

## Deferred Work

- Install the Miri component or run the Miri targets on a provisioned Linux/nightly runner.
- Add allocator-fault injection for table/blob-array reallocation panic paths. Current tests cover drop panic recovery and randomized storage invariants, but do not simulate allocator failure during column reallocation.
- Add focused Miri tests for `BundleSpawner` cached-pointer invalidation if future passes change spawn batching internals.
