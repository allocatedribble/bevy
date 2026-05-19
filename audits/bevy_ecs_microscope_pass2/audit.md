# Bevy ECS Microscope Pass 2: Query Oracle Follow-Up

## Scope

This follow-up starts the missing Pass 2 query correctness surface. It does not attempt a planner change. The patch adds one deterministic slow-oracle regression that scans live entities directly and compares that oracle against normal Bevy query results for sparse optional fetches, `Has<Sparse>`, `AnyOf`, and `With`/`Without` filters after component churn.

The known dense-transmute sparse edge remains documented by `dense_query_over_option_is_buggy`; this follow-up adds a passing oracle for the non-transmuted query shapes that planner work must preserve.

## Baseline

| Field | Value |
| --- | --- |
| Baseline commit | `e44ecd3b9837f6714960fb7c20406bdfdd20f5ae` |
| Branch | `codex-bevy-ecs-command-pass4` |
| Branch policy | No new branch created, per operator instruction. |

## Patch Surface

- Added `slow_oracle_matches_sparse_optional_has_anyof_and_filters` in `crates/bevy_ecs/src/query/state.rs`.
- The oracle iterates `World::iter_entities`, checks component presence through direct `World::get`, sorts entity sets, and compares against query iteration output.
- The generated world includes table components, sparse-set components, insert/remove churn, and despawn churn.
- The query shapes covered are:
  - `(Entity, &OracleTableA, Option<&OracleSparseA>)`
  - `(Entity, Has<OracleSparseA>)` filtered by `With<OracleTableA>`
  - `(Entity, AnyOf<(&OracleTableA, &OracleSparseA)>)`
  - `Entity` filtered by `(With<OracleTableA>, Without<OracleSparseB>)`

## Correctness Evidence

This is intentionally a seed oracle, not a full randomized query model. It confirms that ordinary sparse optional and sparse `Has` paths match a direct world scan under structural churn. It also gives the future planner pass a concrete harness style for comparing entity sets and fetched sparse/table values without relying on current query internals.

Remaining Pass 2 gaps:

- Dynamic `QueryBuilder` oracle cases.
- Manual `QueryState::update_archetypes` oracle cases.
- `QueryState::transmute`, `QueryLens`, and `FilteredEntityRef` oracle cases.
- `Added<T>` and `Changed<T>` oracle cases.
- Sparse optional and `Has<Sparse>` dense-transmute behavior decision: fix, make impossible, or document as intentionally unsupported with named tests.

## Validation Commands

| Command | Result |
| --- | --- |
| `cargo fmt -p bevy_ecs` | pass |
| `cargo test -p bevy_ecs slow_oracle_matches_sparse_optional_has_anyof_and_filters --lib` | pass |
| `cargo test -p bevy_ecs query::state --lib` | pass, 34 passed |
| `cargo test -p bevy_ecs --lib` | pass, 918 passed, 2 ignored |

## Next Patch Candidates

- Expand this oracle into a table-driven query matrix that can reuse one generated world across ordinary, dynamic, and transmuted query paths.
- Add an explicit failing or current-behavior oracle around the dense-transmuted `Option<&Sparse>` and `Has<Sparse>` edge so the planner decision is visible.
- Add the first query planner benchmark gate for sparse optional density and sparse-driving iteration before attempting any optimization.
