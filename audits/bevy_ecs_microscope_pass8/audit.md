# Bevy ECS Microscope Pass 8: Relationship and Hierarchy Audit

## Baseline

- Baseline commit: `8767d4df23e9de6c04210bd735b18db65e9a040d`
- Branch: `codex-bevy-ecs-command-pass4`
- Branch policy: no new branch created, per operator instruction.
- Rust: `rustc 1.97.0-nightly (4b0c9d76a 2026-05-10)`
- Cargo: `cargo 1.97.0-nightly (a343accce 2026-05-08)`
- Host: `x86_64-pc-windows-msvc`
- OS: `Microsoft Windows [Version 10.0.26200.8457]`
- CPU: `AMD Ryzen 9 7900X3D 12-Core Processor`
- Threads: 24
- Power plan: `High performance` (`8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c`)

## Patch Surface

- Patched `EntityWorldMut::replace_related` to preserve first occurrence order while deduplicating repeated sources before rebuilding the relationship target mirror.
- Added debug-only duplicate input assertions to `replace_related_with_difference`.
- Added a randomized relationship mirror oracle over live entities and target collections.
- Added regression tests for duplicate `replace_children` input and duplicate difference input.
- Added an observer regression proving relationship target maintenance still emits lifecycle observer events.
- Added collection choice docs for `Vec<Entity>`, `SmallVec`, `EntityHashSet`, and `EntityIndexSet`.
- Added `ecs_microscope/relationship_hierarchy` benchmarks for large child sets, removal positions, reparenting, tree traversal, whole-tree despawn, and relationship source collection choices.

Public API shape is unchanged. The behavior change is that safe replacement helpers now collapse duplicate input sources before writing the target mirror; this enforces the documented source-of-truth invariant instead of allowing duplicate mirror entries.

## Correctness Evidence

The new oracle test runs 3 deterministic seeds with 512 operations per seed. It generates:

- insert/reparent relationship
- remove relationship
- despawn source or target
- spawn new entity
- insert relationship to an invalid target
- risky direct target mutation through `set_risky`
- batch spawn with relationships

After every operation it flushes hooks and checks:

- every live `MirrorRel` source points at a live target
- every `MirrorTarget` contains exactly the live sources pointing to that target
- target mirror entries contain no duplicate source entities
- no extra target mirror exists without a source

Duplicate behavior is now locked down:

- `replace_children(&[a, b, a, b])` produces `[a, b]`.
- `replace_children_with_difference` panics in debug builds when duplicate input violates its invariant.

Observer coverage is now explicit:

- adding a first relationship source inserts the target mirror and triggers `On<Add, MirrorTarget>`
- removing the last relationship source removes the target mirror and triggers `On<Remove, MirrorTarget>`

## Validation Commands

| Command | Result |
| --- | --- |
| `cargo fmt -p bevy_ecs -p benches` | pass |
| `cargo test -p bevy_ecs relationship --lib` | pass, 37 passed |
| `cargo test -p bevy_ecs hierarchy --lib` | pass, 31 passed |
| `cargo bench -p benches --bench ecs_microscope --features ecs_audit --no-run` | pass |
| `cargo bench -p benches --bench ecs_microscope --features ecs_audit relationship_hierarchy -- --quiet` | pass |
| `cargo test -p bevy_ecs --lib` | pass, 911 passed, 2 ignored |
| `cargo test -p bevy_ecs --features multi_threaded --lib` | pass, 913 passed, 2 ignored |
| `cargo test -p bevy_ecs --features bevy_ecs_audit --lib` | pass, 913 passed, 2 ignored |
| `cargo test -p bevy_ecs --no-default-features --lib` | blocked before Pass 8 code paths by existing std/bevy_reflect-gated test imports and std-gated executor references |
| `cargo test -p bevy_ecs --all-features --lib` | blocked in `bevy_reflect`: auto reflect registration backend feature not selected |
| `cargo miri test -p bevy_ecs relationship --lib` | blocked: `cargo-miri.exe` not installed for `nightly-x86_64-pc-windows-msvc` |

Profiler availability:

- `cargo-flamegraph`: not found
- `valgrind`: not found
- `heaptrack`: not found
- `samply`: not found

No allocation profile, cache-miss profile, flamegraph, or callgrind artifact was produced in this environment.

## Benchmark Summary

Criterion confidence intervals below are from the Pass 8 `relationship_hierarchy` run.

### Hierarchy Operations

| Benchmark | 95% CI |
| --- | --- |
| `add_children_vec/10` | 4.4266 us .. 4.6634 us |
| `add_children_vec/100` | 13.467 us .. 13.735 us |
| `add_children_vec/1000` | 92.879 us .. 93.742 us |
| `add_children_vec/10000` | 919.13 us .. 922.53 us |
| `add_children_vec/100000` | 10.030 ms .. 10.114 ms |
| `remove_first_child/10` | 1.5531 us .. 1.6346 us |
| `remove_middle_child/10` | 1.5288 us .. 1.5808 us |
| `remove_last_child/10` | 1.5845 us .. 1.6835 us |
| `remove_first_child/1000` | 2.1896 us .. 2.3872 us |
| `remove_middle_child/1000` | 2.2226 us .. 2.9530 us |
| `remove_last_child/1000` | 2.3138 us .. 3.2028 us |
| `remove_first_child/10000` | 11.633 us .. 12.938 us |
| `remove_middle_child/10000` | 9.4736 us .. 10.599 us |
| `remove_last_child/10000` | 6.7742 us .. 7.6819 us |
| `remove_first_child/100000` | 311.36 us .. 334.06 us |
| `remove_middle_child/100000` | 242.47 us .. 264.94 us |
| `remove_last_child/100000` | 231.51 us .. 253.57 us |
| `reparent_many_children/100` | 16.033 us .. 18.713 us |
| `reparent_many_children/1000` | 335.68 us .. 662.76 us |
| `reparent_many_children/10000` | 19.722 ms .. 20.084 ms |
| `despawn_whole_tree/100` | 25.558 us .. 67.225 us |
| `despawn_whole_tree/1000` | 59.453 us .. 61.342 us |
| `despawn_whole_tree/10000` | 578.32 us .. 588.14 us |

### Traversal

| Benchmark | 95% CI |
| --- | --- |
| `deep_tree_ancestors/100` | 688.36 ns .. 701.89 ns |
| `deep_tree_descendants/100` | 799.13 ns .. 824.33 ns |
| `deep_tree_ancestors/1000` | 7.0411 us .. 7.1804 us |
| `deep_tree_descendants/1000` | 8.3893 us .. 12.782 us |
| `deep_tree_ancestors/10000` | 70.926 us .. 73.991 us |
| `deep_tree_descendants/10000` | 81.549 us .. 83.334 us |
| `wide_tree_descendants/100` | 279.27 ns .. 291.06 ns |
| `wide_tree_descendants/1000` | 2.4028 us .. 2.4959 us |
| `wide_tree_descendants/10000` | 23.641 us .. 24.330 us |
| `wide_tree_descendants/100000` | 233.91 us .. 258.38 us |

### Relationship Source Collections

| Benchmark | 95% CI |
| --- | --- |
| `collection_add_remove/vec/10` | 5.9624 us .. 6.2571 us |
| `collection_add_remove/vec/100` | 25.478 us .. 26.441 us |
| `collection_add_remove/vec/1000` | 369.72 us .. 388.14 us |
| `collection_add_remove/vec/10000` | 20.736 ms .. 21.768 ms |
| `collection_add_remove/smallvec/10` | 6.3211 us .. 6.5459 us |
| `collection_add_remove/smallvec/100` | 24.202 us .. 24.787 us |
| `collection_add_remove/smallvec/1000` | 215.66 us .. 231.28 us |
| `collection_add_remove/smallvec/10000` | 4.8983 ms .. 5.1852 ms |
| `collection_add_remove/entity_hash_set/10` | 6.3019 us .. 6.6529 us |
| `collection_add_remove/entity_hash_set/100` | 23.332 us .. 24.464 us |
| `collection_add_remove/entity_hash_set/1000` | 182.68 us .. 191.22 us |
| `collection_add_remove/entity_hash_set/10000` | 2.1771 ms .. 2.8389 ms |
| `collection_add_remove/entity_index_set/10` | 7.5036 us .. 8.2203 us |
| `collection_add_remove/entity_index_set/100` | 30.549 us .. 31.918 us |
| `collection_add_remove/entity_index_set/1000` | 851.60 us .. 932.12 us |
| `collection_add_remove/entity_index_set/10000` | 57.345 ms .. 59.420 ms |

Criterion warned that the 10-sample target could not complete within 1 second for `reparent_many_children/10000` and `collection_add_remove/vec/10000`; those cases are still useful pressure points but should be rerun with longer measurement time before making tighter claims.

## Triage

- S1: duplicate input to safe relationship replacement could write duplicate source entries into the mirror collection. Patched by first-occurrence deduplication in `replace_related`.
- S1: relationship target mirror invariant lacked broad randomized coverage. Patched with a live-entity oracle.
- S2: large `Vec` relationship collections have measurable arbitrary-removal and reparenting cliffs. Quantified with position-sensitive removal and reparent benchmarks.
- S2: collection choice materially changes large add/remove cost. `EntityHashSet` is much better for the unordered 10k add/remove stress case, while `EntityIndexSet` is worst in this matrix.
- S3: collection choice guidance was implicit. Patched docs now steer large arbitrary-removal relationships toward set-backed collections.

## Deferred Work

- No default collection change in this pass. `Children` still benefits from compact insertion-order storage for common small sets, and changing it would be public behavior-adjacent.
- No swap-remove mode. It would require explicit unordered semantics on the relationship target collection.
- No adaptive `Vec` to set transition. It needs a design pass around representation stability, iteration order, and clone/reflect behavior.
- No relationship graph index, packed hierarchy storage, or deferred maintenance batcher. The new oracle and benchmarks are the admission gate for those larger designs.
