use crate::{
    component::Component,
    entity::{Entity, EntityIndex},
    storage::TableId,
    system::Commands,
    world::{CommandQueue, World},
};
use alloc::{sync::Arc, vec::Vec};
use core::sync::atomic::{AtomicUsize, Ordering};
use std::panic::{catch_unwind, AssertUnwindSafe};
#[cfg(feature = "bevy_ecs_audit")]
use std::println;

#[derive(Component)]
struct DenseA(u64);

#[derive(Component)]
struct DenseB(u64);

#[derive(Component)]
#[component(storage = "SparseSet")]
struct SparseA(u64);

#[derive(Component)]
struct Zst;

#[repr(align(128))]
#[derive(Component)]
struct Aligned(u64);

#[derive(Component, Default)]
struct RequiredLeaf;

#[derive(Component)]
#[require(RequiredLeaf)]
struct RequiredRoot;

#[cfg(feature = "bevy_ecs_audit")]
macro_rules! churn_components {
    ($($name:ident),* $(,)?) => {
        $(
            #[derive(Component)]
            struct $name;
        )*
    };
}

#[cfg(feature = "bevy_ecs_audit")]
churn_components!(
    Churn0, Churn1, Churn2, Churn3, Churn4, Churn5, Churn6, Churn7, Churn8, Churn9, Churn10,
    Churn11,
);

#[derive(Component)]
struct DropAudit {
    drops: Arc<AtomicUsize>,
}

impl Drop for DropAudit {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

#[derive(Component)]
struct PanicOnDrop {
    drops: Arc<AtomicUsize>,
}

impl Drop for PanicOnDrop {
    fn drop(&mut self) {
        if self.drops.fetch_add(1, Ordering::SeqCst) == 0 {
            panic!("storage audit drop panic");
        }
    }
}

#[derive(Default)]
struct SlotState {
    dense_a: Option<u64>,
    dense_b: Option<u64>,
    sparse_a: Option<u64>,
    zst: bool,
    aligned: Option<u64>,
    required_root: bool,
    required_leaf: bool,
    drop_audit: bool,
}

struct Slot {
    entity: Entity,
    live: bool,
    state: SlotState,
}

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }

    fn range(&mut self, end: usize) -> usize {
        (self.next() as usize) % end
    }
}

fn pick_live_slot(rng: &mut Lcg, slots: &[Slot]) -> Option<usize> {
    let live = slots.iter().filter(|slot| slot.live).count();
    if live == 0 {
        return None;
    }
    let target = rng.range(live);
    slots
        .iter()
        .enumerate()
        .filter(|(_, slot)| slot.live)
        .nth(target)
        .map(|(index, _)| index)
}

fn spawn_mode(world: &mut World, slots: &mut Vec<Slot>, mode: usize, value: u64) {
    let (entity, state) = match mode % 5 {
        0 => (
            world.spawn((DenseA(value), Zst)).id(),
            SlotState {
                dense_a: Some(value),
                zst: true,
                ..Default::default()
            },
        ),
        1 => (
            world.spawn((DenseA(value), DenseB(value + 1))).id(),
            SlotState {
                dense_a: Some(value),
                dense_b: Some(value + 1),
                ..Default::default()
            },
        ),
        2 => (
            world.spawn((DenseA(value), SparseA(value + 2))).id(),
            SlotState {
                dense_a: Some(value),
                sparse_a: Some(value + 2),
                ..Default::default()
            },
        ),
        3 => (
            world.spawn((Aligned(value), SparseA(value + 3))).id(),
            SlotState {
                sparse_a: Some(value + 3),
                aligned: Some(value),
                ..Default::default()
            },
        ),
        _ => (
            world.spawn(RequiredRoot).id(),
            SlotState {
                required_root: true,
                required_leaf: true,
                ..Default::default()
            },
        ),
    };
    slots.push(Slot {
        entity,
        live: true,
        state,
    });
}

#[cfg(feature = "bevy_ecs_audit")]
fn insert_churn_components(world: &mut World, entity: Entity, bits: usize) {
    let mut entity = world.entity_mut(entity);
    if bits & 1 != 0 {
        entity.insert(Churn0);
    }
    if bits & 2 != 0 {
        entity.insert(Churn1);
    }
    if bits & 4 != 0 {
        entity.insert(Churn2);
    }
    if bits & 8 != 0 {
        entity.insert(Churn3);
    }
    if bits & 16 != 0 {
        entity.insert(Churn4);
    }
    if bits & 32 != 0 {
        entity.insert(Churn5);
    }
    if bits & 64 != 0 {
        entity.insert(Churn6);
    }
    if bits & 128 != 0 {
        entity.insert(Churn7);
    }
    if bits & 256 != 0 {
        entity.insert(Churn8);
    }
    if bits & 512 != 0 {
        entity.insert(Churn9);
    }
    if bits & 1024 != 0 {
        entity.insert(Churn10);
    }
    if bits & 2048 != 0 {
        entity.insert(Churn11);
    }
}

fn assert_storage_matches_model(
    world: &World,
    slots: &[Slot],
    constructed_drop_components: usize,
    drops: &AtomicUsize,
) {
    for slot in slots {
        if !slot.live {
            continue;
        }
        assert!(world.entities().contains_spawned(slot.entity));

        assert_eq!(
            world
                .get::<DenseA>(slot.entity)
                .map(|component| component.0),
            slot.state.dense_a
        );
        assert_eq!(
            world
                .get::<DenseB>(slot.entity)
                .map(|component| component.0),
            slot.state.dense_b
        );
        assert_eq!(
            world
                .get::<SparseA>(slot.entity)
                .map(|component| component.0),
            slot.state.sparse_a
        );
        assert_eq!(world.get::<Zst>(slot.entity).is_some(), slot.state.zst);
        assert_eq!(
            world
                .get::<Aligned>(slot.entity)
                .map(|component| component.0),
            slot.state.aligned
        );
        assert_eq!(
            world.get::<RequiredRoot>(slot.entity).is_some(),
            slot.state.required_root
        );
        assert_eq!(
            world.get::<RequiredLeaf>(slot.entity).is_some(),
            slot.state.required_leaf
        );
        assert_eq!(
            world.get::<DropAudit>(slot.entity).is_some(),
            slot.state.drop_audit
        );
    }

    let live_drop_components = slots
        .iter()
        .filter(|slot| slot.live && slot.state.drop_audit)
        .count();
    assert_eq!(
        drops.load(Ordering::SeqCst),
        constructed_drop_components - live_drop_components
    );

    for archetype in world.archetypes().iter() {
        for (entity, location) in archetype.entities_with_location() {
            assert_eq!(world.entities().get_spawned(entity).unwrap(), location);
            assert_eq!(location.table_id, archetype.table_id());
            assert_eq!(
                archetype.entity_table_row(location.archetype_row),
                location.table_row
            );
            assert_eq!(
                world.storages.tables[location.table_id].entities()[location.table_row.index()],
                entity
            );
        }
    }

    for (table_index, table) in world.storages.tables.iter().enumerate() {
        let table_id = TableId::from_usize(table_index);
        for (row, entity) in table.entities().iter().enumerate() {
            let location = world.entities().get_spawned(*entity).unwrap();
            assert_eq!(location.table_id, table_id);
            assert_eq!(location.table_row.index(), row);
        }
    }

    world
        .storages
        .sparse_sets
        .audit_assert_dense_sparse_mappings();
    let _storage_pressure = (
        world.archetypes().audit_empty_count(),
        world.archetypes().audit_edge_entries(),
        world.archetypes().audit_edge_slots(),
        world.storages.tables.audit_empty_count(),
        world.storages.tables.audit_entity_count(),
        world.storages.tables.audit_entity_capacity(),
        world.storages.tables.audit_column_count(),
        world.storages.sparse_sets.audit_entity_capacity(),
    );
}

#[cfg(feature = "bevy_ecs_audit")]
#[test]
fn storage_metrics_report_archetype_and_sparse_pressure() {
    for churn_count in [1_000, 10_000] {
        let mut world = World::new();
        for bits in 1..=churn_count {
            let entity = world.spawn_empty().id();
            insert_churn_components(&mut world, entity, bits);
        }
        world.clear_entities();

        let metrics = crate::audit::storage_metrics(&world);
        println!(
            "storage_metrics churn_count={churn_count} archetypes={} empty_archetypes={} edge_entries={} edge_slots={} tables={} empty_tables={} table_entity_capacity={} table_columns={}",
            metrics.archetype_count,
            metrics.empty_archetype_count,
            metrics.archetype_edge_entries,
            metrics.archetype_edge_slots,
            metrics.table_count,
            metrics.empty_table_count,
            metrics.table_entity_capacity,
            metrics.table_column_count,
        );
        assert!(metrics.archetype_count > 1);
        assert_eq!(metrics.archetype_count, metrics.empty_archetype_count);
        assert!(metrics.table_count > 1);
    }

    let mut world = World::new();
    let high = Entity::from_index(EntityIndex::from_raw_u32(1_000_000).unwrap());
    world.spawn_empty_at(high).unwrap().insert(SparseA(7));
    let metrics = crate::audit::storage_metrics(&world);
    println!(
        "storage_metrics high_sparse_index=1000000 sparse_sets={} sparse_entities={} sparse_capacity={} sparse_slots={}",
        metrics.sparse_set_count,
        metrics.sparse_set_entity_count,
        metrics.sparse_set_entity_capacity,
        metrics.sparse_set_sparse_slots,
    );
    assert!(metrics.sparse_set_sparse_slots > 1_000_000);
}

#[test]
fn randomized_storage_operations_preserve_locations_and_drops() {
    for seed in [1, 2, 3, 0x5eed_5eed] {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut constructed_drop_components = 0;
        let mut rng = Lcg(seed);
        let mut world = World::new();
        let mut slots = Vec::new();

        for step in 0..768 {
            if slots.is_empty() || rng.range(5) == 0 {
                let value = (seed << 32) ^ step;
                spawn_mode(&mut world, &mut slots, rng.range(5), value);
            } else if let Some(index) = pick_live_slot(&mut rng, &slots) {
                let entity = slots[index].entity;
                let state = &mut slots[index].state;
                match rng.range(13) {
                    0 => {
                        let value = rng.next();
                        world.entity_mut(entity).insert(DenseA(value));
                        state.dense_a = Some(value);
                    }
                    1 => {
                        world.entity_mut(entity).remove::<DenseA>();
                        state.dense_a = None;
                    }
                    2 => {
                        let value = rng.next();
                        world.entity_mut(entity).insert(DenseB(value));
                        state.dense_b = Some(value);
                    }
                    3 => {
                        if state.dense_b.is_some() {
                            let DenseB(value) = world.entity_mut(entity).take::<DenseB>().unwrap();
                            assert_eq!(Some(value), state.dense_b.take());
                        }
                    }
                    4 => {
                        let value = rng.next();
                        world.entity_mut(entity).insert(SparseA(value));
                        state.sparse_a = Some(value);
                    }
                    5 => {
                        if state.sparse_a.is_some() {
                            let SparseA(value) =
                                world.entity_mut(entity).take::<SparseA>().unwrap();
                            assert_eq!(Some(value), state.sparse_a.take());
                        }
                    }
                    6 => {
                        world.entity_mut(entity).insert(Zst);
                        state.zst = true;
                    }
                    7 => {
                        world.entity_mut(entity).remove::<Zst>();
                        state.zst = false;
                    }
                    8 => {
                        let value = rng.next();
                        world.entity_mut(entity).insert(Aligned(value));
                        state.aligned = Some(value);
                    }
                    9 => {
                        world.entity_mut(entity).remove::<Aligned>();
                        state.aligned = None;
                    }
                    10 => {
                        world.entity_mut(entity).insert(RequiredRoot);
                        state.required_root = true;
                        state.required_leaf = true;
                    }
                    11 => {
                        world.entity_mut(entity).insert(DropAudit {
                            drops: drops.clone(),
                        });
                        constructed_drop_components += 1;
                        state.drop_audit = true;
                    }
                    _ => {
                        assert!(world.despawn(entity));
                        slots[index].live = false;
                        slots[index].state = SlotState::default();
                    }
                }
            }

            if step % 127 == 63 {
                let spawned = world
                    .spawn_batch((0..3).map(|offset| (DenseA(step + offset), SparseA(offset))))
                    .collect::<Vec<_>>();
                slots.extend(
                    spawned
                        .into_iter()
                        .enumerate()
                        .map(|(offset, entity)| Slot {
                            entity,
                            live: true,
                            state: SlotState {
                                dense_a: Some(step + offset as u64),
                                sparse_a: Some(offset as u64),
                                ..Default::default()
                            },
                        }),
                );
            }

            if step % 251 == 250 {
                world.clear_entities();
                for slot in &mut slots {
                    slot.live = false;
                    slot.state = SlotState::default();
                }
            }

            if step % 37 == 0 {
                assert_storage_matches_model(&world, &slots, constructed_drop_components, &drops);
            }
        }

        assert_storage_matches_model(&world, &slots, constructed_drop_components, &drops);
        drop(world);
        assert_eq!(drops.load(Ordering::SeqCst), constructed_drop_components);
    }
}

#[test]
fn high_entity_index_sparse_storage_reports_sparse_slots_and_clears() {
    let mut world = World::new();
    let high = Entity::from_index(EntityIndex::from_raw_u32(1_000_000).unwrap());
    world.spawn_empty_at(high).unwrap().insert(SparseA(42));

    assert_eq!(
        world.get::<SparseA>(high).map(|component| component.0),
        Some(42)
    );
    assert!(world.storages.sparse_sets.audit_sparse_slot_count() > 1_000_000);
    let sparse_a_id = world.component_id::<SparseA>().unwrap();
    assert_eq!(
        world.storages.sparse_sets.get(sparse_a_id).unwrap().len(),
        1
    );
    assert!(world.storages.sparse_sets.audit_entity_count() >= 1);
    world
        .storages
        .sparse_sets
        .audit_assert_dense_sparse_mappings();

    world.clear_entities();
    assert_eq!(world.storages.sparse_sets.audit_entity_count(), 0);
    assert_eq!(world.storages.sparse_sets.audit_sparse_slot_count(), 0);
}

#[test]
fn drop_panic_during_clear_does_not_double_drop_on_world_drop() {
    let drops = Arc::new(AtomicUsize::new(0));
    let mut world = World::new();
    world.spawn((PanicOnDrop {
        drops: drops.clone(),
    },));

    let clear_result = catch_unwind(AssertUnwindSafe(|| world.clear_entities()));
    assert!(clear_result.is_err());
    assert_eq!(drops.load(Ordering::SeqCst), 1);

    let drop_result = catch_unwind(AssertUnwindSafe(|| drop(world)));
    assert!(drop_result.is_ok());
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[test]
fn command_panic_leaves_storage_model_checkable_for_later_commands() {
    let drops = Arc::new(AtomicUsize::new(0));
    let mut world = World::new();
    let mut slots = Vec::new();
    let mut queue = CommandQueue::default();

    queue.push(|world: &mut World| {
        world.spawn((DenseA(1), SparseA(2)));
        panic!("storage audit command panic");
    });
    let apply_result = catch_unwind(AssertUnwindSafe(|| queue.apply(&mut world)));
    assert!(apply_result.is_err());
    world
        .storages
        .sparse_sets
        .audit_assert_dense_sparse_mappings();

    {
        let mut commands = Commands::new(&mut queue, &world);
        commands.queue(|world: &mut World| {
            world.spawn((DenseA(3), SparseA(4)));
        });
    }
    queue.apply(&mut world);

    for entity in world.query::<Entity>().iter(&world) {
        slots.push(Slot {
            entity,
            live: true,
            state: SlotState {
                dense_a: world.get::<DenseA>(entity).map(|component| component.0),
                sparse_a: world.get::<SparseA>(entity).map(|component| component.0),
                ..Default::default()
            },
        });
    }
    assert_storage_matches_model(&world, &slots, 0, &drops);
}
