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
        OBSERVER_DISPATCHES,
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
        pub observer_dispatches: usize,
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
        pub table_count: usize,
        pub empty_table_count: usize,
        pub table_entity_count: usize,
        pub table_entity_capacity: usize,
        pub table_column_count: usize,
        pub sparse_set_count: usize,
        pub sparse_set_entity_count: usize,
        pub sparse_set_entity_capacity: usize,
        pub sparse_set_sparse_slots: usize,
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
    pub(crate) fn observer_dispatch() {
        inc(&OBSERVER_DISPATCHES);
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
            observer_dispatches: load(&OBSERVER_DISPATCHES),
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
            table_count: storages.tables.len(),
            empty_table_count: storages.tables.audit_empty_count(),
            table_entity_count: storages.tables.audit_entity_count(),
            table_entity_capacity: storages.tables.audit_entity_capacity(),
            table_column_count: storages.tables.audit_column_count(),
            sparse_set_count: storages.sparse_sets.len(),
            sparse_set_entity_count: storages.sparse_sets.audit_entity_count(),
            sparse_set_entity_capacity: storages.sparse_sets.audit_entity_capacity(),
            sparse_set_sparse_slots: storages.sparse_sets.audit_sparse_slot_count(),
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
    pub(crate) fn observer_dispatch() {}
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
    force_check_change_ticks, reset, snapshot, storage_metrics, AuditCounters, StorageMetrics,
};

#[cfg(all(test, feature = "bevy_ecs_audit"))]
mod tests {
    use crate::{
        component::Component,
        event::Event,
        observer::On,
        prelude::Resource,
        schedule::{ApplyDeferred, IntoScheduleConfigs, MultiThreadedExecutor, Schedule},
        system::{Commands, ResMut},
        world::{CommandQueue, World},
    };

    #[derive(Component)]
    struct TableComponent;

    #[derive(Component)]
    #[component(storage = "SparseSet")]
    struct SparseComponent;

    #[derive(Component)]
    struct EmptyTableComponent;

    #[derive(Event)]
    struct AuditEvent;

    #[derive(Resource, Default)]
    struct AuditScheduleLog(usize);

    fn audit_schedule_command(mut commands: Commands) {
        commands.queue(|world: &mut World| {
            world.resource_mut::<AuditScheduleLog>().0 += 1;
        });
    }

    fn audit_schedule_system(mut log: ResMut<AuditScheduleLog>) {
        log.0 += 1;
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
        assert!(counters.observer_max_trigger_depth > 0);
        assert!(counters.change_tick_check_skipped_under_threshold > 0);
        assert!(counters.change_tick_checks > 0);
        assert!(counters.change_tick_check_tables > 0);
        assert!(counters.change_tick_check_empty_tables > 0);
        assert!(counters.change_tick_check_sparse_sets > 0);
        assert!(counters.change_tick_check_empty_sparse_sets > 0);
        assert!(counters.change_tick_check_component_ticks > 0);
    }
}
