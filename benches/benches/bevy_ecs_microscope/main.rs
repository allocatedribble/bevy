use bevy_ecs::{
    entity::{EntityHashSet, EntityIndexSet},
    event::EntityComponentsTrigger,
    hierarchy::{ChildOf, Children},
    prelude::*,
    relationship::Relationship,
    schedule::{
        ApplyDeferred, MultiThreadedExecutor, Schedule, ScheduleBuildSettings,
        SingleThreadedExecutor,
    },
    system::{Command, Commands, ParallelCommands},
    world::{CommandQueue, World},
};
use core::hint::black_box;
use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use smallvec::SmallVec;

#[derive(Component)]
struct A0;
#[derive(Component)]
struct A1;
#[derive(Component)]
struct A2;
#[derive(Component)]
struct A3;
#[derive(Component)]
struct A4;
#[derive(Component)]
struct A5;
#[derive(Component)]
struct A6;
#[derive(Component)]
struct A7;
#[derive(Component)]
struct A8;
#[derive(Component)]
struct A9;
#[derive(Component)]
struct A10;
#[derive(Component)]
struct A11;
#[derive(Component)]
struct A12;
#[derive(Component)]
struct A13;
#[derive(Component)]
struct A14;
#[derive(Component)]
struct A15;

#[derive(Component)]
struct TableOnly(u32);

#[derive(Component)]
#[component(storage = "SparseSet")]
struct Sparse(u32);

macro_rules! sparse_components {
    ($($name:ident),* $(,)?) => {
        $(
            #[derive(Component)]
            #[component(storage = "SparseSet")]
            #[expect(dead_code, reason = "benchmark payload keeps sparse tick columns non-ZST")]
            struct $name(u32);
        )*
    };
}

sparse_components!(
    Sparse0, Sparse1, Sparse2, Sparse3, Sparse4, Sparse5, Sparse6, Sparse7, Sparse8, Sparse9,
    Sparse10, Sparse11, Sparse12, Sparse13, Sparse14, Sparse15,
);

#[derive(Event)]
struct MicroscopeEvent;

#[derive(EntityEvent)]
struct MicroscopeEntityEvent(Entity);

#[derive(EntityEvent)]
#[entity_event(trigger = EntityComponentsTrigger<'a>)]
struct MicroscopeEntityComponentsEvent(Entity);

#[derive(Component)]
struct TransitionMarker;

#[derive(Component)]
#[relationship(relationship_target = VecRelSources)]
struct VecRel(Entity);

#[derive(Component)]
#[relationship_target(relationship = VecRel)]
struct VecRelSources(Vec<Entity>);

#[derive(Component)]
#[relationship(relationship_target = SmallVecRelSources)]
struct SmallVecRel(Entity);

#[derive(Component)]
#[relationship_target(relationship = SmallVecRel)]
struct SmallVecRelSources(SmallVec<[Entity; 8]>);

#[derive(Component)]
#[relationship(relationship_target = HashSetRelSources)]
struct HashSetRel(Entity);

#[derive(Component)]
#[relationship_target(relationship = HashSetRel)]
struct HashSetRelSources(EntityHashSet);

#[derive(Component)]
#[relationship(relationship_target = IndexSetRelSources)]
struct IndexSetRel(Entity);

#[derive(Component)]
#[relationship_target(relationship = IndexSetRel)]
struct IndexSetRelSources(EntityIndexSet);

macro_rules! wide_components {
    ($($name:ident),* $(,)?) => {
        $(
            #[derive(Component)]
            #[expect(dead_code, reason = "benchmark payload keeps wide table columns non-ZST")]
            struct $name(u64);
        )*
    };
}

wide_components!(W0, W1, W2, W3, W4, W5, W6, W7, W8, W9, W10, W11, W12, W13, W14, W15,);

#[derive(Bundle)]
struct Wide1 {
    c0: W0,
}

#[derive(Bundle)]
struct Wide4 {
    c0: W0,
    c1: W1,
    c2: W2,
    c3: W3,
}

#[derive(Bundle)]
struct Wide16 {
    c0: W0,
    c1: W1,
    c2: W2,
    c3: W3,
    c4: W4,
    c5: W5,
    c6: W6,
    c7: W7,
    c8: W8,
    c9: W9,
    c10: W10,
    c11: W11,
    c12: W12,
    c13: W13,
    c14: W14,
    c15: W15,
}

struct FakeCommand(u64);

impl Command for FakeCommand {
    type Out = ();

    fn apply(self, world: &mut World) {
        black_box((self.0, world.entities().len()));
    }
}

struct NestedCommand(u64);

impl Command for NestedCommand {
    type Out = ();

    fn apply(self, world: &mut World) {
        world.commands().queue(FakeCommand(self.0));
    }
}

struct LargeCommand([u8; 4096]);

impl Command for LargeCommand {
    type Out = ();

    fn apply(self, world: &mut World) {
        black_box((self.0[0], world.entities().len()));
    }
}

fn tiny_system() {}

#[derive(Resource, Default)]
struct SchedulerBenchCounter(u64);

fn counter_system(mut counter: ResMut<SchedulerBenchCounter>) {
    counter.0 = counter.0.wrapping_add(1);
}

fn medium_table_system(mut query: Query<&mut TableOnly>) {
    for mut item in &mut query {
        item.0 = item.0.wrapping_add(1);
    }
}

fn always_true_condition() -> bool {
    true
}

fn queue_spawn_command(mut commands: Commands) {
    commands.spawn(TableOnly(1));
}

fn queue_fake_command(mut commands: Commands) {
    commands.queue(FakeCommand(1));
}

fn parallel_command_system(query: Query<Entity, With<TableOnly>>, par_commands: ParallelCommands) {
    query.par_iter().for_each(|entity| {
        par_commands.command_scope(|mut commands| {
            commands.entity(entity).insert(TransitionMarker);
        });
    });
}

fn add_archetype_bits(world: &mut World, count: usize) {
    for i in 0..count {
        let mut entity = world.spawn_empty();
        if i & 1 != 0 {
            entity.insert(A0);
        }
        if i & 2 != 0 {
            entity.insert(A1);
        }
        if i & 4 != 0 {
            entity.insert(A2);
        }
        if i & 8 != 0 {
            entity.insert(A3);
        }
        if i & 16 != 0 {
            entity.insert(A4);
        }
        if i & 32 != 0 {
            entity.insert(A5);
        }
        if i & 64 != 0 {
            entity.insert(A6);
        }
        if i & 128 != 0 {
            entity.insert(A7);
        }
        if i & 256 != 0 {
            entity.insert(A8);
        }
        if i & 512 != 0 {
            entity.insert(A9);
        }
        if i & 1024 != 0 {
            entity.insert(A10);
        }
        if i & 2048 != 0 {
            entity.insert(A11);
        }
        if i & 4096 != 0 {
            entity.insert(A12);
        }
        if i & 8192 != 0 {
            entity.insert(A13);
        }
        if i & 16384 != 0 {
            entity.insert(A14);
        }
        if i & 32768 != 0 {
            entity.insert(A15);
        }
    }
}

fn spawn_sparse_set_scan_entities(world: &mut World, count: usize) {
    for i in 0..count {
        let value = i as u32;
        let mut entity = world.spawn_empty();
        match i & 15 {
            0 => {
                entity.insert(Sparse0(value));
            }
            1 => {
                entity.insert(Sparse1(value));
            }
            2 => {
                entity.insert(Sparse2(value));
            }
            3 => {
                entity.insert(Sparse3(value));
            }
            4 => {
                entity.insert(Sparse4(value));
            }
            5 => {
                entity.insert(Sparse5(value));
            }
            6 => {
                entity.insert(Sparse6(value));
            }
            7 => {
                entity.insert(Sparse7(value));
            }
            8 => {
                entity.insert(Sparse8(value));
            }
            9 => {
                entity.insert(Sparse9(value));
            }
            10 => {
                entity.insert(Sparse10(value));
            }
            11 => {
                entity.insert(Sparse11(value));
            }
            12 => {
                entity.insert(Sparse12(value));
            }
            13 => {
                entity.insert(Sparse13(value));
            }
            14 => {
                entity.insert(Sparse14(value));
            }
            15 => {
                entity.insert(Sparse15(value));
            }
            _ => unreachable!(),
        };
    }
}

#[cfg(feature = "ecs_audit")]
fn force_change_tick_scan(world: &mut World) {
    black_box(bevy_ecs::audit::force_check_change_ticks(world));
}

#[cfg(not(feature = "ecs_audit"))]
fn force_change_tick_scan(world: &mut World) {
    black_box(world.check_change_ticks());
}

fn spawn_wide(world: &mut World, width: usize, value: u64) -> Entity {
    match width {
        1 => world.spawn(Wide1 { c0: W0(value) }).id(),
        4 => world
            .spawn(Wide4 {
                c0: W0(value),
                c1: W1(value),
                c2: W2(value),
                c3: W3(value),
            })
            .id(),
        16 => world
            .spawn(Wide16 {
                c0: W0(value),
                c1: W1(value),
                c2: W2(value),
                c3: W3(value),
                c4: W4(value),
                c5: W5(value),
                c6: W6(value),
                c7: W7(value),
                c8: W8(value),
                c9: W9(value),
                c10: W10(value),
                c11: W11(value),
                c12: W12(value),
                c13: W13(value),
                c14: W14(value),
                c15: W15(value),
            })
            .id(),
        _ => unreachable!("unsupported wide benchmark width"),
    }
}

fn storage_metric_tuple(world: &World) -> (usize, usize, usize, usize, usize, usize) {
    #[cfg(feature = "ecs_audit")]
    {
        let metrics = bevy_ecs::audit::storage_metrics(world);
        (
            metrics.archetype_count,
            metrics.empty_archetype_count,
            metrics.archetype_edge_entries,
            metrics.table_count,
            metrics.table_entity_capacity,
            metrics.sparse_set_sparse_slots,
        )
    }
    #[cfg(not(feature = "ecs_audit"))]
    {
        (
            world.archetypes().len(),
            world
                .archetypes()
                .iter()
                .filter(|archetype| archetype.is_empty())
                .count(),
            0,
            world.storages().tables.len(),
            world
                .storages()
                .tables
                .iter()
                .map(|table| table.entity_capacity())
                .sum(),
            0,
        )
    }
}

fn query_update_archetypes(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecs_microscope/query_update_archetypes");
    group.warm_up_time(core::time::Duration::from_millis(250));
    group.measurement_time(core::time::Duration::from_secs(2));
    group.sample_size(30);

    for archetype_count in [10, 100, 1_000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(archetype_count),
            &archetype_count,
            |bencher, &archetype_count| {
                bencher.iter_batched(
                    || {
                        let mut world = World::new();
                        let query = world.query::<&A0>();
                        add_archetype_bits(&mut world, archetype_count);
                        (world, query)
                    },
                    |(world, mut query)| {
                        query.update_archetypes(&world);
                        black_box(query.matched_archetypes().count());
                    },
                    BatchSize::LargeInput,
                );
            },
        );
    }

    group.finish();
}

fn optional_sparse_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecs_microscope/optional_sparse_query");
    group.warm_up_time(core::time::Duration::from_millis(250));
    group.measurement_time(core::time::Duration::from_secs(2));
    group.sample_size(30);

    for density_percent in [1, 10, 50, 90] {
        group.bench_with_input(
            BenchmarkId::new("density_percent", density_percent),
            &density_percent,
            |bencher, &density_percent| {
                let mut world = World::new();
                for i in 0..20_000 {
                    let mut entity = world.spawn(TableOnly(i));
                    if i % 100 < density_percent {
                        entity.insert(Sparse(i));
                    }
                }
                let mut query = world.query::<(&TableOnly, Option<&Sparse>)>();

                bencher.iter(|| {
                    let mut sparse_hits = 0usize;
                    let mut checksum = 0u32;
                    for (table, sparse) in query.iter(&world) {
                        checksum ^= table.0;
                        if let Some(sparse) = sparse {
                            sparse_hits += 1;
                            checksum ^= sparse.0;
                        }
                    }
                    black_box((sparse_hits, checksum));
                });
            },
        );
    }

    group.finish();
}

fn command_storm(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecs_microscope/command_storm");
    group.warm_up_time(core::time::Duration::from_millis(250));
    group.measurement_time(core::time::Duration::from_secs(2));
    group.sample_size(30);

    for command_count in [100, 1_000, 10_000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(command_count),
            &command_count,
            |bencher, &command_count| {
                let mut world = World::new();
                let mut queue = CommandQueue::default();
                bencher.iter(|| {
                    let mut commands = Commands::new(&mut queue, &world);
                    for i in 0..command_count {
                        commands.queue(FakeCommand(i as u64));
                    }
                    queue.apply(&mut world);
                });
            },
        );
    }

    group.finish();
}

fn command_structural_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecs_microscope/command_structural_patterns");
    group.warm_up_time(core::time::Duration::from_millis(250));
    group.measurement_time(core::time::Duration::from_secs(2));
    group.sample_size(30);

    for command_count in [100, 1_000, 10_000] {
        group.bench_with_input(
            BenchmarkId::new("commands_spawn_one_by_one", command_count),
            &command_count,
            |bencher, &command_count| {
                bencher.iter_batched(
                    World::new,
                    |mut world| {
                        let mut queue = CommandQueue::default();
                        {
                            let mut commands = Commands::new(&mut queue, &world);
                            for i in 0..command_count {
                                commands.spawn(TableOnly(i as u32));
                            }
                        }
                        queue.apply(&mut world);
                        black_box(world.entities().len());
                    },
                    BatchSize::LargeInput,
                );
            },
        );

        group.bench_with_input(
            BenchmarkId::new("commands_spawn_batch", command_count),
            &command_count,
            |bencher, &command_count| {
                bencher.iter_batched(
                    World::new,
                    |mut world| {
                        let mut queue = CommandQueue::default();
                        Commands::new(&mut queue, &world)
                            .spawn_batch((0..command_count).map(|i| TableOnly(i as u32)));
                        queue.apply(&mut world);
                        black_box(world.entities().len());
                    },
                    BatchSize::LargeInput,
                );
            },
        );

        group.bench_with_input(
            BenchmarkId::new("mixed_spawn_insert_remove_despawn", command_count),
            &command_count,
            |bencher, &command_count| {
                bencher.iter_batched(
                    World::new,
                    |mut world| {
                        let mut queue = CommandQueue::default();
                        {
                            let mut commands = Commands::new(&mut queue, &world);
                            for i in 0..command_count {
                                let entity = commands.spawn(TableOnly(i as u32)).id();
                                commands.entity(entity).insert(Sparse(i as u32));
                                commands.entity(entity).remove::<Sparse>();
                                if i % 4 == 0 {
                                    commands.entity(entity).despawn();
                                }
                            }
                        }
                        queue.apply(&mut world);
                        black_box(world.entities().len());
                    },
                    BatchSize::LargeInput,
                );
            },
        );
    }

    group.finish();
}

fn command_payload_and_append(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecs_microscope/command_payload_append");
    group.warm_up_time(core::time::Duration::from_millis(250));
    group.measurement_time(core::time::Duration::from_secs(2));
    group.sample_size(30);

    for command_count in [100, 1_000, 10_000] {
        group.bench_with_input(
            BenchmarkId::new("many_tiny_commands", command_count),
            &command_count,
            |bencher, &command_count| {
                let mut world = World::new();
                bencher.iter(|| {
                    let mut queue = CommandQueue::default();
                    for i in 0..command_count {
                        queue.push(FakeCommand(i as u64));
                    }
                    queue.apply(&mut world);
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("nested_commands", command_count),
            &command_count,
            |bencher, &command_count| {
                let mut world = World::new();
                bencher.iter(|| {
                    let mut queue = CommandQueue::default();
                    for i in 0..command_count {
                        queue.push(NestedCommand(i as u64));
                    }
                    queue.apply(&mut world);
                });
            },
        );
    }

    for command_count in [10, 100] {
        group.bench_with_input(
            BenchmarkId::new("few_large_commands", command_count),
            &command_count,
            |bencher, &command_count| {
                let mut world = World::new();
                bencher.iter(|| {
                    let mut queue = CommandQueue::default();
                    for _ in 0..command_count {
                        queue.push(LargeCommand([7; 4096]));
                    }
                    queue.apply(&mut world);
                });
            },
        );
    }

    for queue_count in [10, 100, 1_000] {
        group.bench_with_input(
            BenchmarkId::new("append_many_queues", queue_count),
            &queue_count,
            |bencher, &queue_count| {
                bencher.iter_batched(
                    || {
                        let queues = (0..queue_count)
                            .map(|i| {
                                let mut queue = CommandQueue::default();
                                queue.push(FakeCommand(i as u64));
                                queue
                            })
                            .collect::<Vec<_>>();
                        (World::new(), queues)
                    },
                    |(mut world, mut queues)| {
                        let mut combined = CommandQueue::default();
                        for queue in &mut queues {
                            combined.append(queue);
                        }
                        combined.apply(&mut world);
                    },
                    BatchSize::LargeInput,
                );
            },
        );
    }

    group.finish();
}

fn parallel_and_apply_deferred_commands(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecs_microscope/parallel_apply_deferred_commands");
    group.warm_up_time(core::time::Duration::from_millis(250));
    group.measurement_time(core::time::Duration::from_secs(2));
    group.sample_size(20);

    for entity_count in [1_000, 10_000] {
        group.bench_with_input(
            BenchmarkId::new("parallel_commands_insert", entity_count),
            &entity_count,
            |bencher, &entity_count| {
                let mut world = World::new();
                world.spawn_batch((0..entity_count).map(|i| TableOnly(i as u32)));
                let mut schedule = Schedule::default();
                schedule.add_systems(parallel_command_system);
                schedule.run(&mut world);
                let mut marker_query = world.query::<&TransitionMarker>();
                bencher.iter(|| {
                    schedule.run(&mut world);
                    black_box(marker_query.query(&world).count());
                });
            },
        );
    }

    for barrier_count in [1, 4, 16] {
        group.bench_with_input(
            BenchmarkId::new("explicit_apply_deferred_barriers", barrier_count),
            &barrier_count,
            |bencher, &barrier_count| {
                let mut world = World::new();
                let mut schedule = Schedule::default();
                for _ in 0..barrier_count {
                    schedule.add_systems((queue_spawn_command, ApplyDeferred).chain());
                }
                schedule.run(&mut world);
                bencher.iter(|| {
                    schedule.run(&mut world);
                    black_box(world.entities().len());
                });
            },
        );
    }

    group.finish();
}

fn storage_churn(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecs_microscope/storage_churn");
    group.warm_up_time(core::time::Duration::from_millis(250));
    group.measurement_time(core::time::Duration::from_secs(2));
    group.sample_size(30);

    for entity_count in [1_000, 10_000] {
        group.bench_with_input(
            BenchmarkId::new("table_insert_remove", entity_count),
            &entity_count,
            |bencher, &entity_count| {
                bencher.iter_batched(
                    || {
                        let mut world = World::new();
                        let entities = (0..entity_count)
                            .map(|_| world.spawn_empty().id())
                            .collect::<Vec<_>>();
                        (world, entities)
                    },
                    |(mut world, entities)| {
                        for entity in &entities {
                            world.entity_mut(*entity).insert(TableOnly(1));
                        }
                        for entity in &entities {
                            world.entity_mut(*entity).remove::<TableOnly>();
                        }
                        black_box(world.entities().len());
                    },
                    BatchSize::LargeInput,
                );
            },
        );

        group.bench_with_input(
            BenchmarkId::new("sparse_insert_remove", entity_count),
            &entity_count,
            |bencher, &entity_count| {
                bencher.iter_batched(
                    || {
                        let mut world = World::new();
                        let entities = (0..entity_count)
                            .map(|_| world.spawn_empty().id())
                            .collect::<Vec<_>>();
                        (world, entities)
                    },
                    |(mut world, entities)| {
                        for entity in &entities {
                            world.entity_mut(*entity).insert(Sparse(1));
                        }
                        for entity in &entities {
                            world.entity_mut(*entity).remove::<Sparse>();
                        }
                        black_box(world.entities().len());
                    },
                    BatchSize::LargeInput,
                );
            },
        );
    }

    group.finish();
}

fn storage_row_moves_and_growth(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecs_microscope/storage_row_moves_growth");
    group.warm_up_time(core::time::Duration::from_millis(250));
    group.measurement_time(core::time::Duration::from_secs(2));
    group.sample_size(30);

    for width in [1, 4, 16] {
        group.bench_with_input(
            BenchmarkId::new("row_move_insert_remove_1000", width),
            &width,
            |bencher, &width| {
                bencher.iter_batched(
                    || {
                        let mut world = World::new();
                        let entities = (0..1_000)
                            .map(|i| spawn_wide(&mut world, width, i))
                            .collect::<Vec<_>>();
                        (world, entities)
                    },
                    |(mut world, entities)| {
                        for entity in &entities {
                            world.entity_mut(*entity).insert(TransitionMarker);
                        }
                        for entity in &entities {
                            world.entity_mut(*entity).remove::<TransitionMarker>();
                        }
                        black_box(storage_metric_tuple(&world));
                    },
                    BatchSize::LargeInput,
                );
            },
        );

        group.bench_with_input(
            BenchmarkId::new("spawn_growth_10000", width),
            &width,
            |bencher, &width| {
                bencher.iter_batched(
                    World::new,
                    |mut world| {
                        for i in 0..10_000 {
                            spawn_wide(&mut world, width, i);
                        }
                        black_box(storage_metric_tuple(&world));
                    },
                    BatchSize::LargeInput,
                );
            },
        );
    }

    group.finish();
}

fn archetype_churn_and_empty_cache(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecs_microscope/archetype_churn");
    group.warm_up_time(core::time::Duration::from_millis(250));
    group.measurement_time(core::time::Duration::from_secs(2));
    group.sample_size(20);

    for churn_count in [1_000, 10_000] {
        group.bench_with_input(
            BenchmarkId::new("create_transient_combinations", churn_count),
            &churn_count,
            |bencher, &churn_count| {
                bencher.iter_batched(
                    World::new,
                    |mut world| {
                        add_archetype_bits(&mut world, churn_count);
                        world.clear_entities();
                        black_box(storage_metric_tuple(&world));
                    },
                    BatchSize::LargeInput,
                );
            },
        );

        group.bench_with_input(
            BenchmarkId::new("query_update_after_empty_churn", churn_count),
            &churn_count,
            |bencher, &churn_count| {
                bencher.iter_batched(
                    || {
                        let mut world = World::new();
                        add_archetype_bits(&mut world, churn_count);
                        world.clear_entities();
                        world
                    },
                    |mut world| {
                        let mut query = world.query::<&A0>();
                        query.update_archetypes(&world);
                        black_box((
                            query.matched_archetypes().count(),
                            storage_metric_tuple(&world),
                        ));
                    },
                    BatchSize::LargeInput,
                );
            },
        );

        group.bench_with_input(
            BenchmarkId::new("world_clear_after_churn", churn_count),
            &churn_count,
            |bencher, &churn_count| {
                bencher.iter_batched(
                    || {
                        let mut world = World::new();
                        add_archetype_bits(&mut world, churn_count);
                        world
                    },
                    |mut world| {
                        world.clear_entities();
                        black_box(storage_metric_tuple(&world));
                    },
                    BatchSize::LargeInput,
                );
            },
        );
    }

    group.finish();
}

fn sparse_high_index(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecs_microscope/sparse_high_index");
    group.warm_up_time(core::time::Duration::from_millis(250));
    group.measurement_time(core::time::Duration::from_secs(2));
    group.sample_size(30);

    for entity_index in [10_000, 1_000_000] {
        group.bench_with_input(
            BenchmarkId::new("insert_get_clear", entity_index),
            &entity_index,
            |bencher, &entity_index| {
                bencher.iter_batched(
                    World::new,
                    |mut world| {
                        let entity = Entity::from_raw_u32(entity_index).unwrap();
                        world.spawn_empty_at(entity).unwrap().insert(Sparse(1));
                        let value = world.get::<Sparse>(entity).map(|component| component.0);
                        let before_clear = storage_metric_tuple(&world);
                        world.clear_entities();
                        black_box((value, before_clear, storage_metric_tuple(&world)));
                    },
                    BatchSize::LargeInput,
                );
            },
        );
    }

    group.finish();
}

fn add_tiny_systems(schedule: &mut Schedule, system_count: usize) {
    for _ in 0..system_count {
        schedule.add_systems(tiny_system);
    }
}

fn add_counter_systems(schedule: &mut Schedule, system_count: usize) {
    for _ in 0..system_count {
        schedule.add_systems(counter_system);
    }
}

fn add_conditioned_counter_systems(schedule: &mut Schedule, system_count: usize) {
    for _ in 0..system_count {
        schedule.add_systems(counter_system.run_if(always_true_condition));
    }
}

fn add_every_ten_deferred_block(schedule: &mut Schedule) {
    schedule.add_systems(
        (
            queue_fake_command,
            queue_fake_command,
            queue_fake_command,
            queue_fake_command,
            queue_fake_command,
            queue_fake_command,
            queue_fake_command,
            queue_fake_command,
            queue_fake_command,
            queue_fake_command,
            ApplyDeferred,
        )
            .chain(),
    );
}

fn add_apply_deferred_frequency_systems(
    schedule: &mut Schedule,
    system_count: usize,
    every: usize,
) {
    schedule.set_build_settings(ScheduleBuildSettings {
        auto_insert_apply_deferred: false,
        ..Default::default()
    });

    match every {
        0 => {
            for _ in 0..system_count {
                schedule.add_systems(queue_fake_command);
            }
        }
        1 => {
            for _ in 0..system_count {
                schedule.add_systems((queue_fake_command, ApplyDeferred).chain());
            }
        }
        10 => {
            for _ in 0..(system_count / 10) {
                add_every_ten_deferred_block(schedule);
            }
        }
        _ => unreachable!("unsupported ApplyDeferred frequency"),
    }
}

fn scheduler_pressure(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecs_microscope/scheduler_pressure");
    group.warm_up_time(core::time::Duration::from_millis(250));
    group.measurement_time(core::time::Duration::from_secs(2));
    group.sample_size(20);

    for system_count in [10, 100, 1_000, 10_000] {
        group.bench_with_input(
            BenchmarkId::new("single_threaded", system_count),
            &system_count,
            |bencher, &system_count| {
                let mut world = World::new();
                let mut schedule = Schedule::default();
                schedule.set_executor(SingleThreadedExecutor::new());
                add_tiny_systems(&mut schedule, system_count);
                schedule.run(&mut world);
                bencher.iter(|| schedule.run(&mut world));
            },
        );

        group.bench_with_input(
            BenchmarkId::new("multi_threaded", system_count),
            &system_count,
            |bencher, &system_count| {
                let mut world = World::new();
                let mut schedule = Schedule::default();
                schedule.set_executor(MultiThreadedExecutor::new());
                add_tiny_systems(&mut schedule, system_count);
                schedule.run(&mut world);
                bencher.iter(|| schedule.run(&mut world));
            },
        );
    }

    for system_count in [10, 100, 1_000] {
        group.bench_with_input(
            BenchmarkId::new("multi_threaded_medium_query", system_count),
            &system_count,
            |bencher, &system_count| {
                let mut world = World::new();
                world.spawn_batch((0..1_000).map(|i| TableOnly(i as u32)));
                let mut schedule = Schedule::default();
                schedule.set_executor(MultiThreadedExecutor::new());
                for _ in 0..system_count {
                    schedule.add_systems(medium_table_system);
                }
                schedule.run(&mut world);
                bencher.iter(|| schedule.run(&mut world));
            },
        );
    }

    group.finish();
}

fn scheduler_build_pressure(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecs_microscope/scheduler_build_pressure");
    group.warm_up_time(core::time::Duration::from_millis(250));
    group.measurement_time(core::time::Duration::from_secs(2));
    group.sample_size(10);

    for system_count in [100, 1_000, 10_000] {
        group.bench_with_input(
            BenchmarkId::new("noop_single_threaded_build", system_count),
            &system_count,
            |bencher, &system_count| {
                bencher.iter_batched(
                    || {
                        let mut schedule = Schedule::default();
                        schedule.set_executor(SingleThreadedExecutor::new());
                        add_tiny_systems(&mut schedule, system_count);
                        (World::new(), schedule)
                    },
                    |(mut world, mut schedule)| {
                        black_box(schedule.initialize(&mut world).unwrap());
                    },
                    BatchSize::LargeInput,
                );
            },
        );

        group.bench_with_input(
            BenchmarkId::new("noop_multi_threaded_build", system_count),
            &system_count,
            |bencher, &system_count| {
                bencher.iter_batched(
                    || {
                        let mut schedule = Schedule::default();
                        schedule.set_executor(MultiThreadedExecutor::new());
                        add_tiny_systems(&mut schedule, system_count);
                        (World::new(), schedule)
                    },
                    |(mut world, mut schedule)| {
                        black_box(schedule.initialize(&mut world).unwrap());
                    },
                    BatchSize::LargeInput,
                );
            },
        );
    }

    for system_count in [100, 1_000] {
        group.bench_with_input(
            BenchmarkId::new("conflicting_resmut_build", system_count),
            &system_count,
            |bencher, &system_count| {
                bencher.iter_batched(
                    || {
                        let mut world = World::new();
                        world.init_resource::<SchedulerBenchCounter>();
                        let mut schedule = Schedule::default();
                        schedule.set_executor(MultiThreadedExecutor::new());
                        add_counter_systems(&mut schedule, system_count);
                        (world, schedule)
                    },
                    |(mut world, mut schedule)| {
                        black_box(schedule.initialize(&mut world).unwrap());
                    },
                    BatchSize::LargeInput,
                );
            },
        );

        group.bench_with_input(
            BenchmarkId::new("conditioned_resmut_build", system_count),
            &system_count,
            |bencher, &system_count| {
                bencher.iter_batched(
                    || {
                        let mut world = World::new();
                        world.init_resource::<SchedulerBenchCounter>();
                        let mut schedule = Schedule::default();
                        schedule.set_executor(MultiThreadedExecutor::new());
                        add_conditioned_counter_systems(&mut schedule, system_count);
                        (world, schedule)
                    },
                    |(mut world, mut schedule)| {
                        black_box(schedule.initialize(&mut world).unwrap());
                    },
                    BatchSize::LargeInput,
                );
            },
        );
    }

    group.finish();
}

fn scheduler_apply_deferred_frequency(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecs_microscope/scheduler_apply_deferred_frequency");
    group.warm_up_time(core::time::Duration::from_millis(250));
    group.measurement_time(core::time::Duration::from_secs(2));
    group.sample_size(20);

    for system_count in [100, 1_000] {
        for (name, every) in [
            ("final_only", 0usize),
            ("every_10_systems", 10),
            ("every_system", 1),
        ] {
            group.bench_with_input(
                BenchmarkId::new(name, system_count),
                &(system_count, every),
                |bencher, &(system_count, every)| {
                    let mut world = World::new();
                    let mut schedule = Schedule::default();
                    schedule.set_executor(MultiThreadedExecutor::new());
                    add_apply_deferred_frequency_systems(&mut schedule, system_count, every);
                    schedule.run(&mut world);
                    bencher.iter(|| schedule.run(&mut world));
                },
            );
        }
    }

    group.finish();
}

fn change_detection_scan(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecs_microscope/change_detection_scan");
    group.warm_up_time(core::time::Duration::from_millis(250));
    group.measurement_time(core::time::Duration::from_secs(2));
    group.sample_size(10);

    group.bench_function("check_ticks_1m_table_components", |bencher| {
        let mut world = World::new();
        world.spawn_batch((0..1_000_000).map(|i| TableOnly(i as u32)));
        bencher.iter(|| force_change_tick_scan(&mut world));
    });

    for archetype_count in [1_000, 10_000] {
        group.bench_with_input(
            BenchmarkId::new("check_ticks_many_tables", archetype_count),
            &archetype_count,
            |bencher, &archetype_count| {
                let mut world = World::new();
                add_archetype_bits(&mut world, archetype_count);
                bencher.iter(|| force_change_tick_scan(&mut world));
            },
        );
    }

    for entity_count in [10_000, 100_000] {
        group.bench_with_input(
            BenchmarkId::new("check_ticks_many_sparse_sets", entity_count),
            &entity_count,
            |bencher, &entity_count| {
                let mut world = World::new();
                spawn_sparse_set_scan_entities(&mut world, entity_count);
                bencher.iter(|| force_change_tick_scan(&mut world));
            },
        );
    }

    group.finish();
}

fn change_detection_filters(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecs_microscope/change_detection_filters");
    group.warm_up_time(core::time::Duration::from_millis(250));
    group.measurement_time(core::time::Duration::from_secs(2));
    group.sample_size(20);

    for entity_count in [10_000, 100_000] {
        group.bench_with_input(
            BenchmarkId::new("changed_table_almost_nothing", entity_count),
            &entity_count,
            |bencher, &entity_count| {
                let mut world = World::new();
                let entities = world
                    .spawn_batch((0..entity_count).map(|i| TableOnly(i as u32)))
                    .collect::<Vec<_>>();
                world.clear_trackers();
                for entity in entities.iter().take(entity_count / 100) {
                    if let Some(mut item) = world.entity_mut(*entity).get_mut::<TableOnly>() {
                        item.0 = item.0.wrapping_add(1);
                    }
                }
                let mut query = world.query_filtered::<&TableOnly, Changed<TableOnly>>();
                bencher.iter(|| black_box(query.iter(&world).count()));
            },
        );

        group.bench_with_input(
            BenchmarkId::new("changed_table_everything", entity_count),
            &entity_count,
            |bencher, &entity_count| {
                let mut world = World::new();
                world.spawn_batch((0..entity_count).map(|i| TableOnly(i as u32)));
                let mut query = world.query_filtered::<&TableOnly, Changed<TableOnly>>();
                bencher.iter(|| black_box(query.iter(&world).count()));
            },
        );

        group.bench_with_input(
            BenchmarkId::new("changed_sparse_almost_nothing", entity_count),
            &entity_count,
            |bencher, &entity_count| {
                let mut world = World::new();
                let entities = world
                    .spawn_batch((0..entity_count).map(|i| Sparse(i as u32)))
                    .collect::<Vec<_>>();
                world.clear_trackers();
                for entity in entities.iter().take(entity_count / 100) {
                    if let Some(mut item) = world.entity_mut(*entity).get_mut::<Sparse>() {
                        item.0 = item.0.wrapping_add(1);
                    }
                }
                let mut query = world.query_filtered::<&Sparse, Changed<Sparse>>();
                bencher.iter(|| black_box(query.iter(&world).count()));
            },
        );

        group.bench_with_input(
            BenchmarkId::new("changed_sparse_everything", entity_count),
            &entity_count,
            |bencher, &entity_count| {
                let mut world = World::new();
                world.spawn_batch((0..entity_count).map(|i| Sparse(i as u32)));
                let mut query = world.query_filtered::<&Sparse, Changed<Sparse>>();
                bencher.iter(|| black_box(query.iter(&world).count()));
            },
        );
    }

    group.finish();
}

fn observer_dispatch_pressure(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecs_microscope/observer_dispatch");
    group.warm_up_time(core::time::Duration::from_millis(250));
    group.measurement_time(core::time::Duration::from_secs(2));
    group.sample_size(20);

    group.bench_function("no_observers_global", |bencher| {
        let mut world = World::new();
        bencher.iter(|| world.trigger(MicroscopeEvent));
    });

    group.bench_function("global_observer", |bencher| {
        let mut world = World::new();
        world.add_observer(|event: On<MicroscopeEvent>| {
            black_box(event);
        });
        bencher.iter(|| world.trigger(MicroscopeEvent));
    });

    group.bench_function("entity_observer", |bencher| {
        let mut world = World::new();
        let entity = world
            .spawn_empty()
            .observe(|event: On<MicroscopeEntityEvent>| {
                black_box(event);
            })
            .id();
        bencher.iter(|| world.trigger(MicroscopeEntityEvent(entity)));
    });

    group.bench_function("component_observer", |bencher| {
        let mut world = World::new();
        let entity = world.spawn_empty().id();
        let component = world.register_component::<TableOnly>();
        world.add_observer(|event: On<MicroscopeEntityComponentsEvent, TableOnly>| {
            black_box(event);
        });
        bencher.iter(|| {
            world.trigger_with(
                MicroscopeEntityComponentsEvent(entity),
                EntityComponentsTrigger {
                    components: &[component],
                    old_archetype: None,
                    new_archetype: None,
                },
            );
        });
    });

    group.bench_function("entity_component_observer", |bencher| {
        let mut world = World::new();
        let component = world.register_component::<TableOnly>();
        let entity = world
            .spawn_empty()
            .observe(|event: On<MicroscopeEntityComponentsEvent, TableOnly>| {
                black_box(event);
            })
            .id();
        bencher.iter(|| {
            world.trigger_with(
                MicroscopeEntityComponentsEvent(entity),
                EntityComponentsTrigger {
                    components: &[component],
                    old_archetype: None,
                    new_archetype: None,
                },
            );
        });
    });

    for observer_count in [1_000, 10_000] {
        group.bench_with_input(
            BenchmarkId::new("many_irrelevant_entity_observers", observer_count),
            &observer_count,
            |bencher, &observer_count| {
                let mut world = World::new();
                let target = world.spawn_empty().id();
                for _ in 0..observer_count {
                    world
                        .spawn_empty()
                        .observe(|event: On<MicroscopeEntityEvent>| {
                            black_box(event);
                        });
                }
                bencher.iter(|| world.trigger(MicroscopeEntityEvent(target)));
            },
        );
    }

    group.bench_function("lifecycle_add_no_observers", |bencher| {
        bencher.iter_batched(
            World::new,
            |mut world| {
                world.spawn(TableOnly(1));
                black_box(world.entities().len());
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("lifecycle_add_with_observer_flags", |bencher| {
        bencher.iter_batched(
            || {
                let mut world = World::new();
                world.add_observer(|event: On<Add, TableOnly>| {
                    black_box(event);
                });
                world
            },
            |mut world| {
                world.spawn(TableOnly(1));
                black_box(world.entities().len());
            },
            BatchSize::SmallInput,
        );
    });

    for archetype_count in [1_000, 10_000] {
        group.bench_with_input(
            BenchmarkId::new("unregister_add_observer_many_archetypes", archetype_count),
            &archetype_count,
            |bencher, &archetype_count| {
                bencher.iter_batched(
                    || {
                        let mut world = World::new();
                        add_archetype_bits(&mut world, archetype_count);
                        let observer = world.add_observer(|_: On<Add, A0>| {}).id();
                        (world, observer)
                    },
                    |(mut world, observer)| {
                        world.despawn(observer);
                        world.flush();
                        black_box(world.entities().len());
                    },
                    BatchSize::LargeInput,
                );
            },
        );
    }

    group.finish();
}

fn observer_and_relationship_storms(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecs_microscope/observer_relationship");
    group.warm_up_time(core::time::Duration::from_millis(250));
    group.measurement_time(core::time::Duration::from_secs(2));
    group.sample_size(30);

    group.bench_function("global_observer_10000", |bencher| {
        let mut world = World::new();
        world.add_observer(|event: On<MicroscopeEvent>| {
            black_box(event);
        });
        bencher.iter(|| {
            for _ in 0..10_000 {
                world.trigger(MicroscopeEvent);
            }
        });
    });

    for child_count in [1_000, 10_000] {
        group.bench_with_input(
            BenchmarkId::new("one_parent_children", child_count),
            &child_count,
            |bencher, &child_count| {
                bencher.iter_batched(
                    || {
                        let mut world = World::new();
                        let parent = world.spawn_empty().id();
                        let children = (0..child_count)
                            .map(|_| world.spawn_empty().id())
                            .collect::<Vec<_>>();
                        (world, parent, children)
                    },
                    |(mut world, parent, children)| {
                        for child in &children {
                            world.entity_mut(*child).insert(ChildOf(parent));
                        }
                        for child in &children {
                            world.entity_mut(*child).remove::<ChildOf>();
                        }
                        black_box(world.entities().len());
                    },
                    BatchSize::LargeInput,
                );
            },
        );
    }

    group.finish();
}

fn spawn_child_chain(world: &mut World, depth: usize) -> (Entity, Entity) {
    let root = world.spawn_empty().id();
    let mut leaf = root;
    for _ in 0..depth {
        leaf = world.spawn(ChildOf(leaf)).id();
    }
    (root, leaf)
}

fn spawn_children(world: &mut World, parent: Entity, child_count: usize) -> Vec<Entity> {
    (0..child_count)
        .map(|_| world.spawn(ChildOf(parent)).id())
        .collect()
}

fn relationship_hierarchy_pressure(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecs_microscope/relationship_hierarchy");
    group.warm_up_time(core::time::Duration::from_millis(150));
    group.measurement_time(core::time::Duration::from_secs(1));
    group.sample_size(10);

    for child_count in [10, 100, 1_000, 10_000, 100_000] {
        group.bench_with_input(
            BenchmarkId::new("add_children_vec", child_count),
            &child_count,
            |bencher, &child_count| {
                bencher.iter_batched(
                    || {
                        let mut world = World::new();
                        let parent = world.spawn_empty().id();
                        let children = (0..child_count)
                            .map(|_| world.spawn_empty().id())
                            .collect::<Vec<_>>();
                        (world, parent, children)
                    },
                    |(mut world, parent, children)| {
                        world.entity_mut(parent).add_children(&children);
                        black_box(world.get::<Children>(parent).map(|children| children.len()));
                    },
                    BatchSize::LargeInput,
                );
            },
        );
    }

    for child_count in [10, 1_000, 10_000, 100_000] {
        for (case, index) in [
            ("remove_first_child", 0),
            ("remove_middle_child", child_count / 2),
            ("remove_last_child", child_count - 1),
        ] {
            group.bench_with_input(
                BenchmarkId::new(case, child_count),
                &child_count,
                |bencher, &child_count| {
                    bencher.iter_batched(
                        || {
                            let mut world = World::new();
                            let parent = world.spawn_empty().id();
                            let children = spawn_children(&mut world, parent, child_count);
                            (world, parent, children[index])
                        },
                        |(mut world, parent, child)| {
                            world.entity_mut(child).remove::<ChildOf>();
                            black_box(world.get::<Children>(parent).map(|children| children.len()));
                        },
                        BatchSize::LargeInput,
                    );
                },
            );
        }
    }

    for child_count in [100, 1_000, 10_000] {
        group.bench_with_input(
            BenchmarkId::new("reparent_many_children", child_count),
            &child_count,
            |bencher, &child_count| {
                bencher.iter_batched(
                    || {
                        let mut world = World::new();
                        let old_parent = world.spawn_empty().id();
                        let new_parent = world.spawn_empty().id();
                        let children = spawn_children(&mut world, old_parent, child_count);
                        (world, old_parent, new_parent, children)
                    },
                    |(mut world, old_parent, new_parent, children)| {
                        for child in children {
                            world.entity_mut(child).insert(ChildOf(new_parent));
                        }
                        black_box(
                            world
                                .get::<Children>(old_parent)
                                .map(|children| children.len()),
                        );
                        black_box(
                            world
                                .get::<Children>(new_parent)
                                .map(|children| children.len()),
                        );
                    },
                    BatchSize::LargeInput,
                );
            },
        );

        group.bench_with_input(
            BenchmarkId::new("despawn_whole_tree", child_count),
            &child_count,
            |bencher, &child_count| {
                bencher.iter_batched(
                    || {
                        let mut world = World::new();
                        let root = world.spawn_empty().id();
                        spawn_children(&mut world, root, child_count);
                        (world, root)
                    },
                    |(mut world, root)| {
                        world.entity_mut(root).despawn();
                        black_box(world.entities().len());
                    },
                    BatchSize::LargeInput,
                );
            },
        );
    }

    for depth in [100, 1_000, 10_000] {
        group.bench_with_input(
            BenchmarkId::new("deep_tree_ancestors", depth),
            &depth,
            |bencher, &depth| {
                let mut world = World::new();
                let (_, leaf) = spawn_child_chain(&mut world, depth);
                let mut query = world.query::<&ChildOf>();
                bencher.iter(|| {
                    black_box(query.query(&world).iter_ancestors::<ChildOf>(leaf).count());
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("deep_tree_descendants", depth),
            &depth,
            |bencher, &depth| {
                let mut world = World::new();
                let (root, _) = spawn_child_chain(&mut world, depth);
                let mut query = world.query::<&Children>();
                bencher.iter(|| {
                    black_box(
                        query
                            .query(&world)
                            .iter_descendants_depth_first::<Children>(root)
                            .count(),
                    );
                });
            },
        );
    }

    for child_count in [100, 1_000, 10_000, 100_000] {
        group.bench_with_input(
            BenchmarkId::new("wide_tree_descendants", child_count),
            &child_count,
            |bencher, &child_count| {
                let mut world = World::new();
                let root = world.spawn_empty().id();
                spawn_children(&mut world, root, child_count);
                let mut query = world.query::<&Children>();
                bencher.iter(|| {
                    black_box(
                        query
                            .query(&world)
                            .iter_descendants::<Children>(root)
                            .count(),
                    );
                });
            },
        );
    }

    macro_rules! bench_collection {
        ($relationship:ty, $name:literal) => {
            for source_count in [10, 100, 1_000, 10_000] {
                group.bench_with_input(
                    BenchmarkId::new(concat!("collection_add_remove/", $name), source_count),
                    &source_count,
                    |bencher, &source_count| {
                        bencher.iter_batched(
                            || {
                                let mut world = World::new();
                                let target = world.spawn_empty().id();
                                let sources = (0..source_count)
                                    .map(|_| world.spawn_empty().id())
                                    .collect::<Vec<_>>();
                                (world, target, sources)
                            },
                            |(mut world, target, sources)| {
                                for source in &sources {
                                    world
                                        .entity_mut(*source)
                                        .insert(<$relationship as Relationship>::from(target));
                                }
                                for source in &sources {
                                    world.entity_mut(*source).remove::<$relationship>();
                                }
                                black_box(world.entities().len());
                            },
                            BatchSize::LargeInput,
                        );
                    },
                );
            }
        };
    }

    bench_collection!(VecRel, "vec");
    bench_collection!(SmallVecRel, "smallvec");
    bench_collection!(HashSetRel, "entity_hash_set");
    bench_collection!(IndexSetRel, "entity_index_set");

    group.finish();
}

criterion_group!(
    benches,
    query_update_archetypes,
    optional_sparse_query,
    command_storm,
    command_structural_patterns,
    command_payload_and_append,
    parallel_and_apply_deferred_commands,
    storage_churn,
    storage_row_moves_and_growth,
    archetype_churn_and_empty_cache,
    sparse_high_index,
    scheduler_pressure,
    scheduler_build_pressure,
    scheduler_apply_deferred_frequency,
    change_detection_scan,
    change_detection_filters,
    observer_dispatch_pressure,
    observer_and_relationship_storms,
    relationship_hierarchy_pressure,
);
criterion_main!(benches);
