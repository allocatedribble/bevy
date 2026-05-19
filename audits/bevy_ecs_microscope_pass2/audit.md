# Bevy ECS Microscope Pass 2: Query Oracle Follow-Up

## Scope

This follow-up fills the missing Pass 2 query correctness surface listed in the hypothesis ledger. It does not attempt a planner change. The patch adds deterministic slow-oracle regressions that scan live entities directly and compare that oracle against Bevy query results for sparse optional fetches, `Has<Sparse>`, `AnyOf`, `With`/`Without`, dynamic `QueryBuilder`, manual `QueryState::update_archetypes`, `QueryState::transmute`, `QueryLens`, `FilteredEntityRef`, and `Added<T>`/`Changed<T>` filters after component churn.

The known dense-transmute sparse edge remains documented by `dense_query_over_option_is_buggy` and is now explicitly locked by an oracle-mismatch test. The decision for this pass is to preserve and document current behavior rather than make a planner or type-system change.

## Baseline

| Field | Value |
| --- | --- |
| Baseline commit | `e44ecd3b9837f6714960fb7c20406bdfdd20f5ae` |
| Expansion baseline | `fb2d2843a Seed Bevy ECS query oracle audit` |
| Branch | `codex-bevy-ecs-command-pass4` |
| Branch policy | No new branch created, per operator instruction. |

## Patch Surface

- The oracle iterates `World::iter_entities`, checks component presence through direct `World::get`, sorts entity sets, and compares against query iteration output.
- The generated world includes table components, sparse-set components, insert/remove churn, and despawn churn.
- Added oracle coverage for:
  - ordinary sparse optional, sparse `Has`, `AnyOf`, and `With`/`Without`
  - dynamic `QueryBuilder<FilteredEntityRef>` with runtime component IDs
  - manual `QueryState::update_archetypes` after a new archetype appears
  - `QueryState::transmute` into sparse `Has` and `FilteredEntityRef`
  - `Query::transmute_lens` through `QueryLens`
  - table and sparse `Added<T>` / `Changed<T>` filters
  - dense-transmuted `Option<&Sparse>` and `Has<Sparse>` current-behavior oracle mismatch

## Correctness Evidence

The deterministic oracle now covers every Pass 2 gap listed in the hypothesis ledger. It confirms that ordinary sparse optional, sparse `Has`, `AnyOf`, dynamic builder, manual update, transmute, lens, filtered entity, and change-filter paths match a direct world scan under controlled structural churn.

The dense-transmuted sparse optional/`Has` edge is intentionally not fixed here. The added test records the direct oracle result and asserts that current dense-transmuted query behavior still returns no sparse hits. That keeps the known behavior visible until a later planner/type-system patch chooses to fix or forbid it.

Remaining hardening work is no longer a gap prerequisite for planner admission; it is scale work:

- Convert the deterministic cases into a broader table-driven randomized matrix.
- Add dynamic query transmutation cases for more `FilteredEntityRef`/`FilteredEntityMut` access combinations.
- Add query planner benchmarks before any optimization lands.

## Validation Commands

| Command | Result |
| --- | --- |
| `cargo fmt -p bevy_ecs` | pass |
| `cargo test -p bevy_ecs slow_oracle_matches_sparse_optional_has_anyof_and_filters --lib` | pass |
| `cargo test -p bevy_ecs query::state --lib` | pass, 40 passed |
| `cargo test -p bevy_ecs --lib` | pass, 924 passed, 2 ignored |

## Next Patch Candidates

- Expand this oracle into a randomized table-driven matrix that can reuse one generated world across ordinary, dynamic, and transmuted query paths.
- Add the first query planner benchmark gate for sparse optional density and sparse-driving iteration before attempting any optimization.
