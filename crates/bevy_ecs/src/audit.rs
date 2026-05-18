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
        SCHEDULER_LOCK_FAILURES,
        SCHEDULER_READY_SCAN_PASSES,
        SCHEDULER_READY_SYSTEMS_SCANNED,
        SCHEDULER_CONDITION_EVALUATIONS,
        APPLY_DEFERRED_CALLS,
        APPLY_DEFERRED_SYSTEMS,
        APPLY_DEFERRED_NANOS,
        OBSERVER_TRIGGERS,
        OBSERVER_DISPATCHES,
        OBSERVER_MAX_TRIGGER_DEPTH,
        RELATIONSHIP_ADDS,
        RELATIONSHIP_REMOVES,
        RELATIONSHIP_COLLECTION_SCAN_LEN,
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
        pub scheduler_lock_failures: usize,
        pub scheduler_ready_scan_passes: usize,
        pub scheduler_ready_systems_scanned: usize,
        pub scheduler_condition_evaluations: usize,
        pub apply_deferred_calls: usize,
        pub apply_deferred_systems: usize,
        pub apply_deferred_nanos: usize,
        pub observer_triggers: usize,
        pub observer_dispatches: usize,
        pub observer_max_trigger_depth: usize,
        pub relationship_adds: usize,
        pub relationship_removes: usize,
        pub relationship_collection_scan_len: usize,
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
            scheduler_lock_failures: load(&SCHEDULER_LOCK_FAILURES),
            scheduler_ready_scan_passes: load(&SCHEDULER_READY_SCAN_PASSES),
            scheduler_ready_systems_scanned: load(&SCHEDULER_READY_SYSTEMS_SCANNED),
            scheduler_condition_evaluations: load(&SCHEDULER_CONDITION_EVALUATIONS),
            apply_deferred_calls: load(&APPLY_DEFERRED_CALLS),
            apply_deferred_systems: load(&APPLY_DEFERRED_SYSTEMS),
            apply_deferred_nanos: load(&APPLY_DEFERRED_NANOS),
            observer_triggers: load(&OBSERVER_TRIGGERS),
            observer_dispatches: load(&OBSERVER_DISPATCHES),
            observer_max_trigger_depth: load(&OBSERVER_MAX_TRIGGER_DEPTH),
            relationship_adds: load(&RELATIONSHIP_ADDS),
            relationship_removes: load(&RELATIONSHIP_REMOVES),
            relationship_collection_scan_len: load(&RELATIONSHIP_COLLECTION_SCAN_LEN),
        }
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
    pub(crate) fn scheduler_lock_failed() {}
    #[inline]
    pub(crate) fn scheduler_ready_scan(_: usize) {}
    #[inline]
    pub(crate) fn scheduler_condition_evaluations(_: usize) {}
    #[inline]
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
}

pub(crate) use imp::*;

#[cfg(feature = "bevy_ecs_audit")]
pub use imp::{reset, snapshot, AuditCounters};

#[cfg(all(test, feature = "bevy_ecs_audit"))]
mod tests {
    use crate::{
        component::Component,
        event::Event,
        observer::On,
        world::{CommandQueue, World},
    };

    #[derive(Component)]
    struct TableComponent;

    #[derive(Component)]
    #[component(storage = "SparseSet")]
    struct SparseComponent;

    #[derive(Event)]
    struct AuditEvent;

    #[test]
    fn audit_counters_record_representative_ecs_paths() {
        super::reset();

        let mut world = World::new();
        let entity = world.spawn((TableComponent, SparseComponent)).id();
        let mut query = world.query::<(&TableComponent, Option<&SparseComponent>)>();
        assert_eq!(query.iter(&world).count(), 1);

        world.entity_mut(entity).remove::<SparseComponent>();

        let mut queue = CommandQueue::default();
        queue.push(|world: &mut World| {
            world.spawn(TableComponent);
        });
        queue.apply(&mut world);

        world.add_observer(|_: On<AuditEvent>| {});
        world.trigger(AuditEvent);

        let counters = super::snapshot();
        assert!(counters.query_update_archetypes > 0);
        assert!(counters.query_new_archetype_calls > 0);
        assert!(counters.table_allocations > 0);
        assert!(counters.sparse_set_gets > 0);
        assert!(counters.sparse_set_removes > 0);
        assert!(counters.command_queue_commands_pushed >= 1);
        assert!(counters.command_queue_applies > 0);
        assert!(counters.observer_dispatches >= 1);
        assert!(counters.observer_max_trigger_depth > 0);
    }
}
