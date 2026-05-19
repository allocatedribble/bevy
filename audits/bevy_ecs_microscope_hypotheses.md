# Bevy ECS Microscope Hypothesis Ledger

This ledger turns the high-value hunt list into falsifiable audit targets. Update it after every pass that adds tests, benchmarks, telemetry, or a production patch.

Status values:

- `confirmed`: current evidence supports the hypothesis strongly enough to drive a narrow patch.
- `partially_confirmed`: evidence supports the direction, but the next patch still needs a tighter proof surface.
- `open`: evidence exists, but the hypothesis has not been tested directly enough.
- `falsified`: current evidence argues against the hypothesis.

## H1: Command Application Is The Most Accessible Performance Win

Status: `partially_confirmed`

Claim: one-by-one command application is suboptimal, and batching homogeneous structural commands can improve throughput and reduce surprising command-order pressure.

Current evidence:

- Pass 4 patched the sharp `CommandQueue::append` invariant for partially consumed queues and added panic, nested, ZST, large, alignment-sensitive, and drop-counter command tests.
- Pass 4 benchmarks show a clear spawn batching signal:
  - `commands.spawn` one-by-one, 10,000: `455.93 us .. 462.57 us`
  - `commands.spawn_batch`, 10,000: `161.06 us .. 164.54 us`
- Pass 10 shows repeated command storms can leave visible command queue retained capacity: 1,572,864 bytes in the measured storm.

Falsification test:

- Add a homogeneous spawn/insert/remove/despawn batching prototype behind an experiment flag.
- Compare it against existing command benchmarks with observers and relationship hooks enabled.
- Falsify if the batching prototype fails to beat current one-by-one application outside trivial spawn-only cases, or if preserving command, hook, and observer ordering requires enough barriers to erase the win.

Next patch target:

- Prototype one narrow typed run detector for adjacent spawn commands, with strict order-preservation tests before any benchmark claim.

## H2: Query Performance Cliffs Are Shape-Dependent

Status: `open`

Claim: table-heavy dense iteration is already strong, but sparse optional fetches, `Has<Sparse>`, and change filters are more fragile. A planner could beat one-size iteration for selected shapes.

Current evidence:

- Pass 1 benchmarks show optional sparse query cost rises with density:
  - 1 percent: `18.209 us .. 20.022 us`
  - 90 percent: `46.384 us .. 47.423 us`
- Pass 6 benchmarks show sparse `Changed<T>` filters are materially slower than table `Changed<T>` at the same entity counts:
  - `Changed<TableOnly>`, 1 percent dirty, 100k: `26.737 us .. 27.119 us`
  - `Changed<Sparse>`, 1 percent dirty, 100k: `193.54 us .. 195.04 us`
- Source comments and existing tests identify the dense-iteration edge for `Option<&Sparse>` and `Has<Sparse>` after query transmutation.

Evidence gap:

- No Pass 2 query oracle audit is present in the current branch history.
- There is no broad slow-oracle differential test suite for sparse optionals, `Has<Sparse>`, dynamic query transmutation, manual query-state update, and non-archetypal filters.

Falsification test:

- Build the slow query oracle first.
- Run the same randomized worlds through dense table-heavy queries, sparse-driving queries, optional sparse queries, `Has<Sparse>`, `AnyOf`, `Added`, and `Changed`.
- Falsify planner work if shape-specific plans do not beat the current strategy once oracle-covered semantics and update costs are included.

Next patch target:

- Fill the missing Pass 2 artifact: oracle tests, sparse optional regression, query planner benchmarks, and an explicit "planner admitted or rejected" table.

## H3: Archetype Churn Needs Telemetry Before Redesign

Status: `confirmed`

Claim: empty archetype persistence is explicit behavior. The first fix should be telemetry and stats; compaction is risky and not yet justified as the first move.

Current evidence:

- Pass 3 measured 10,000 transient combinations retaining 4,097 empty archetypes and 4,097 empty tables.
- Pass 3 measured query update after 10,000 empty-churn archetypes at `10.383 ms .. 11.098 ms`.
- Pass 10 measured `archetype_churn_100k` retaining 191,419,494 bytes, with 158,988,722 bytes attributed to archetype metadata and related structures.

Falsification test:

- Add telemetry consumers and real-world sample hooks before attempting compaction.
- Falsify compaction priority if query-level skipping, warnings, and docs solve the observed application pressure without moving archetype identity or cache-invalidation semantics.

Next patch target:

- Add a debug-facing archetype churn report that exposes archetype count, empty archetype count, edge capacity, empty table count, and query update pressure.

## H4: Scheduler Overpays For Tiny Systems

Status: `confirmed`

Claim: mutexes, bitset scans, condition checks, and task spawning are reasonable individually, but together dominate when systems are tiny.

Current evidence:

- Pass 1 showed 10k no-op schedules:
  - single-threaded: `11.498 us .. 12.021 us`
  - multi-threaded: `787.02 us .. 800.47 us`
- Pass 5 measured 10k no-op systems:
  - single-threaded run: `124.39 us .. 176.17 us`
  - multi-threaded run: `6.1867 ms .. 7.2330 ms`
- Pass 5 measured schedule build pressure:
  - 10k no-op single-threaded build: `2.0927 s .. 2.4136 s`
  - 10k no-op multi-threaded build: `2.2364 s .. 2.3251 s`
- Pass 5 patched one low-risk `ApplyDeferred` bitset clone by recycling the bitset instead.

Falsification test:

- Add fast-path executor benchmarks for no conditions, no exclusive systems, and no non-Send systems.
- Falsify tiny-system specialization if a fast path does not materially reduce overhead without regressing medium query systems.

Next patch target:

- Implement and benchmark a schedule/executor fast path for schedules with no run conditions and no exclusive or non-Send systems.

## H5: Relationship Correctness Needs An Oracle

Status: `confirmed`

Claim: hook-maintained relationship target mirrors are elegant but can desynchronize under reentrant operations, despawn, clone, invalid targets, and duplicate input.

Current evidence:

- Pass 8 added a randomized live-entity oracle over relationship components and target collections.
- Pass 8 found and patched an S1 duplicate mirror-entry bug in safe relationship replacement helpers.
- Pass 8 benchmarks show large relationship collection choice materially changes performance:
  - `collection_add_remove/vec/10000`: `20.736 ms .. 21.768 ms`
  - `collection_add_remove/entity_hash_set/10000`: `2.1771 ms .. 2.8389 ms`
  - `collection_add_remove/entity_index_set/10000`: `57.345 ms .. 59.420 ms`

Falsification test:

- Extend the oracle to clone paths, linked-spawn despawn, observer-triggered relationship maintenance, and reentrant command application.
- Falsify additional relationship redesign if the oracle remains clean and large-collection docs plus collection choice cover the observed pressure.

Next patch target:

- Add clone and linked-spawn cases to the relationship oracle before any graph-index or deferred-maintenance design.

## H6: Observers Need Duplicate And Reentrancy Tests

Status: `confirmed`

Claim: push-based observer dispatch is powerful enough that duplicate routes and reentrancy need permanent regression coverage.

Current evidence:

- Pass 7 added tests for self-despawn, despawning another observer, adding observers during dispatch, same-event recursion, different-event recursion, and overlapping component-route dedupe.
- Pass 7 locked duplicate route behavior: one observer reached through overlapping entity/component routes fires exactly once per trigger via trigger-id dedupe.
- Pass 7 patched no-observer fast paths and empty non-lifecycle cache cleanup.
- Pass 7 measured lifecycle unregister cost across 10k archetypes at `10.995 ms .. 12.660 ms`.
- Pass 10 measured observer register/unregister churn retaining only 312 bytes in observer cache storage after unregister, so cache cleanup currently looks effective.

Falsification test:

- Add allocation-profiled observer register/unregister runs with real heap tooling.
- Falsify further cache cleanup work if retained observer cache bytes remain negligible and unregister cost is dominated by lifecycle flag scans instead.

Next patch target:

- Reduce lifecycle unregister archetype scans or add a targeted remaining-observer count, gated by the existing 10k-archetype benchmark.

## Current Priority Order

1. Query Pass 2: highest correctness gap. Build the oracle before planner work.
2. Command batching experiment: highest accessible performance bet, but only after strict ordering tests.
3. Scheduler tiny-system fast path: well-quantified S2 pressure with a narrow possible low-risk path.
4. Archetype churn diagnostics: telemetry/reporting first, compaction later only if evidence forces it.
5. Relationship oracle expansion: extend existing oracle before deeper representation work.
6. Observer lifecycle unregister: target archetype scan cost, not cache cleanup, unless heap profiling says otherwise.
