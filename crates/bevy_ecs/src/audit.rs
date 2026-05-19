//! Audit-only counters for ECS microscope passes.

#[cfg(feature = "bevy_ecs_audit")]
mod imp {
    use core::sync::atomic::{AtomicUsize, Ordering};

    macro_rules! counters {
        ($($name:ident),* $(,)?) => {
            $(
                static $name: AtomicUsize = AtomicUsize::new(0);
            )*

            fn reset_all() {
                $(
                    $name.store(0, Ordering::Relaxed);
                )*
            }
        };
    }

    counters!(
        QUERY_UPDATE_ARCHETYPES,
        QUERY_ARCHETYPES_SCANNED,
        QUERY_NEW_ARCHETYPE_CALLS,
        QUERY_MATCHED_ARCHETYPES,
        QUERY_MATCHED_TABLES,
        TABLE_ALLOCATIONS,
        TABLE_ROW_MOVES,
        TABLE_ROW_SWAP_REMOVES,
        SPARSE_SET_INSERTS,
        SPARSE_SET_REMOVES,
        SPARSE_SET_GETS,
        COMMAND_QUEUE_BYTES_PUSHED,
        COMMAND_QUEUE_COMMANDS_PUSHED,
        COMMAND_QUEUE_APPLIES,
        COMMAND_QUEUE_RECURSIVE_APPLIES,
        COMMAND_QUEUE_APPEND_CALLS,
        COMMAND_QUEUE_BYTES_APPENDED,
        COMMAND_QUEUE_REALLOCATIONS,
        COMMAND_QUEUE_WORLD_FLUSHES,
        SCHEDULER_LOCK_FAILURES,
        SCHEDULER_READY_SCAN_PASSES,
        SCHEDULER_READY_SYSTEMS_SCANNED,
        SCHEDULER_CONDITION_EVALUATIONS,
        SCHEDULER_TASKS_SPAWNED,
        SCHEDULER_EXCLUSIVE_TASKS_SPAWNED,
        SCHEDULER_NON_SEND_TASKS_SPAWNED,
        SCHEDULER_READY_TO_RUN_NANOS,
        SCHEDULER_READY_TO_RUN_SAMPLES,
        SCHEDULER_IDLE_READY_NANOS,
        SCHEDULER_LOCK_HOLD_NANOS,
        SCHEDULER_LOCK_HOLD_SAMPLES,
        SCHEDULER_APPLY_DEFERRED_BITSET_REUSES,
        APPLY_DEFERRED_CALLS,
        APPLY_DEFERRED_SYSTEMS,
        APPLY_DEFERRED_NANOS,
        OBSERVER_TRIGGERS,
        OBSERVER_NO_OBSERVERS,
        OBSERVER_DISPATCHES,
        OBSERVER_GLOBAL_DISPATCHES,
        OBSERVER_ENTITY_DISPATCHES,
        OBSERVER_COMPONENT_DISPATCHES,
        OBSERVER_ENTITY_COMPONENT_DISPATCHES,
        OBSERVER_DEDUPED,
        OBSERVER_MAX_TRIGGER_DEPTH,
        RELATIONSHIP_ADDS,
        RELATIONSHIP_REMOVES,
        RELATIONSHIP_COLLECTION_SCAN_LEN,
        CHANGE_TICK_CHECKS,
        CHANGE_TICK_CHECK_SKIPPED_UNDER_THRESHOLD,
        CHANGE_TICK_CHECK_NANOS,
        CHANGE_TICK_CHECK_TABLES,
        CHANGE_TICK_CHECK_EMPTY_TABLES,
        CHANGE_TICK_CHECK_SPARSE_SETS,
        CHANGE_TICK_CHECK_EMPTY_SPARSE_SETS,
        CHANGE_TICK_CHECK_COMPONENT_TICKS,
    );

    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
    pub struct AuditCounters {
        pub query_update_archetypes: usize,
        pub query_archetypes_scanned: usize,
        pub query_new_archetype_calls: usize,
        pub query_matched_archetypes: usize,
        pub query_matched_tables: usize,
        pub table_allocations: usize,
        pub table_row_moves: usize,
        pub table_row_swap_removes: usize,
        pub sparse_set_inserts: usize,
        pub sparse_set_removes: usize,
        pub sparse_set_gets: usize,
        pub command_queue_bytes_pushed: usize,
        pub command_queue_commands_pushed: usize,
        pub command_queue_applies: usize,
        pub command_queue_recursive_applies: usize,
        pub command_queue_append_calls: usize,
        pub command_queue_bytes_appended: usize,
        pub command_queue_reallocations: usize,
        pub command_queue_world_flushes: usize,
        pub scheduler_lock_failures: usize,
        pub scheduler_ready_scan_passes: usize,
        pub scheduler_ready_systems_scanned: usize,
        pub scheduler_condition_evaluations: usize,
        pub scheduler_tasks_spawned: usize,
        pub scheduler_exclusive_tasks_spawned: usize,
        pub scheduler_non_send_tasks_spawned: usize,
        pub scheduler_ready_to_run_nanos: usize,
        pub scheduler_ready_to_run_samples: usize,
        pub scheduler_idle_ready_nanos: usize,
        pub scheduler_lock_hold_nanos: usize,
        pub scheduler_lock_hold_samples: usize,
        pub scheduler_apply_deferred_bitset_reuses: usize,
        pub apply_deferred_calls: usize,
        pub apply_deferred_systems: usize,
        pub apply_deferred_nanos: usize,
        pub observer_triggers: usize,
        pub observer_no_observers: usize,
        pub observer_dispatches: usize,
        pub observer_global_dispatches: usize,
        pub observer_entity_dispatches: usize,
        pub observer_component_dispatches: usize,
        pub observer_entity_component_dispatches: usize,
        pub observer_deduped: usize,
        pub observer_max_trigger_depth: usize,
        pub relationship_adds: usize,
        pub relationship_removes: usize,
        pub relationship_collection_scan_len: usize,
        pub change_tick_checks: usize,
        pub change_tick_check_skipped_under_threshold: usize,
        pub change_tick_check_nanos: usize,
        pub change_tick_check_tables: usize,
        pub change_tick_check_empty_tables: usize,
        pub change_tick_check_sparse_sets: usize,
        pub change_tick_check_empty_sparse_sets: usize,
        pub change_tick_check_component_ticks: usize,
    }

    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
    pub struct StorageMetrics {
        pub archetype_count: usize,
        pub empty_archetype_count: usize,
        pub archetype_edge_entries: usize,
        pub archetype_edge_slots: usize,
        pub archetype_edge_capacity: usize,
        pub table_count: usize,
        pub empty_table_count: usize,
        pub table_entity_count: usize,
        pub table_entity_capacity: usize,
        pub table_column_count: usize,
        pub sparse_set_count: usize,
        pub sparse_set_entity_count: usize,
        pub sparse_set_entity_capacity: usize,
        pub sparse_set_sparse_slots: usize,
        pub sparse_set_sparse_capacity: usize,
    }

    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
    pub struct MemoryMetrics {
        pub estimated_retained_bytes: usize,
        pub entity_meta_len: usize,
        pub entity_meta_capacity: usize,
        pub entity_meta_retained_bytes: usize,
        pub archetype_count: usize,
        pub empty_archetype_count: usize,
        pub archetype_edge_entries: usize,
        pub archetype_edge_slots: usize,
        pub archetype_edge_capacity: usize,
        pub archetype_retained_bytes: usize,
        pub table_count: usize,
        pub empty_table_count: usize,
        pub table_entity_count: usize,
        pub table_entity_capacity: usize,
        pub table_entity_retained_bytes: usize,
        pub table_column_count: usize,
        pub table_column_retained_bytes: usize,
        pub sparse_set_count: usize,
        pub sparse_set_entity_count: usize,
        pub sparse_set_entity_capacity: usize,
        pub sparse_set_sparse_slots: usize,
        pub sparse_set_sparse_capacity: usize,
        pub sparse_set_retained_bytes: usize,
        pub command_queue_len_bytes: usize,
        pub command_queue_capacity_bytes: usize,
        pub command_queue_panic_recovery_len_bytes: usize,
        pub command_queue_panic_recovery_capacity_bytes: usize,
        pub observer_event_cache_entries: usize,
        pub observer_event_cache_capacity: usize,
        pub observer_runner_entries: usize,
        pub observer_runner_capacity: usize,
        pub observer_retained_bytes: usize,
    }

    #[inline]
    fn add(counter: &AtomicUsize, value: usize) {
        counter.fetch_add(value, Ordering::Relaxed);
    }

    #[inline]
    fn inc(counter: &AtomicUsize) {
        add(counter, 1);
    }

    #[inline]
    fn load(counter: &AtomicUsize) -> usize {
        counter.load(Ordering::Relaxed)
    }

    #[inline]
    fn max(counter: &AtomicUsize, value: usize) {
        let mut current = counter.load(Ordering::Relaxed);
        while current < value {
            match counter.compare_exchange_weak(
                current,
                value,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(next) => current = next,
            }
        }
    }

    pub struct ObserverTriggerGuard;

    static OBSERVER_TRIGGER_DEPTH: AtomicUsize = AtomicUsize::new(0);

    impl Drop for ObserverTriggerGuard {
        fn drop(&mut self) {
            OBSERVER_TRIGGER_DEPTH.fetch_sub(1, Ordering::Relaxed);
        }
    }

    #[inline]
    pub(crate) fn query_update_archetypes(archetypes_scanned: usize) {
        inc(&QUERY_UPDATE_ARCHETYPES);
        add(&QUERY_ARCHETYPES_SCANNED, archetypes_scanned);
    }

    #[inline]
    pub(crate) fn query_new_archetype(matched_archetype: bool, matched_table: bool) {
        inc(&QUERY_NEW_ARCHETYPE_CALLS);
        if matched_archetype {
            inc(&QUERY_MATCHED_ARCHETYPES);
        }
        if matched_table {
            inc(&QUERY_MATCHED_TABLES);
        }
    }

    #[inline]
    pub(crate) fn table_allocate() {
        inc(&TABLE_ALLOCATIONS);
    }

    #[inline]
    pub(crate) fn table_move_row() {
        inc(&TABLE_ROW_MOVES);
    }

    #[inline]
    pub(crate) fn table_swap_remove() {
        inc(&TABLE_ROW_SWAP_REMOVES);
    }

    #[inline]
    pub(crate) fn sparse_set_insert() {
        inc(&SPARSE_SET_INSERTS);
    }

    #[inline]
    pub(crate) fn sparse_set_remove() {
        inc(&SPARSE_SET_REMOVES);
    }

    #[inline]
    pub(crate) fn sparse_set_get() {
        inc(&SPARSE_SET_GETS);
    }

    #[inline]
    pub(crate) fn command_queue_push(bytes: usize) {
        inc(&COMMAND_QUEUE_COMMANDS_PUSHED);
        add(&COMMAND_QUEUE_BYTES_PUSHED, bytes);
    }

    #[inline]
    pub(crate) fn command_queue_apply(start: usize, _: usize) {
        inc(&COMMAND_QUEUE_APPLIES);
        if start > 0 {
            inc(&COMMAND_QUEUE_RECURSIVE_APPLIES);
        }
    }

    #[inline]
    pub(crate) fn command_queue_append(bytes: usize) {
        inc(&COMMAND_QUEUE_APPEND_CALLS);
        add(&COMMAND_QUEUE_BYTES_APPENDED, bytes);
    }

    #[inline]
    pub(crate) fn command_queue_reallocation() {
        inc(&COMMAND_QUEUE_REALLOCATIONS);
    }

    #[inline]
    pub(crate) fn command_queue_world_flush() {
        inc(&COMMAND_QUEUE_WORLD_FLUSHES);
    }

    #[inline]
    pub(crate) fn scheduler_lock_failed() {
        inc(&SCHEDULER_LOCK_FAILURES);
    }

    #[inline]
    pub(crate) fn scheduler_ready_scan(systems_scanned: usize) {
        inc(&SCHEDULER_READY_SCAN_PASSES);
        add(&SCHEDULER_READY_SYSTEMS_SCANNED, systems_scanned);
    }

    #[inline]
    pub(crate) fn scheduler_condition_evaluations(count: usize) {
        add(&SCHEDULER_CONDITION_EVALUATIONS, count);
    }

    #[inline]
    pub(crate) fn scheduler_task_spawned(is_exclusive: bool, is_send: bool) {
        inc(&SCHEDULER_TASKS_SPAWNED);
        if is_exclusive {
            inc(&SCHEDULER_EXCLUSIVE_TASKS_SPAWNED);
        }
        if !is_send {
            inc(&SCHEDULER_NON_SEND_TASKS_SPAWNED);
        }
    }

    #[inline]
    pub(crate) fn scheduler_ready_to_run_delay(elapsed_nanos: usize) {
        inc(&SCHEDULER_READY_TO_RUN_SAMPLES);
        add(&SCHEDULER_READY_TO_RUN_NANOS, elapsed_nanos);
    }

    #[inline]
    pub(crate) fn scheduler_idle_ready_wait(elapsed_nanos: usize) {
        add(&SCHEDULER_IDLE_READY_NANOS, elapsed_nanos);
    }

    #[inline]
    pub(crate) fn scheduler_lock_held(elapsed_nanos: usize) {
        inc(&SCHEDULER_LOCK_HOLD_SAMPLES);
        add(&SCHEDULER_LOCK_HOLD_NANOS, elapsed_nanos);
    }

    #[inline]
    pub(crate) fn scheduler_apply_deferred_bitset_reuse() {
        inc(&SCHEDULER_APPLY_DEFERRED_BITSET_REUSES);
    }

    #[inline]
    pub(crate) fn apply_deferred_finished(system_count: usize, elapsed_nanos: usize) {
        inc(&APPLY_DEFERRED_CALLS);
        add(&APPLY_DEFERRED_SYSTEMS, system_count);
        add(&APPLY_DEFERRED_NANOS, elapsed_nanos);
    }

    #[inline]
    pub(crate) fn observer_trigger_scope() -> ObserverTriggerGuard {
        inc(&OBSERVER_TRIGGERS);
        let depth = OBSERVER_TRIGGER_DEPTH.fetch_add(1, Ordering::Relaxed) + 1;
        max(&OBSERVER_MAX_TRIGGER_DEPTH, depth);
        ObserverTriggerGuard
    }

    #[inline]
    pub(crate) fn observer_no_observers() {
        inc(&OBSERVER_NO_OBSERVERS);
    }

    #[inline]
    pub(crate) fn observer_dispatch() {
        inc(&OBSERVER_DISPATCHES);
    }

    #[inline]
    pub(crate) fn observer_global_dispatch() {
        observer_dispatch();
        inc(&OBSERVER_GLOBAL_DISPATCHES);
    }

    #[inline]
    pub(crate) fn observer_entity_dispatch() {
        observer_dispatch();
        inc(&OBSERVER_ENTITY_DISPATCHES);
    }

    #[inline]
    pub(crate) fn observer_component_dispatch() {
        observer_dispatch();
        inc(&OBSERVER_COMPONENT_DISPATCHES);
    }

    #[inline]
    pub(crate) fn observer_entity_component_dispatch() {
        observer_dispatch();
        inc(&OBSERVER_ENTITY_COMPONENT_DISPATCHES);
    }

    #[inline]
    pub(crate) fn observer_deduped() {
        inc(&OBSERVER_DEDUPED);
    }

    #[inline]
    pub(crate) fn relationship_add(collection_scan_len: usize) {
        inc(&RELATIONSHIP_ADDS);
        add(&RELATIONSHIP_COLLECTION_SCAN_LEN, collection_scan_len);
    }

    #[inline]
    pub(crate) fn relationship_remove(collection_scan_len: usize) {
        inc(&RELATIONSHIP_REMOVES);
        add(&RELATIONSHIP_COLLECTION_SCAN_LEN, collection_scan_len);
    }

    #[inline]
    pub(crate) fn change_tick_check_skipped_under_threshold() {
        inc(&CHANGE_TICK_CHECK_SKIPPED_UNDER_THRESHOLD);
    }

    #[inline]
    pub(crate) fn change_tick_check_finished(nanos: usize) {
        inc(&CHANGE_TICK_CHECKS);
        add(&CHANGE_TICK_CHECK_NANOS, nanos);
    }

    #[inline]
    pub(crate) fn change_tick_table_scanned(entity_count: usize, component_count: usize) {
        inc(&CHANGE_TICK_CHECK_TABLES);
        if entity_count == 0 {
            inc(&CHANGE_TICK_CHECK_EMPTY_TABLES);
        } else {
            add(
                &CHANGE_TICK_CHECK_COMPONENT_TICKS,
                entity_count
                    .saturating_mul(component_count)
                    .saturating_mul(2),
            );
        }
    }

    #[inline]
    pub(crate) fn change_tick_sparse_set_scanned(entity_count: usize) {
        inc(&CHANGE_TICK_CHECK_SPARSE_SETS);
        if entity_count == 0 {
            inc(&CHANGE_TICK_CHECK_EMPTY_SPARSE_SETS);
        } else {
            add(
                &CHANGE_TICK_CHECK_COMPONENT_TICKS,
                entity_count.saturating_mul(2),
            );
        }
    }

    pub fn reset() {
        reset_all();
        OBSERVER_TRIGGER_DEPTH.store(0, Ordering::Relaxed);
    }

    pub fn snapshot() -> AuditCounters {
        AuditCounters {
            query_update_archetypes: load(&QUERY_UPDATE_ARCHETYPES),
            query_archetypes_scanned: load(&QUERY_ARCHETYPES_SCANNED),
            query_new_archetype_calls: load(&QUERY_NEW_ARCHETYPE_CALLS),
            query_matched_archetypes: load(&QUERY_MATCHED_ARCHETYPES),
            query_matched_tables: load(&QUERY_MATCHED_TABLES),
            table_allocations: load(&TABLE_ALLOCATIONS),
            table_row_moves: load(&TABLE_ROW_MOVES),
            table_row_swap_removes: load(&TABLE_ROW_SWAP_REMOVES),
            sparse_set_inserts: load(&SPARSE_SET_INSERTS),
            sparse_set_removes: load(&SPARSE_SET_REMOVES),
            sparse_set_gets: load(&SPARSE_SET_GETS),
            command_queue_bytes_pushed: load(&COMMAND_QUEUE_BYTES_PUSHED),
            command_queue_commands_pushed: load(&COMMAND_QUEUE_COMMANDS_PUSHED),
            command_queue_applies: load(&COMMAND_QUEUE_APPLIES),
            command_queue_recursive_applies: load(&COMMAND_QUEUE_RECURSIVE_APPLIES),
            command_queue_append_calls: load(&COMMAND_QUEUE_APPEND_CALLS),
            command_queue_bytes_appended: load(&COMMAND_QUEUE_BYTES_APPENDED),
            command_queue_reallocations: load(&COMMAND_QUEUE_REALLOCATIONS),
            command_queue_world_flushes: load(&COMMAND_QUEUE_WORLD_FLUSHES),
            scheduler_lock_failures: load(&SCHEDULER_LOCK_FAILURES),
            scheduler_ready_scan_passes: load(&SCHEDULER_READY_SCAN_PASSES),
            scheduler_ready_systems_scanned: load(&SCHEDULER_READY_SYSTEMS_SCANNED),
            scheduler_condition_evaluations: load(&SCHEDULER_CONDITION_EVALUATIONS),
            scheduler_tasks_spawned: load(&SCHEDULER_TASKS_SPAWNED),
            scheduler_exclusive_tasks_spawned: load(&SCHEDULER_EXCLUSIVE_TASKS_SPAWNED),
            scheduler_non_send_tasks_spawned: load(&SCHEDULER_NON_SEND_TASKS_SPAWNED),
            scheduler_ready_to_run_nanos: load(&SCHEDULER_READY_TO_RUN_NANOS),
            scheduler_ready_to_run_samples: load(&SCHEDULER_READY_TO_RUN_SAMPLES),
            scheduler_idle_ready_nanos: load(&SCHEDULER_IDLE_READY_NANOS),
            scheduler_lock_hold_nanos: load(&SCHEDULER_LOCK_HOLD_NANOS),
            scheduler_lock_hold_samples: load(&SCHEDULER_LOCK_HOLD_SAMPLES),
            scheduler_apply_deferred_bitset_reuses: load(&SCHEDULER_APPLY_DEFERRED_BITSET_REUSES),
            apply_deferred_calls: load(&APPLY_DEFERRED_CALLS),
            apply_deferred_systems: load(&APPLY_DEFERRED_SYSTEMS),
            apply_deferred_nanos: load(&APPLY_DEFERRED_NANOS),
            observer_triggers: load(&OBSERVER_TRIGGERS),
            observer_no_observers: load(&OBSERVER_NO_OBSERVERS),
            observer_dispatches: load(&OBSERVER_DISPATCHES),
            observer_global_dispatches: load(&OBSERVER_GLOBAL_DISPATCHES),
            observer_entity_dispatches: load(&OBSERVER_ENTITY_DISPATCHES),
            observer_component_dispatches: load(&OBSERVER_COMPONENT_DISPATCHES),
            observer_entity_component_dispatches: load(&OBSERVER_ENTITY_COMPONENT_DISPATCHES),
            observer_deduped: load(&OBSERVER_DEDUPED),
            observer_max_trigger_depth: load(&OBSERVER_MAX_TRIGGER_DEPTH),
            relationship_adds: load(&RELATIONSHIP_ADDS),
            relationship_removes: load(&RELATIONSHIP_REMOVES),
            relationship_collection_scan_len: load(&RELATIONSHIP_COLLECTION_SCAN_LEN),
            change_tick_checks: load(&CHANGE_TICK_CHECKS),
            change_tick_check_skipped_under_threshold: load(
                &CHANGE_TICK_CHECK_SKIPPED_UNDER_THRESHOLD,
            ),
            change_tick_check_nanos: load(&CHANGE_TICK_CHECK_NANOS),
            change_tick_check_tables: load(&CHANGE_TICK_CHECK_TABLES),
            change_tick_check_empty_tables: load(&CHANGE_TICK_CHECK_EMPTY_TABLES),
            change_tick_check_sparse_sets: load(&CHANGE_TICK_CHECK_SPARSE_SETS),
            change_tick_check_empty_sparse_sets: load(&CHANGE_TICK_CHECK_EMPTY_SPARSE_SETS),
            change_tick_check_component_ticks: load(&CHANGE_TICK_CHECK_COMPONENT_TICKS),
        }
    }

    pub fn storage_metrics(world: &crate::world::World) -> StorageMetrics {
        let storages = world.storages();
        StorageMetrics {
            archetype_count: world.archetypes().len(),
            empty_archetype_count: world.archetypes().audit_empty_count(),
            archetype_edge_entries: world.archetypes().audit_edge_entries(),
            archetype_edge_slots: world.archetypes().audit_edge_slots(),
            archetype_edge_capacity: world.archetypes().audit_edge_capacity(),
            table_count: storages.tables.len(),
            empty_table_count: storages.tables.audit_empty_count(),
            table_entity_count: storages.tables.audit_entity_count(),
            table_entity_capacity: storages.tables.audit_entity_capacity(),
            table_column_count: storages.tables.audit_column_count(),
            sparse_set_count: storages.sparse_sets.len(),
            sparse_set_entity_count: storages.sparse_sets.audit_entity_count(),
            sparse_set_entity_capacity: storages.sparse_sets.audit_entity_capacity(),
            sparse_set_sparse_slots: storages.sparse_sets.audit_sparse_slot_count(),
            sparse_set_sparse_capacity: storages.sparse_sets.audit_sparse_slot_capacity(),
        }
    }

    pub fn memory_metrics(world: &crate::world::World) -> MemoryMetrics {
        let storages = world.storages();
        let archetypes = world.archetypes();
        let command_queue = &world.command_queue;
        let (command_queue_len_bytes, command_queue_capacity_bytes) = unsafe {
            let bytes = command_queue.bytes.as_ref();
            (bytes.len(), bytes.capacity())
        };
        let (command_queue_panic_recovery_len_bytes, command_queue_panic_recovery_capacity_bytes) = unsafe {
            let bytes = command_queue.panic_recovery.as_ref();
            (bytes.len(), bytes.capacity())
        };
        let entity_meta_retained_bytes = world.entities().audit_meta_retained_bytes();
        let archetype_retained_bytes = archetypes.audit_retained_bytes();
        let table_entity_retained_bytes = storages.tables.audit_entity_retained_bytes();
        let table_column_retained_bytes = storages.tables.audit_column_retained_bytes();
        let sparse_set_retained_bytes = storages.sparse_sets.audit_retained_bytes();
        let command_queue_retained_bytes = command_queue_capacity_bytes
            .saturating_add(command_queue_panic_recovery_capacity_bytes);
        let observer_retained_bytes = world.observers().audit_retained_bytes();
        let estimated_retained_bytes = entity_meta_retained_bytes
            .saturating_add(archetype_retained_bytes)
            .saturating_add(table_entity_retained_bytes)
            .saturating_add(table_column_retained_bytes)
            .saturating_add(sparse_set_retained_bytes)
            .saturating_add(command_queue_retained_bytes)
            .saturating_add(observer_retained_bytes);

        MemoryMetrics {
            estimated_retained_bytes,
            entity_meta_len: world.entities().len() as usize,
            entity_meta_capacity: world.entities().audit_meta_capacity(),
            entity_meta_retained_bytes,
            archetype_count: archetypes.len(),
            empty_archetype_count: archetypes.audit_empty_count(),
            archetype_edge_entries: archetypes.audit_edge_entries(),
            archetype_edge_slots: archetypes.audit_edge_slots(),
            archetype_edge_capacity: archetypes.audit_edge_capacity(),
            archetype_retained_bytes,
            table_count: storages.tables.len(),
            empty_table_count: storages.tables.audit_empty_count(),
            table_entity_count: storages.tables.audit_entity_count(),
            table_entity_capacity: storages.tables.audit_entity_capacity(),
            table_entity_retained_bytes,
            table_column_count: storages.tables.audit_column_count(),
            table_column_retained_bytes,
            sparse_set_count: storages.sparse_sets.len(),
            sparse_set_entity_count: storages.sparse_sets.audit_entity_count(),
            sparse_set_entity_capacity: storages.sparse_sets.audit_entity_capacity(),
            sparse_set_sparse_slots: storages.sparse_sets.audit_sparse_slot_count(),
            sparse_set_sparse_capacity: storages.sparse_sets.audit_sparse_slot_capacity(),
            sparse_set_retained_bytes,
            command_queue_len_bytes,
            command_queue_capacity_bytes,
            command_queue_panic_recovery_len_bytes,
            command_queue_panic_recovery_capacity_bytes,
            observer_event_cache_entries: world.observers().audit_event_cache_entries(),
            observer_event_cache_capacity: world.observers().audit_event_cache_capacity(),
            observer_runner_entries: world.observers().audit_runner_entries(),
            observer_runner_capacity: world.observers().audit_runner_capacity(),
            observer_retained_bytes,
        }
    }

    pub fn force_check_change_ticks(
        world: &mut crate::world::World,
    ) -> Option<crate::change_detection::CheckChangeTicks> {
        world.audit_force_check_change_ticks()
    }
}

#[cfg(not(feature = "bevy_ecs_audit"))]
mod imp {
    pub struct ObserverTriggerGuard;

    #[inline]
    pub(crate) fn query_update_archetypes(_: usize) {}
    #[inline]
    pub(crate) fn query_new_archetype(_: bool, _: bool) {}
    #[inline]
    pub(crate) fn table_allocate() {}
    #[inline]
    pub(crate) fn table_move_row() {}
    #[inline]
    pub(crate) fn table_swap_remove() {}
    #[inline]
    pub(crate) fn sparse_set_insert() {}
    #[inline]
    pub(crate) fn sparse_set_remove() {}
    #[inline]
    pub(crate) fn sparse_set_get() {}
    #[inline]
    pub(crate) fn command_queue_push(_: usize) {}
    #[inline]
    pub(crate) fn command_queue_apply(_: usize, _: usize) {}
    #[inline]
    pub(crate) fn command_queue_append(_: usize) {}
    #[inline]
    pub(crate) fn command_queue_reallocation() {}
    #[inline]
    pub(crate) fn command_queue_world_flush() {}
    #[inline]
    pub(crate) fn scheduler_lock_failed() {}
    #[inline]
    pub(crate) fn scheduler_ready_scan(_: usize) {}
    #[inline]
    pub(crate) fn scheduler_condition_evaluations(_: usize) {}
    #[inline]
    pub(crate) fn scheduler_task_spawned(_: bool, _: bool) {}
    #[inline]
    #[expect(
        dead_code,
        reason = "audit-only timing hooks are compiled out when bevy_ecs_audit is disabled"
    )]
    pub(crate) fn scheduler_ready_to_run_delay(_: usize) {}
    #[inline]
    #[expect(
        dead_code,
        reason = "audit-only timing hooks are compiled out when bevy_ecs_audit is disabled"
    )]
    pub(crate) fn scheduler_idle_ready_wait(_: usize) {}
    #[inline]
    #[expect(
        dead_code,
        reason = "audit-only timing hooks are compiled out when bevy_ecs_audit is disabled"
    )]
    pub(crate) fn scheduler_lock_held(_: usize) {}
    #[inline]
    pub(crate) fn scheduler_apply_deferred_bitset_reuse() {}
    #[inline]
    #[expect(
        dead_code,
        reason = "audit-only timing hooks are compiled out when bevy_ecs_audit is disabled"
    )]
    pub(crate) fn apply_deferred_finished(_: usize, _: usize) {}
    #[inline]
    pub(crate) fn observer_trigger_scope() -> ObserverTriggerGuard {
        ObserverTriggerGuard
    }
    #[inline]
    pub(crate) fn observer_no_observers() {}
    #[inline]
    pub(crate) fn observer_dispatch() {}
    #[inline]
    pub(crate) fn observer_global_dispatch() {
        observer_dispatch();
    }
    #[inline]
    pub(crate) fn observer_entity_dispatch() {
        observer_dispatch();
    }
    #[inline]
    pub(crate) fn observer_component_dispatch() {
        observer_dispatch();
    }
    #[inline]
    pub(crate) fn observer_entity_component_dispatch() {
        observer_dispatch();
    }
    #[inline]
    pub(crate) fn observer_deduped() {}
    #[inline]
    pub(crate) fn relationship_add(_: usize) {}
    #[inline]
    pub(crate) fn relationship_remove(_: usize) {}
    #[inline]
    pub(crate) fn change_tick_check_skipped_under_threshold() {}
    #[inline]
    #[expect(
        dead_code,
        reason = "audit-only timing hooks are compiled out when bevy_ecs_audit is disabled"
    )]
    pub(crate) fn change_tick_check_finished(_: usize) {}
    #[inline]
    pub(crate) fn change_tick_table_scanned(_: usize, _: usize) {}
    #[inline]
    pub(crate) fn change_tick_sparse_set_scanned(_: usize) {}
}

pub(crate) use imp::*;

#[cfg(feature = "bevy_ecs_audit")]
pub use imp::{
    force_check_change_ticks, memory_metrics, reset, snapshot, storage_metrics, AuditCounters,
    MemoryMetrics, StorageMetrics,
};

#[cfg(all(test, feature = "bevy_ecs_audit"))]
mod tests {
    use crate::{
        component::Component,
        entity::{Entity, EntityIndex},
        event::Event,
        hierarchy::ChildOf,
        observer::On,
        prelude::Resource,
        schedule::{ApplyDeferred, IntoScheduleConfigs, MultiThreadedExecutor, Schedule},
        system::{Commands, ResMut},
        world::{CommandQueue, World},
    };
    use std::eprintln;

    #[derive(Component)]
    struct TableComponent;

    #[derive(Component)]
    #[component(storage = "SparseSet")]
    struct SparseComponent;

    #[derive(Component)]
    struct EmptyTableComponent;

    #[derive(Event)]
    struct AuditEvent;

    #[derive(Event)]
    struct AuditNoObserverEvent;

    #[derive(Resource, Default)]
    struct AuditScheduleLog(usize);

    #[derive(Component)]
    struct MemoryDense;

    #[derive(Component)]
    #[component(storage = "SparseSet")]
    struct MemorySparse;

    #[repr(align(256))]
    #[derive(Component)]
    struct MemoryAligned {
        _bytes: [u8; 64],
    }

    #[derive(Event)]
    struct MemoryEvent;

    macro_rules! marker_components {
        ($($name:ident),* $(,)?) => {
            $(
                #[derive(Component)]
                struct $name;
            )*
        };
    }

    marker_components!(
        M0, M1, M2, M3, M4, M5, M6, M7, M8, M9, M10, M11, M12, M13, M14, M15, M16, M17, M18, M19,
        M20, M21, M22, M23, M24, M25, M26, M27, M28, M29, M30, M31,
    );

    fn audit_schedule_command(mut commands: Commands) {
        commands.queue(|world: &mut World| {
            world.resource_mut::<AuditScheduleLog>().0 += 1;
        });
    }

    fn audit_schedule_system(mut log: ResMut<AuditScheduleLog>) {
        log.0 += 1;
    }

    fn audit_noop_system() {}

    macro_rules! insert_markers {
        ($entity:ident, $mask:ident, $($bit:literal => $component:ident),* $(,)?) => {
            $(
                if $mask & (1usize << $bit) != 0 {
                    $entity.insert($component);
                }
            )*
        };
    }

    fn insert_archetype_churn_components(world: &mut World, entity: Entity, mask: usize) {
        let mut entity = world.entity_mut(entity);
        insert_markers!(
            entity, mask,
            0 => M0, 1 => M1, 2 => M2, 3 => M3, 4 => M4, 5 => M5, 6 => M6, 7 => M7,
            8 => M8, 9 => M9, 10 => M10, 11 => M11, 12 => M12, 13 => M13, 14 => M14,
            15 => M15, 16 => M16,
        );
    }

    fn insert_wide_components(world: &mut World, entity: Entity) {
        let mut entity = world.entity_mut(entity);
        entity
            .insert(M0)
            .insert(M1)
            .insert(M2)
            .insert(M3)
            .insert(M4)
            .insert(M5)
            .insert(M6)
            .insert(M7)
            .insert(M8)
            .insert(M9)
            .insert(M10)
            .insert(M11)
            .insert(M12)
            .insert(M13)
            .insert(M14)
            .insert(M15)
            .insert(M16)
            .insert(M17)
            .insert(M18)
            .insert(M19)
            .insert(M20)
            .insert(M21)
            .insert(M22)
            .insert(M23)
            .insert(M24)
            .insert(M25)
            .insert(M26)
            .insert(M27)
            .insert(M28)
            .insert(M29)
            .insert(M30)
            .insert(M31);
    }

    fn metric_report(label: &str, metrics: super::MemoryMetrics) {
        eprintln!(
            "memory_audit label={label} estimated_retained_bytes={} entity_meta_bytes={} archetype_bytes={} table_entity_bytes={} table_column_bytes={} sparse_bytes={} command_queue_capacity_bytes={} observer_bytes={} archetypes={} empty_archetypes={} tables={} empty_tables={} table_entity_capacity={} sparse_slots={} sparse_slot_capacity={} observer_cache_capacity={} observer_runner_capacity={}",
            metrics.estimated_retained_bytes,
            metrics.entity_meta_retained_bytes,
            metrics.archetype_retained_bytes,
            metrics.table_entity_retained_bytes,
            metrics.table_column_retained_bytes,
            metrics.sparse_set_retained_bytes,
            metrics.command_queue_capacity_bytes
                + metrics.command_queue_panic_recovery_capacity_bytes,
            metrics.observer_retained_bytes,
            metrics.archetype_count,
            metrics.empty_archetype_count,
            metrics.table_count,
            metrics.empty_table_count,
            metrics.table_entity_capacity,
            metrics.sparse_set_sparse_slots,
            metrics.sparse_set_sparse_capacity,
            metrics.observer_event_cache_capacity,
            metrics.observer_runner_capacity,
        );
    }

    fn spawn_despawn_entities(entity_count: usize) -> super::MemoryMetrics {
        let mut world = World::new();
        world.spawn_batch((0..entity_count).map(|_| MemoryDense));
        world.clear_entities();
        super::memory_metrics(&world)
    }

    fn archetype_churn(archetype_count: usize) -> super::MemoryMetrics {
        let mut world = World::new();
        for mask in 0..archetype_count {
            let entity = world.spawn_empty().id();
            insert_archetype_churn_components(&mut world, entity, mask);
        }
        world.clear_entities();
        super::memory_metrics(&world)
    }

    fn sparse_high_index(
        high_start: u32,
        sparse_entities: u32,
        stride: u32,
    ) -> super::MemoryMetrics {
        let mut world = World::new();
        for offset in 0..sparse_entities {
            let entity = Entity::from_index(
                EntityIndex::from_raw_u32(high_start + offset.saturating_mul(stride)).unwrap(),
            );
            world.spawn_empty_at(entity).unwrap();
            world.entity_mut(entity).insert(MemorySparse);
        }
        world.clear_entities();
        super::memory_metrics(&world)
    }

    fn command_storm(command_count: usize) -> super::MemoryMetrics {
        let mut world = World::new();
        for _ in 0..command_count {
            world.commands().queue(|world: &mut World| {
                world.spawn(MemoryDense);
            });
        }
        world.flush();
        world.clear_entities();
        super::memory_metrics(&world)
    }

    fn observer_storm(observer_count: usize) -> super::MemoryMetrics {
        let mut world = World::new();
        let observers = (0..observer_count)
            .map(|_| world.add_observer(|_: On<MemoryEvent>| {}).id())
            .collect::<alloc::vec::Vec<_>>();
        for observer in observers {
            world.despawn(observer);
        }
        super::memory_metrics(&world)
    }

    fn relationship_storm(child_count: usize) -> super::MemoryMetrics {
        let mut world = World::new();
        let parent = world.spawn_empty().id();
        let children = (0..child_count)
            .map(|_| world.spawn_empty().id())
            .collect::<alloc::vec::Vec<_>>();
        world.entity_mut(parent).add_children(&children);
        assert_eq!(world.query::<&ChildOf>().iter(&world).count(), child_count);
        world.entity_mut(parent).detach_children(&children);
        world.clear_entities();
        super::memory_metrics(&world)
    }

    fn schedule_rebuild_storm(
        rebuild_count: usize,
        systems_per_schedule: usize,
    ) -> super::MemoryMetrics {
        let mut world = World::new();
        for _ in 0..rebuild_count {
            let mut schedule = Schedule::default();
            for _ in 0..systems_per_schedule {
                schedule.add_systems(audit_noop_system);
            }
            schedule.run(&mut world);
        }
        super::memory_metrics(&world)
    }

    fn aligned_table_churn(entity_count: usize) -> super::MemoryMetrics {
        let mut world = World::new();
        world.spawn_batch((0..entity_count).map(|_| MemoryAligned { _bytes: [0; 64] }));
        world.clear_entities();
        super::memory_metrics(&world)
    }

    fn wide_table_removal(entity_count: usize) -> super::MemoryMetrics {
        let mut world = World::new();
        let entities = (0..entity_count)
            .map(|_| {
                let entity = world.spawn_empty().id();
                insert_wide_components(&mut world, entity);
                entity
            })
            .collect::<alloc::vec::Vec<_>>();
        for entity in entities {
            world.entity_mut(entity).remove::<M15>();
        }
        world.clear_entities();
        super::memory_metrics(&world)
    }

    #[test]
    fn audit_counters_record_representative_ecs_paths() {
        super::reset();

        let mut world = World::new();
        let entity = world.spawn((TableComponent, SparseComponent)).id();
        let mut query = world.query::<(&TableComponent, Option<&SparseComponent>)>();
        assert_eq!(query.iter(&world).count(), 1);

        let empty_table_entity = world.spawn(EmptyTableComponent).id();
        world.despawn(empty_table_entity);

        world.entity_mut(entity).remove::<SparseComponent>();

        let mut queue = CommandQueue::default();
        queue.push(|world: &mut World| {
            world.spawn(TableComponent);
        });
        let mut appended_queue = CommandQueue::default();
        appended_queue.push(|world: &mut World| {
            world.spawn(TableComponent);
        });
        queue.append(&mut appended_queue);
        queue.apply(&mut world);
        world.commands().queue(|world: &mut World| {
            world.spawn(TableComponent);
        });
        world.flush();

        world.add_observer(|_: On<AuditEvent>| {});
        world.trigger(AuditEvent);
        world.trigger(AuditNoObserverEvent);

        world.init_resource::<AuditScheduleLog>();
        let mut schedule = Schedule::default();
        schedule.set_executor(MultiThreadedExecutor::new());
        schedule
            .add_systems((audit_schedule_command, ApplyDeferred, audit_schedule_system).chain());
        schedule.run(&mut world);

        assert!(world.check_change_ticks().is_none());
        assert!(super::force_check_change_ticks(&mut world).is_some());

        let counters = super::snapshot();
        assert!(counters.query_update_archetypes > 0);
        assert!(counters.query_new_archetype_calls > 0);
        assert!(counters.table_allocations > 0);
        assert!(counters.sparse_set_gets > 0);
        assert!(counters.sparse_set_removes > 0);
        assert!(counters.command_queue_commands_pushed >= 1);
        assert!(counters.command_queue_applies > 0);
        assert!(counters.command_queue_append_calls > 0);
        assert!(counters.command_queue_bytes_appended > 0);
        assert!(counters.command_queue_reallocations > 0);
        assert!(counters.command_queue_world_flushes > 0);
        assert!(counters.scheduler_ready_scan_passes > 0);
        assert!(counters.scheduler_tasks_spawned > 0);
        assert!(counters.scheduler_exclusive_tasks_spawned > 0);
        assert!(counters.scheduler_ready_to_run_samples > 0);
        assert!(counters.scheduler_lock_hold_samples > 0);
        assert!(counters.scheduler_apply_deferred_bitset_reuses > 0);
        assert!(counters.apply_deferred_calls > 0);
        assert!(counters.observer_dispatches >= 1);
        assert!(counters.observer_global_dispatches >= 1);
        assert!(counters.observer_no_observers >= 1);
        assert!(counters.observer_max_trigger_depth > 0);
        assert!(counters.change_tick_check_skipped_under_threshold > 0);
        assert!(counters.change_tick_checks > 0);
        assert!(counters.change_tick_check_tables > 0);
        assert!(counters.change_tick_check_empty_tables > 0);
        assert!(counters.change_tick_check_sparse_sets > 0);
        assert!(counters.change_tick_check_empty_sparse_sets > 0);
        assert!(counters.change_tick_check_component_ticks > 0);
    }

    #[test]
    fn memory_metrics_record_representative_retained_capacity() {
        let entity_metrics = spawn_despawn_entities(1_024);
        assert_eq!(entity_metrics.table_entity_count, 0);
        assert!(entity_metrics.entity_meta_capacity >= 1_024);
        assert!(entity_metrics.table_column_retained_bytes > 0);

        let sparse_metrics = sparse_high_index(20_000, 4, 1_000);
        assert_eq!(sparse_metrics.sparse_set_entity_count, 0);
        assert!(
            sparse_metrics.sparse_set_sparse_capacity >= 23_001,
            "sparse capacity should retain the high entity-index backing array"
        );

        let command_metrics = command_storm(512);
        assert!(command_metrics.command_queue_capacity_bytes > 0);
        assert!(command_metrics.estimated_retained_bytes > 0);
    }

    #[test]
    #[ignore = "heavy memory audit scenario; run explicitly with --ignored --nocapture"]
    fn memory_audit_heavy_churn_scenarios() {
        macro_rules! run_scenario {
            ($label:literal, $metrics:expr) => {{
                let label = $label;
                let metrics = $metrics;
                metric_report(label, metrics);
                assert!(
                    metrics.estimated_retained_bytes > 0 || label == "schedule_rebuild_storm",
                    "retained bytes should be visible for {label}"
                );
            }};
        }

        run_scenario!("spawn_despawn_1m", spawn_despawn_entities(1_000_000));
        run_scenario!("archetype_churn_100k", archetype_churn(100_000));
        run_scenario!(
            "sparse_high_index_low_density",
            sparse_high_index(10_000_000, 100, 10_000)
        );
        run_scenario!("command_storm_repeated", command_storm(100_000));
        run_scenario!("observer_register_unregister_storm", observer_storm(10_000));
        run_scenario!("relationship_add_remove_storm", relationship_storm(100_000));
        run_scenario!("schedule_rebuild_storm", schedule_rebuild_storm(1_000, 64));
        run_scenario!("large_alignment_table_churn", aligned_table_churn(100_000));
        run_scenario!("wide_table_component_removal", wide_table_removal(10_000));
    }
}
