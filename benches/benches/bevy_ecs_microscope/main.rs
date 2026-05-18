use bevy_ecs::{
    hierarchy::ChildOf,
    prelude::*,
    schedule::{MultiThreadedExecutor, Schedule, SingleThreadedExecutor},
    system::{Command, Commands},
    world::{CommandQueue, World},
};
use core::hint::black_box;
use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};

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
struct TableOnly(u32);

#[derive(Component)]
#[component(storage = "SparseSet")]
struct Sparse(u32);

#[derive(Event)]
struct MicroscopeEvent;

struct FakeCommand(u64);

impl Command for FakeCommand {
    type Out = ();

    fn apply(self, world: &mut World) {
        black_box((self.0, world.entities().len()));
    }
}

fn tiny_system() {}

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

fn scheduler_pressure(c: &mut Criterion) {
    let mut group = c.benchmark_group("ecs_microscope/scheduler_pressure");
    group.warm_up_time(core::time::Duration::from_millis(250));
    group.measurement_time(core::time::Duration::from_secs(2));
    group.sample_size(30);

    for system_count in [10, 100, 1_000] {
        group.bench_with_input(
            BenchmarkId::new("single_threaded", system_count),
            &system_count,
            |bencher, &system_count| {
                let mut world = World::new();
                let mut schedule = Schedule::default();
                schedule.set_executor(SingleThreadedExecutor::new());
                for _ in 0..system_count {
                    schedule.add_systems(tiny_system);
                }
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
                for _ in 0..system_count {
                    schedule.add_systems(tiny_system);
                }
                schedule.run(&mut world);
                bencher.iter(|| schedule.run(&mut world));
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

criterion_group!(
    benches,
    query_update_archetypes,
    optional_sparse_query,
    command_storm,
    storage_churn,
    scheduler_pressure,
    observer_and_relationship_storms,
);
criterion_main!(benches);
