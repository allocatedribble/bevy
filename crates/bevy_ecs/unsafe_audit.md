# bevy_ecs Unsafe-Code Ledger

Machine ledger version: 1

Scope: prioritized unsafe islands in `query/fetch.rs`, `query/state.rs`, `world/unsafe_world_cell.rs`, `world/command_queue.rs`, `storage/table/*`, `storage/sparse_set.rs`, `storage/blob_array.rs`, `archetype.rs`, `schedule/executor/multi_threaded.rs`, and `bundle/*`.

Policy: every later production unsafe change in these islands must add or update a ledger entry, update the named tests, or explicitly record why the invariant is covered elsewhere.

Miri target set:

- `cargo miri test -p bevy_ecs --lib`
- `MIRIFLAGS="-Zmiri-strict-provenance" cargo miri test -p bevy_ecs --lib`
- Focus targets when full-library Miri is too slow: `query::state`, `world::unsafe_world_cell`, `world::command_queue`, `storage::audit_tests`, `storage::blob_array`, `schedule::executor::multi_threaded`.

## file:crates/bevy_ecs/src/query/fetch.rs:324

machine:
  id: query-data-trait-contract
  operation: unsafe trait contract
  risk: S0

### Unsafe operation
Unsafe trait contract for `QueryData`, including fetch state construction, item fetch, release, and contiguous fetch.

### Required invariant
Implementors must only return references or pointers to components that match the declared access, storage type, archetype/table, and entity row. Mutable query data must not create aliased mutable references. Read-only query data must not mutate component or resource data. Any state cached during `set_archetype` or `set_table` must match the table row later passed to `fetch`.

### Who establishes it
Built-in `WorldQuery` and `QueryData` impls in `query/fetch.rs`, derive-generated query data, `QueryState::new_archetype`, and schedule/system access validation.

### Who can invalidate it
Manual `unsafe impl QueryData`, incorrect derive output, stale query cache updates, wrong storage type metadata, or direct calls to unchecked query APIs with overlapping mutable access.

### Tests covering it
`query::state::tests::unchecked_query_fetch_after_archetype_update_sees_moved_entity`, `query::state::tests::dense_query_over_option_is_buggy`, `query::state::tests::transmute_from_sparse_to_dense`, `query::state::tests::transmute_from_dense_to_sparse`, `query::state::tests::cannot_transmute_mutable_after_readonly`.

### Miri status
Targeted by the full-library Miri commands above. Current local status is recorded by the Pass 9 validation; Miri is required for future changes touching this contract.

### Risk
S0.

## file:crates/bevy_ecs/src/query/fetch.rs:1778

machine:
  id: query-read-fetch-table-sparse
  operation: pointer cast / unchecked index
  risk: S0

### Unsafe operation
`&T` fetch stores table or sparse-set component pointers and dereferences them as `T`.

### Required invariant
The component id must belong to `T`; table rows must be in bounds for the currently selected table; sparse-set lookups must use the same entity whose row is being fetched; no mutable reference to the same component may be live.

### Who establishes it
Component registration, archetype/table matching in `QueryState::new_archetype`, `FetchState`, and safe query entry points that validate world identity and update archetypes.

### Who can invalidate it
Stale manual query state, incorrect `set_archetype`/`set_table` selection, invalid component registration metadata, or unchecked query calls while mutable access exists.

### Tests covering it
`query::state::tests::unchecked_query_fetch_after_archetype_update_sees_moved_entity`, `storage::audit_tests::table_row_move_updates_swapped_entity_location`, `storage::audit_tests::randomized_storage_operations_preserve_locations_and_drops`.

### Miri status
Targeted by full-library Miri and strict-provenance Miri. This is a priority Miri target because it exercises table/sparse pointer provenance.

### Risk
S0.

## file:crates/bevy_ecs/src/query/fetch.rs:2260

machine:
  id: query-mut-fetch-aliasing
  operation: aliasing split / pointer cast
  risk: S0

### Unsafe operation
`&mut T` fetch produces unique mutable references from table or sparse-set storage.

### Required invariant
The query must have unique mutable access to component `T` for every fetched entity. Iterators yielding multiple mutable items must never produce the same entity/component pair twice. Parallel query paths must partition work without overlapping rows or sparse entries.

### Who establishes it
System parameter access registration, `FilteredAccess` conflict checks, unique entity collection APIs, query iterator partitioning, and scheduler conflict metadata.

### Who can invalidate it
Unchecked query APIs, custom `QueryData`, invalid `UniqueEntityArray` construction, scheduler conflict bugs, or query transmutation that widens mutable access.

### Tests covering it
`query::state::tests::cannot_transmute_immut_to_mut`, `query::state::tests::cannot_transmute_mutable_after_readonly`, `world::unsafe_world_cell::tests::as_unsafe_world_cell_readonly_component_mut_forbidden`, `world::unsafe_world_cell::tests::as_unsafe_world_cell_readonly_component_mut_by_id_forbidden`.

### Miri status
Targeted by full-library Miri and strict-provenance Miri. Add focused Miri runs for any query iterator or parallel query mutation patch.

### Risk
S0.

## file:crates/bevy_ecs/src/query/fetch.rs:3028

machine:
  id: query-option-fetch-search-space
  operation: unchecked optional fetch
  risk: S1

### Unsafe operation
`Option<T>` broadens query matching and conditionally calls the inner fetch. Dense iteration can bypass sparse-component presence in known edge cases.

### Required invariant
Optional fetch must return `None` only when the component is genuinely absent for the current entity, and `Some` only when the inner fetch is valid for the current table row/entity.

### Who establishes it
`Option<T>::set_archetype`, `Option<T>::set_table`, inner query fetch state, and query planner dense/archetypal iteration decisions.

### Who can invalidate it
Dense iteration over sparse optional data, query transmutation from dynamic entity fetches, or stale matched table/archetype caches.

### Tests covering it
`query::state::tests::dense_query_over_option_is_buggy` documents the current known wrong-result edge; Pass 2 query oracle tests cover sparse optional and `Has<Sparse>` shapes.

### Miri status
Miri can check pointer misuse here, but the known issue is S1 semantic correctness rather than UB.

### Risk
S1.

## file:crates/bevy_ecs/src/query/state.rs:150

machine:
  id: query-state-transmutation
  operation: type-state transmutation
  risk: S0

### Unsafe operation
`QueryState::as_transmuted_state` reinterprets one `QueryState` as another query shape after access subset checks.

### Required invariant
The new fetch and filter access must be a subset of the original access, and the copied matched table/archetype cache must remain valid for the new query semantics. Mutable access may not be introduced from read-only access.

### Who establishes it
`transmute`, `transmute_filtered`, `join`, `join_filtered`, access subset checks, and world-id validation.

### Who can invalidate it
Incorrect access subset logic, custom query data that underreports access, or changing cache semantics without updating transmutation rules.

### Tests covering it
`query::state::tests::cannot_transmute_to_include_data_not_in_original_query`, `query::state::tests::cannot_transmute_immut_to_mut`, `query::state::tests::transmute_with_different_world`, `query::state::tests::can_transmute_filtered_entity`.

### Miri status
Targeted by full-library Miri. Strict provenance is less central than access soundness, but should still be run after fetch-state layout changes.

### Risk
S0.

## file:crates/bevy_ecs/src/query/state.rs:185

machine:
  id: query-state-new-unchecked
  operation: unchecked query state initialization
  risk: S0

### Unsafe operation
`QueryState::new_unchecked` initializes query state without external caller proof that all access conflicts are sound.

### Required invariant
The `World` must remain the same world for this state, all archetypes must be registered before unchecked manual use, and mutable access must be externally unique.

### Who establishes it
Safe `World::query` constructors, `QueryState::new`, `SystemState` initialization, and explicit `update_archetypes` calls before manual methods.

### Who can invalidate it
Using a state with a different world, manual unchecked access before archetype updates, or reusing mutable query state while aliases exist.

### Tests covering it
`query::state::tests::right_world_get`, `query::state::tests::right_world_get_many`, `query::state::tests::right_world_get_many_mut`, `query::state::tests::unchecked_query_fetch_after_archetype_update_sees_moved_entity`.

### Miri status
Targeted by full-library Miri.

### Risk
S0.

## file:crates/bevy_ecs/src/query/state.rs:647

machine:
  id: query-state-new-archetype-cache
  operation: unchecked cache extension
  risk: S1

### Unsafe operation
`QueryState::new_archetype` updates matched archetype/table caches based on fetch and filter matching.

### Required invariant
The archetype must come from the same world and have a valid table id; fetch/filter state must have been initialized for that world; dense flags must agree with the actual query shape.

### Who establishes it
`update_archetypes_unsafe_world_cell`, `Archetypes` append-only ids, and `QueryData::matches_component_set`.

### Who can invalidate it
Calling with an archetype from another world, changing dense-iteration rules without updating sparse/optional behavior, or failing to update after new archetypes.

### Tests covering it
`query::state::tests::unchecked_query_fetch_after_archetype_update_sees_moved_entity`, `query::state::tests::transmute_from_sparse_to_dense`, `query::state::tests::dense_query_over_option_is_buggy`.

### Miri status
Targeted by full-library Miri; semantic regressions require oracle tests in addition to Miri.

### Risk
S1.

## file:crates/bevy_ecs/src/world/unsafe_world_cell.rs:93

machine:
  id: unsafe-world-cell-send-sync
  operation: unsafe impl Send / Sync
  risk: S0

### Unsafe operation
`UnsafeWorldCell` is marked `Send` and `Sync` even though it contains a raw world pointer.

### Required invariant
All component/resource access through the cell must be mediated by scheduler/system access permissions or by explicit unsafe caller proof. A readonly cell must reject mutable access at runtime.

### Who establishes it
`World::as_unsafe_world_cell`, readonly-cell construction, system access metadata, executor conflict checks, and `assert_allows_mutable_access`.

### Who can invalidate it
Adding mutable accessors that skip the readonly guard, executor bugs that run conflicting systems, or passing cells across threads without respecting declared access.

### Tests covering it
`world::unsafe_world_cell::tests::as_unsafe_world_cell_readonly_world_mut_forbidden`, `world::unsafe_world_cell::tests::as_unsafe_world_cell_readonly_resource_mut_forbidden`, `world::unsafe_world_cell::tests::as_unsafe_world_cell_readonly_component_mut_forbidden`, `world::unsafe_world_cell::tests::as_unsafe_world_cell_readonly_component_mut_by_id_forbidden`, `schedule::executor::multi_threaded::tests::run_condition_access_conflicting_with_running_systems_is_serialized`.

### Miri status
Targeted by full-library Miri. Thread sanitizer is also relevant for executor changes.

### Risk
S0.

## file:crates/bevy_ecs/src/world/unsafe_world_cell.rs:196

machine:
  id: unsafe-world-cell-world-mut
  operation: raw pointer to &mut World
  risk: S0

### Unsafe operation
`UnsafeWorldCell::world_mut` turns the stored raw pointer into `&mut World`.

### Required invariant
The cell must have been created from exclusive world access, no other world/component/resource references may be live, and readonly cells must reject this path.

### Who establishes it
Exclusive world methods, executor exclusive system paths, and the mutable-access guard.

### Who can invalidate it
Calling from a readonly cell, retaining references from prior cell access, or constructing multiple mutable world aliases from copied cells.

### Tests covering it
`world::unsafe_world_cell::tests::as_unsafe_world_cell_readonly_world_mut_forbidden`, `schedule::executor::multi_threaded::tests::check_spawn_exclusive_system_task_miri`.

### Miri status
Targeted by full-library Miri and the existing exclusive-system Miri regression test.

### Risk
S0.

## file:crates/bevy_ecs/src/world/unsafe_world_cell.rs:1283

machine:
  id: unsafe-entity-cell-component-pointer
  operation: raw component pointer lookup
  risk: S0

### Unsafe operation
`get_component` and companion tick helpers index table rows or sparse sets and return untyped component pointers.

### Required invariant
Entity location must be current; component id and storage type must match the registered component; table rows and sparse indices must be valid; aliasing must be enforced by the caller.

### Who establishes it
`Entities` location updates during archetype/table moves, `Table::move_row`, sparse-set dense/sparse reverse mapping, and caller access checks.

### Who can invalidate it
Stale entity locations after row moves, sparse-set reverse-map bugs, wrong storage type metadata, or unchecked mutable access aliases.

### Tests covering it
`storage::audit_tests::table_row_move_updates_swapped_entity_location`, `storage::audit_tests::randomized_storage_operations_preserve_locations_and_drops`, `storage::audit_tests::sparse_set_take_forgets_storage_but_transfers_drop_ownership`.

### Miri status
Priority target for strict-provenance Miri.

### Risk
S0.

## file:crates/bevy_ecs/src/world/command_queue.rs:24

machine:
  id: command-queue-type-erased-bytes
  operation: unaligned read / manual drop / function-pointer dispatch
  risk: S0

### Unsafe operation
`CommandMeta::consume_command_and_get_size` type-erases commands stored as raw bytes and later reads them with `read_unaligned`.

### Required invariant
Each metadata record must be adjacent to the exact command type it was created for. Cursor advancement must match the command size. Commands must be consumed exactly once, either applied or dropped.

### Who establishes it
`RawCommandQueue::push`, packed metadata layout, `apply_or_drop_queued`, queue drop, and panic-recovery cursor management.

### Who can invalidate it
Incorrect cursor math, appending partially consumed bytes incorrectly, reallocation while raw pointers are live, or panic recovery that replays consumed commands.

### Tests covering it
`world::command_queue::test::command_queue_handles_zst_large_aligned_and_drop_payloads`, `world::command_queue::test::test_uninit_bytes` under Miri, `world::command_queue::test::test_command_queue_inner_drop`, `world::command_queue::test::test_command_queue_inner_panic_safe`.

### Miri status
Priority Miri target because of uninitialized padding and unaligned reads. `test_uninit_bytes` is gated on `cfg(miri)`.

### Risk
S0.

## file:crates/bevy_ecs/src/world/command_queue.rs:251

machine:
  id: command-queue-panic-recovery
  operation: manual drop / panic recovery buffer splice
  risk: S0

### Unsafe operation
`apply_or_drop_queued` advances global and local cursors, consumes command bytes, catches panics, and preserves unapplied bytes.

### Required invariant
The global cursor must hide commands currently being applied from recursive flushes. On panic, already-consumed commands must not be replayed, and all unapplied commands queued before or during the panic must remain valid and in defined order.

### Who establishes it
The cursor protocol in `apply_or_drop_queued`, `panic_recovery`, and `append_unapplied_command_bytes`.

### Who can invalidate it
Nested `world.flush`, commands enqueuing commands before panicking, partially consumed append, or changing cursor reset order.

### Tests covering it
`world::command_queue::test::test_command_queue_inner_nested_panic_safe`, `world::command_queue::test::command_panics_after_enqueueing_command_preserves_unapplied_commands`, `world::command_queue::test::command_panics_before_enqueueing_command_preserves_only_existing_unapplied_commands`, `storage::audit_tests::command_panic_leaves_storage_model_checkable_for_later_commands`.

### Miri status
Targeted by full-library Miri. Panic-unwind behavior also needs normal test coverage because Miri can be slow here.

### Risk
S0.

## file:crates/bevy_ecs/src/world/command_queue.rs:340

machine:
  id: command-queue-partial-append
  operation: raw byte slice transfer
  risk: S1

### Unsafe operation
`append_unapplied_command_bytes` appends only unapplied command bytes from a possibly partially consumed queue.

### Required invariant
`other.cursor` must point to a command boundary or the end of the byte buffer, and `panic_recovery` must be empty outside active unwind handling.

### Who establishes it
`apply_or_drop_queued` cursor updates and all public `append` callers.

### Who can invalidate it
Manually mutating `cursor`, future APIs that expose partial application, or appending during panic recovery.

### Tests covering it
`world::command_queue::test::append_partially_consumed_queue_moves_only_unapplied_commands`, `world::command_queue::test::raw_commands_append_partially_consumed_queue_moves_only_unapplied_commands`, `world::command_queue::test::append_fully_consumed_queue_appends_nothing_and_resets_source`.

### Miri status
Targeted by full-library Miri.

### Risk
S1.

## file:crates/bevy_ecs/src/storage/blob_array.rs:46

machine:
  id: blob-array-layout-drop-contract
  operation: manual allocation / manual drop function
  risk: S0

### Unsafe operation
`BlobArray::with_capacity` stores an allocation and an optional type-erased drop function for values with a shared layout.

### Required invariant
All values stored in the array must match `item_layout` and the drop function. If `drop` is `None`, values must not require drop or the leak must be intentional.

### Who establishes it
Component registration, `ComponentDescriptor`, `TableBuilder`, sparse-set creation, and bundle writers.

### Who can invalidate it
Registering wrong layout/drop metadata, writing a different type into a column, or replacing values with mismatched drop functions.

### Tests covering it
`storage::blob_array::tests::make_sure_zst_components_get_dropped`, `storage::audit_tests::randomized_storage_operations_preserve_locations_and_drops`, `storage::audit_tests::drop_panic_during_clear_does_not_double_drop_on_world_drop`.

### Miri status
Priority strict-provenance Miri target.

### Risk
S0.

## file:crates/bevy_ecs/src/storage/blob_array.rs:187

machine:
  id: blob-array-clear-unwind
  operation: manual drop / panic guard
  risk: S0

### Unsafe operation
`BlobArray::clear`, `drop`, `drop_last_element`, and `replace_unchecked` call type-erased drop functions while mutating internal drop state for unwind safety.

### Required invariant
Elements must be dropped at most once. If a drop implementation panics, the array must not later observe or drop the same element again.

### Who establishes it
Temporary `self.drop = None` guards, `OnDrop` guards in replacement, table clear/drop paths, and world clear/drop sequencing.

### Who can invalidate it
Changing drop guard ordering, failing to restore drop state on success, or reading entries after a panic left length/drop metadata in recovery state.

### Tests covering it
`storage::audit_tests::drop_panic_during_clear_does_not_double_drop_on_world_drop`, `storage::audit_tests::randomized_storage_operations_preserve_locations_and_drops`.

### Miri status
Targeted by full-library Miri; panic-path tests must remain in normal test runs too.

### Risk
S0.

## file:crates/bevy_ecs/src/storage/blob_array.rs:286

machine:
  id: blob-array-realloc
  operation: manual realloc / raw pointer replacement
  risk: S0

### Unsafe operation
`BlobArray::realloc` grows or moves the backing allocation for component data and tick arrays.

### Required invariant
The old capacity and new capacity must match the real allocation state. No live pointers into the old allocation may be used after realloc. ZST arrays must not allocate.

### Who establishes it
`Table::realloc_columns`, `Table::reserve`, sparse-set insert growth, and abort-on-panic guards in table allocation.

### Who can invalidate it
Allocator failure handling changes, wrong capacity accounting, retaining raw component pointers across structural changes, or reallocation during a panic without aborting.

### Tests covering it
`storage::audit_tests::randomized_storage_operations_preserve_locations_and_drops`, `storage::audit_tests::table_row_move_updates_swapped_entity_location`. Direct allocator-fault injection is not present in this environment.

### Miri status
Priority strict-provenance Miri target.

### Risk
S0.

## file:crates/bevy_ecs/src/storage/blob_array.rs:406

machine:
  id: blob-array-swap-remove
  operation: unchecked move / manual ownership transfer
  risk: S0

### Unsafe operation
`swap_remove_unchecked` and variants move bytes between array slots and return or drop the removed value.

### Required invariant
Both indices must be in bounds. Nonoverlapping variants must not receive equal indices. The returned `OwningPtr` must be dropped exactly once if the value requires drop.

### Who establishes it
`Table::swap_remove_unchecked`, `Table::move_row`, `ComponentSparseSet::remove`, `ComponentSparseSet::remove_and_forget`, and bundle removal/take paths.

### Who can invalidate it
Wrong last-element index, stale row/entity mapping, or forgetting/dropping returned ownership incorrectly.

### Tests covering it
`storage::audit_tests::table_row_move_updates_swapped_entity_location`, `storage::audit_tests::sparse_set_take_forgets_storage_but_transfers_drop_ownership`, `storage::audit_tests::randomized_storage_operations_preserve_locations_and_drops`.

### Miri status
Priority strict-provenance Miri target.

### Risk
S0.

## file:crates/bevy_ecs/src/storage/table/column.rs:137

machine:
  id: table-column-realloc-triplet
  operation: multi-array realloc
  risk: S0

### Unsafe operation
`Column::realloc` reallocates component data, added ticks, changed ticks, and optional changed-by arrays.

### Required invariant
All column arrays must share capacity and logical length. If one allocation fails or panics, table capacity must not be left in a state that causes invalid drops.

### Who establishes it
`Table::realloc_columns` and its allocation guards.

### Who can invalidate it
Reallocating only part of a column, incorrect table capacity updates, or using old array pointers after growth.

### Tests covering it
`storage::audit_tests::randomized_storage_operations_preserve_locations_and_drops`, `storage::audit_tests::drop_panic_during_clear_does_not_double_drop_on_world_drop`.

### Miri status
Priority strict-provenance Miri target.

### Risk
S0.

## file:crates/bevy_ecs/src/storage/table/mod.rs:226

machine:
  id: table-swap-remove-row
  operation: unchecked row swap-remove
  risk: S0

### Unsafe operation
`Table::swap_remove_unchecked` removes one entity row and swap-moves the last row across every column.

### Required invariant
The row must be in bounds, all columns must have the same length as `entities`, and the swapped entity location must be repaired by the caller.

### Who establishes it
Entity despawn/remove paths, `Table::move_row`, `BundleInserter`, `BundleRemover`, and `Entities` location updates.

### Who can invalidate it
Skipping swapped-entity location updates, mismatched column lengths, or using a stale `TableRow` after structural changes.

### Tests covering it
`storage::audit_tests::table_row_move_updates_swapped_entity_location`, `storage::audit_tests::randomized_storage_operations_preserve_locations_and_drops`, `world::entity_access::tests::spawned_after_swap_remove`.

### Miri status
Priority strict-provenance Miri target.

### Risk
S0.

## file:crates/bevy_ecs/src/storage/table/mod.rs:464

machine:
  id: table-realloc-columns
  operation: table-wide allocation guard
  risk: S0

### Unsafe operation
`Table::realloc_columns` grows all component columns after reserving entity capacity and uses panic guards to avoid unsafe capacity mismatch.

### Required invariant
`entities` capacity and all column capacities must stay synchronized. If column allocation panics after entity capacity changes, the process must abort rather than later dropping with false capacity assumptions.

### Who establishes it
`Table::reserve`, `Table::allocate`, and the guard inside `realloc_columns`.

### Who can invalidate it
Changing reserve order, disabling the abort guard, or adding a column array that is not included in the capacity synchronization.

### Tests covering it
`storage::audit_tests::randomized_storage_operations_preserve_locations_and_drops`. Allocator-failure injection is deferred; no direct table-reallocation panic test exists.

### Miri status
Targeted by full-library Miri; allocator-failure behavior needs a separate fault-injection harness.

### Risk
S0.

## file:crates/bevy_ecs/src/storage/table/mod.rs:779

machine:
  id: table-move-row-between-tables
  operation: unchecked disjoint table borrow / row move
  risk: S0

### Unsafe operation
`Table::move_row` obtains two table borrows, allocates a destination row, swap-removes the source row, and moves or drops column values depending on the target table shape.

### Required invariant
Source and destination tables must be distinct and valid. Entity locations for moved and swapped entities must be repaired. Components not present in the destination must be dropped iff `DROP` is true.

### Who establishes it
`BundleInserter`, `BundleRemover`, entity insert/remove/take paths, and `Tables::get_many_mut`.

### Who can invalidate it
Passing equal table ids to the nonoverlapping path, not updating swapped entity locations, or misclassifying columns as move/drop/ignore.

### Tests covering it
`storage::audit_tests::table_row_move_updates_swapped_entity_location`, `storage::audit_tests::randomized_storage_operations_preserve_locations_and_drops`, `storage::audit_tests::sparse_set_take_forgets_storage_but_transfers_drop_ownership`.

### Miri status
Priority strict-provenance Miri target.

### Risk
S0.

## file:crates/bevy_ecs/src/storage/sparse_set.rs:222

machine:
  id: sparse-set-insert
  operation: unchecked dense insert / sparse index update
  risk: S0

### Unsafe operation
`ComponentSparseSet::insert` writes component bytes into dense storage and updates sparse entity-index mappings.

### Required invariant
The dense component value, dense entity list, and sparse reverse map must remain exactly synchronized. Replacing an existing sparse component must drop the old value exactly once.

### Who establishes it
Bundle insertion, `EntityWorldMut::insert`, required component initialization, and sparse-set storage APIs.

### Who can invalidate it
Wrong entity index, stale sparse index, mismatched component layout/drop metadata, or missing drop on replace.

### Tests covering it
`storage::audit_tests::randomized_storage_operations_preserve_locations_and_drops`, `storage::audit_tests::high_entity_index_sparse_storage_reports_sparse_slots_and_clears`, `observer::tests::observer_reentrant_storage_mutation_updates_table_and_sparse_storage`.

### Miri status
Priority strict-provenance Miri target.

### Risk
S0.

## file:crates/bevy_ecs/src/storage/sparse_set.rs:388

machine:
  id: sparse-set-remove-and-forget
  operation: manual ownership transfer / unchecked swap-remove
  risk: S0

### Unsafe operation
`remove_and_forget` removes a sparse component and returns ownership of the value without dropping it.

### Required invariant
The sparse reverse map must be updated for any swapped entity, the returned pointer must be consumed by the caller, and the removed value must not remain reachable from storage.

### Who establishes it
`EntityWorldMut::take`, bundle removal with `BundleFromComponents`, and sparse-set dense/sparse mapping updates.

### Who can invalidate it
Dropping storage after returning the pointer, failing to update the swapped entity's sparse index, or caller forgetting to drop the returned value when ownership is not transferred into a Rust value.

### Tests covering it
`storage::audit_tests::sparse_set_take_forgets_storage_but_transfers_drop_ownership`, `storage::audit_tests::randomized_storage_operations_preserve_locations_and_drops`.

### Miri status
Priority strict-provenance Miri target.

### Risk
S0.

## file:crates/bevy_ecs/src/archetype.rs:609

machine:
  id: archetype-allocate-location
  operation: unchecked row allocation / location bookkeeping
  risk: S0

### Unsafe operation
`Archetype::allocate` appends entity metadata and returns an `EntityLocation`.

### Required invariant
The table row must correspond to the entity's table storage, the archetype row must be in bounds, and the entity allocator must record the same location.

### Who establishes it
Spawn paths, bundle insert/remove transitions, table allocation, and `Entities` location updates.

### Who can invalidate it
Allocating in an archetype without a matching table row, stale location updates after swap-remove, or entity generation reuse bugs.

### Tests covering it
`storage::audit_tests::randomized_storage_operations_preserve_locations_and_drops`, `storage::audit_tests::table_row_move_updates_swapped_entity_location`.

### Miri status
Targeted by full-library Miri.

### Risk
S0.

## file:crates/bevy_ecs/src/archetype.rs:880

machine:
  id: archetypes-disjoint-mut
  operation: unchecked disjoint mutable borrow
  risk: S0

### Unsafe operation
`Archetypes::get_maybe_disjoint_mut` returns mutable references to two archetypes when they are distinct.

### Required invariant
The two ids must be valid and either distinct or handled as the documented same-archetype case without producing aliases.

### Who establishes it
Bundle insert/remove transition code and collision checks before calling.

### Who can invalidate it
Calling with equal ids through a path that assumes disjointness, or holding other mutable references to the same archetype.

### Tests covering it
`storage::audit_tests::randomized_storage_operations_preserve_locations_and_drops`, hierarchy/relationship insert/remove tests, and entity access panic-scope tests.

### Miri status
Targeted by full-library Miri.

### Risk
S0.

## file:crates/bevy_ecs/src/schedule/executor/multi_threaded.rs:489

machine:
  id: executor-ready-spawn-loop
  operation: unchecked system access under scheduler conflict model
  risk: S0

### Unsafe operation
`spawn_system_tasks` evaluates readiness and spawns systems using precomputed conflict metadata.

### Required invariant
No two systems or run conditions with conflicting world access may run at the same time. Ready/running/completed bitsets must reflect the actual task state.

### Who establishes it
Schedule graph initialization, conflict precomputation, executor state lock, completion queue, and dependency counters.

### Who can invalidate it
Incorrect conflict metadata, missed completion handling, condition evaluation while a conflicting system is running, or ApplyDeferred state bugs.

### Tests covering it
`schedule::executor::multi_threaded::tests::run_condition_access_conflicting_with_running_systems_is_serialized`, `schedule::executor::multi_threaded::tests::skipped_systems_notify_dependents`, scheduler property tests from Pass 5.

### Miri status
Targeted by full-library Miri. Thread sanitizer is the preferred dynamic tool for future executor changes.

### Risk
S0.

## file:crates/bevy_ecs/src/schedule/executor/multi_threaded.rs:716

machine:
  id: executor-spawn-system-task
  operation: raw system pointer task capture
  risk: S0

### Unsafe operation
`spawn_system_task` takes a system out of `SyncUnsafeCell` and runs it on the task pool.

### Required invariant
The system must not be accessed elsewhere while running, and the `UnsafeWorldCell` passed to it must cover only the access declared by the system.

### Who establishes it
Running-system bitsets, executor lock protocol, and schedule conflict metadata.

### Who can invalidate it
Spawning the same system twice, applying deferred buffers while the system is still running, or using stale access metadata.

### Tests covering it
`schedule::executor::multi_threaded::tests::run_condition_access_conflicting_with_running_systems_is_serialized`, Pass 5 many-system executor tests.

### Miri status
Targeted by full-library Miri. Thread sanitizer best-effort command is required for executor patches.

### Risk
S0.

## file:crates/bevy_ecs/src/schedule/executor/multi_threaded.rs:761

machine:
  id: executor-spawn-exclusive-system-task
  operation: temporary exclusive world access in async task
  risk: S0

### Unsafe operation
`spawn_exclusive_system_task` runs an exclusive system and must not let `&mut World` escape across async task boundaries.

### Required invariant
No other world access may run while the exclusive system is active, and any mutable world borrow must end before the task yields/completes.

### Who establishes it
Executor exclusive-system scheduling, non-Send/main-thread placement, and the task closure structure.

### Who can invalidate it
Capturing `&mut World` in an async block beyond its intended lifetime or scheduling other systems concurrently with an exclusive system.

### Tests covering it
`schedule::executor::multi_threaded::tests::check_spawn_exclusive_system_task_miri`.

### Miri status
Explicitly Miri-targeted by the existing regression test.

### Risk
S0.

## file:crates/bevy_ecs/src/schedule/executor/multi_threaded.rs:916

machine:
  id: executor-condition-evaluation
  operation: unchecked readonly system run
  risk: S0

### Unsafe operation
`evaluate_and_fold_conditions` calls `readonly_run_unsafe` on run-condition systems.

### Required invariant
The world cell must have read access to every resource/component read by the condition, and no conflicting mutable system may be running at the same time. Conditions must all run even when earlier conditions return false.

### Who establishes it
Condition access metadata, executor ready checks, and schedule graph condition registration.

### Who can invalidate it
Underreported condition access, evaluating conditions outside the conflict gate, or converting conditions to short-circuit execution.

### Tests covering it
`schedule::executor::multi_threaded::tests::run_condition_access_conflicting_with_running_systems_is_serialized`, condition tests covering non-short-circuit semantics.

### Miri status
Targeted by full-library Miri. Thread sanitizer best-effort command is relevant.

### Risk
S0.

## file:crates/bevy_ecs/src/bundle/mod.rs:207

machine:
  id: bundle-trait-contract
  operation: unsafe trait contract / component extraction
  risk: S0

### Unsafe operation
`Bundle`, `BundleFromComponents`, and `DynamicBundle` define unsafe contracts for component ids, pointer extraction, reconstruction, and apply effects.

### Required invariant
Bundle component ids must match the extracted pointer types and order. Duplicate components must be rejected. `apply_effect` must not read moved or dropped data.

### Who establishes it
Built-in bundle impls, derive macro output, `BundleInfo::new`, duplicate component checks, and `BundleWriter`.

### Who can invalidate it
Manual unsafe bundle impls, incorrect derive output, duplicate component ids, or changing tuple component traversal order.

### Tests covering it
Bundle tests under `crates/bevy_ecs/src/bundle/tests.rs`, `storage::audit_tests::randomized_storage_operations_preserve_locations_and_drops`, required-component tests in storage audit.

### Miri status
Targeted by full-library Miri.

### Risk
S0.

## file:crates/bevy_ecs/src/bundle/insert.rs:506

machine:
  id: bundle-insert-archetype-transition
  operation: unchecked archetype/table transition planning
  risk: S0

### Unsafe operation
`insert_bundle_into_archetype` creates or reuses archetype/table transition edges and prepares storage for inserted bundle components.

### Required invariant
All component ids must be valid and initialized in the target storage. Required components must be included exactly once. Transition edges must point to archetypes with the expected component set.

### Who establishes it
`BundleInfo`, component registrators, archetype edge caches, and table/sparse storage preparation.

### Who can invalidate it
Invalid bundle metadata, required-component recursion bugs, stale edge caches, or missing sparse/table storage setup.

### Tests covering it
`storage::audit_tests::randomized_storage_operations_preserve_locations_and_drops`, required-component cases in storage audit, hierarchy/relationship replacement tests.

### Miri status
Targeted by full-library Miri.

### Risk
S0.

## file:crates/bevy_ecs/src/bundle/remove.rs:340

machine:
  id: bundle-remove-archetype-transition
  operation: unchecked archetype/table transition planning
  risk: S0

### Unsafe operation
`remove_bundle_from_archetype` computes the target archetype after removing bundle components and sets up table movement/drop behavior.

### Required invariant
All removed component ids must exist when `require_all` is true. Removed sparse components must be dropped or transferred exactly once. Dense components missing from the target table must be dropped iff the removal path requires it.

### Who establishes it
`BundleRemover`, `BundleInfo`, table move/drop paths, and sparse-set removal paths.

### Who can invalidate it
Wrong component status, stale archetype edges, incorrect `DROP` mode, or `take` paths that forget returned ownership.

### Tests covering it
`storage::audit_tests::sparse_set_take_forgets_storage_but_transfers_drop_ownership`, `storage::audit_tests::randomized_storage_operations_preserve_locations_and_drops`.

### Miri status
Targeted by full-library Miri.

### Risk
S0.

## file:crates/bevy_ecs/src/bundle/spawner.rs:91

machine:
  id: bundle-spawner-spawn-at
  operation: cached raw world/table/archetype pointer use
  risk: S0

### Unsafe operation
`BundleSpawner::spawn_at` uses cached world, archetype, and table pointers to spawn entities efficiently.

### Required invariant
The cached pointers must not be invalidated by structural changes while the spawner is in use. If commands are flushed through the spawner, the spawner must be dropped afterward.

### Who establishes it
`BundleSpawner` construction, exclusive world access, no structural graph mutation through the cached pointers, and `flush_commands` invalidation rules.

### Who can invalidate it
Reusing a spawner after flushing commands, mutating archetype/table storage externally while cached pointers are live, or spawning with a bundle id that does not match the bundle type.

### Tests covering it
Spawn batch coverage in storage audit and existing spawn tests. Any future spawner change should add a focused Miri target.

### Miri status
Targeted by full-library Miri; no dedicated focused test yet.

### Risk
S0.

## file:crates/bevy_ecs/src/bundle/writer.rs:92

machine:
  id: bundle-writer-scratch
  operation: erased component pointer stash / manual drop
  risk: S0

### Unsafe operation
`BundleScratch` and `BundleWriter` store component pointers before writing them into an entity or manually dropping them.

### Required invariant
Every pushed component pointer must match its component id and be written or manually dropped exactly once. The same world/components registry must be used for all operations.

### Who establishes it
`BundleWriter::push_component`, `push_component_by_id`, `write`, `manual_drop`, and the caller's component registry.

### Who can invalidate it
Pushing a pointer with the wrong component id, leaking scratch data after an error, or writing into a different world than the one used to allocate ids.

### Tests covering it
Bundle writer tests and storage panic/drop audit tests. Future writer patches need a focused drop-count regression.

### Miri status
Targeted by full-library Miri.

### Risk
S0.
