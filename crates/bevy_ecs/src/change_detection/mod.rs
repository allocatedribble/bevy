//! Types that detect when their internal data mutate.

mod maybe_location;
mod params;
mod tick;
mod traits;

pub use maybe_location::MaybeLocation;
pub use params::*;
pub use tick::*;
pub use traits::{DetectChanges, DetectChangesMut};

/// The (arbitrarily chosen) minimum number of world tick increments between `check_tick` scans.
///
/// Change ticks can only be scanned when systems aren't running. Thus, if the threshold is `N`,
/// the maximum is `2 * N - 1` (i.e. the world ticks `N - 1` times, then `N` times).
///
/// If no change is older than `u32::MAX - (2 * N - 1)` following a scan, none of their ages can
/// overflow and cause false positives.
// (518,400,000 = 1000 ticks per frame * 144 frames per second * 3600 seconds per hour)
pub const CHECK_TICK_THRESHOLD: u32 = 518_400_000;

/// The maximum change tick difference that won't overflow before the next `check_tick` scan.
///
/// Changes stop being detected once they become this old.
pub const MAX_CHANGE_AGE: u32 = u32::MAX - (2 * CHECK_TICK_THRESHOLD - 1);

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;
    use bevy_ecs_macros::Resource;
    use bevy_ptr::PtrMut;
    use bevy_reflect::{FromType, ReflectFromPtr};
    use core::{
        ops::{Deref, DerefMut},
        sync::atomic::{AtomicU32, Ordering},
    };

    use crate::{
        change_detection::{
            ComponentTicks, ComponentTicksMut, MaybeLocation, Mut, NonSendMut, Ref, ResMut, Tick,
            CHECK_TICK_THRESHOLD, MAX_CHANGE_AGE,
        },
        component::Component,
        query::{Added, Changed, QueryFilter},
        system::{IntoSystem, Single, System},
        world::World,
    };

    use super::{DetectChanges, DetectChangesMut, MutUntyped};

    #[derive(Component, PartialEq)]
    struct C;

    #[derive(Resource)]
    struct R;

    #[derive(Resource, PartialEq)]
    struct R2(u8);

    #[derive(Component)]
    struct TableTracked(u32);

    #[derive(Component)]
    #[component(storage = "SparseSet")]
    #[expect(
        dead_code,
        reason = "change-detection regression payload stays non-ZST"
    )]
    struct SparseTracked(u32);

    #[derive(Resource)]
    struct InteriorResource(AtomicU32);

    impl Deref for R2 {
        type Target = u8;
        fn deref(&self) -> &u8 {
            &self.0
        }
    }

    impl DerefMut for R2 {
        fn deref_mut(&mut self) -> &mut u8 {
            &mut self.0
        }
    }

    #[test]
    fn change_expiration() {
        fn change_detected(query: Option<Single<Ref<C>>>) -> bool {
            query.unwrap().is_changed()
        }

        fn change_expired(query: Option<Single<Ref<C>>>) -> bool {
            query.unwrap().is_changed()
        }

        let mut world = World::new();

        // component added: 1, changed: 1
        world.spawn(C);

        let mut change_detected_system = IntoSystem::into_system(change_detected);
        let mut change_expired_system = IntoSystem::into_system(change_expired);
        change_detected_system.initialize(&mut world);
        change_expired_system.initialize(&mut world);

        // world: 1, system last ran: 0, component changed: 1
        // The spawn will be detected since it happened after the system "last ran".
        assert!(change_detected_system.run((), &mut world).unwrap());

        // world: 1 + MAX_CHANGE_AGE
        let change_tick = world.change_tick.get_mut();
        *change_tick = change_tick.wrapping_add(MAX_CHANGE_AGE);

        // Both the system and component appeared `MAX_CHANGE_AGE` ticks ago.
        // Since we clamp things to `MAX_CHANGE_AGE` for determinism,
        // `ComponentTicks::is_changed` will now see `MAX_CHANGE_AGE > MAX_CHANGE_AGE`
        // and return `false`.
        assert!(!change_expired_system.run((), &mut world).unwrap());
    }

    #[test]
    fn change_tick_wraparound() {
        let mut world = World::new();
        world.last_change_tick = Tick::new(u32::MAX);
        *world.change_tick.get_mut() = 0;

        // component added: 0, changed: 0
        world.spawn(C);

        world.increment_change_tick();

        // Since the world is always ahead, as long as changes can't get older than `u32::MAX` (which we ensure),
        // the wrapping difference will always be positive, so wraparound doesn't matter.
        let mut query = world.query::<Ref<C>>();
        assert!(query.single(&world).unwrap().is_changed());
    }

    #[test]
    fn change_tick_scan() {
        let mut world = World::new();

        // component added: 1, changed: 1
        world.spawn(C);

        // a bunch of stuff happens, the component is now older than `MAX_CHANGE_AGE`
        *world.change_tick.get_mut() += MAX_CHANGE_AGE + CHECK_TICK_THRESHOLD;
        let change_tick = world.change_tick();

        let mut query = world.query::<Ref<C>>();
        for tracker in query.iter(&world) {
            let ticks_since_insert = change_tick.relative_to(*tracker.ticks.added).get();
            let ticks_since_change = change_tick.relative_to(*tracker.ticks.changed).get();
            assert!(ticks_since_insert > MAX_CHANGE_AGE);
            assert!(ticks_since_change > MAX_CHANGE_AGE);
        }

        // scan change ticks and clamp those at risk of overflow
        world.check_change_ticks();

        for tracker in query.iter(&world) {
            let ticks_since_insert = change_tick.relative_to(*tracker.ticks.added).get();
            let ticks_since_change = change_tick.relative_to(*tracker.ticks.changed).get();
            assert_eq!(ticks_since_insert, MAX_CHANGE_AGE);
            assert_eq!(ticks_since_change, MAX_CHANGE_AGE);
        }
    }

    fn count_filtered<F: QueryFilter>(world: &mut World) -> usize {
        world.query_filtered::<(), F>().iter(world).count()
    }

    fn tick_behind(tick: Tick, age: u32) -> Tick {
        Tick::new(tick.get().wrapping_sub(age))
    }

    fn expected_change_visible(event_tick: Tick, last_run: Tick, this_run: Tick) -> bool {
        let event_age = this_run
            .get()
            .wrapping_sub(event_tick.get())
            .min(MAX_CHANGE_AGE);
        let system_age = this_run
            .get()
            .wrapping_sub(last_run.get())
            .min(MAX_CHANGE_AGE);
        system_age > event_age
    }

    #[test]
    fn randomized_tick_window_oracle_matches_table_and_sparse_filters() {
        let mut seed = 0x5eed_c0de_u32;
        let mut interesting_ages = Vec::from([
            0,
            1,
            CHECK_TICK_THRESHOLD - 1,
            CHECK_TICK_THRESHOLD,
            MAX_CHANGE_AGE - 1,
            MAX_CHANGE_AGE,
            MAX_CHANGE_AGE.wrapping_add(1),
            u32::MAX - 1,
            u32::MAX,
        ]);

        for _ in 0..64 {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            interesting_ages.push(seed);
        }

        for (index, event_age) in interesting_ages.iter().copied().enumerate() {
            for system_age in interesting_ages.iter().copied().skip(index % 7).step_by(7) {
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                let this_run = Tick::new(seed);
                let event_tick = tick_behind(this_run, event_age);
                let last_run = tick_behind(this_run, system_age);
                let expected = expected_change_visible(event_tick, last_run, this_run);

                let mut world = World::new();
                *world.change_tick.get_mut() = event_tick.get();
                world.spawn((TableTracked(index as u32), SparseTracked(system_age)));
                *world.change_tick.get_mut() = this_run.get();

                world.last_change_tick_scope(last_run, |world| {
                    assert_eq!(
                        count_filtered::<Added<TableTracked>>(world) == 1,
                        expected,
                        "table Added mismatch for event_age={event_age} system_age={system_age}"
                    );
                    assert_eq!(
                        count_filtered::<Changed<TableTracked>>(world) == 1,
                        expected,
                        "table Changed mismatch for event_age={event_age} system_age={system_age}"
                    );
                    assert_eq!(
                        count_filtered::<Added<SparseTracked>>(world) == 1,
                        expected,
                        "sparse Added mismatch for event_age={event_age} system_age={system_age}"
                    );
                    assert_eq!(
                        count_filtered::<Changed<SparseTracked>>(world) == 1,
                        expected,
                        "sparse Changed mismatch for event_age={event_age} system_age={system_age}"
                    );
                });
            }
        }
    }

    #[test]
    fn add_change_remove_resource_and_sparse_sequences_update_filters() {
        let mut world = World::new();
        world.insert_resource(R2(0));
        let entity = world.spawn((TableTracked(0), SparseTracked(0))).id();

        assert_eq!(count_filtered::<Added<TableTracked>>(&mut world), 1);
        assert_eq!(count_filtered::<Changed<TableTracked>>(&mut world), 1);
        assert_eq!(count_filtered::<Added<SparseTracked>>(&mut world), 1);
        assert_eq!(count_filtered::<Changed<SparseTracked>>(&mut world), 1);
        assert!(world.is_resource_changed::<R2>());

        world.clear_trackers();
        assert_eq!(count_filtered::<Added<TableTracked>>(&mut world), 0);
        assert_eq!(count_filtered::<Changed<TableTracked>>(&mut world), 0);
        assert_eq!(count_filtered::<Added<SparseTracked>>(&mut world), 0);
        assert_eq!(count_filtered::<Changed<SparseTracked>>(&mut world), 0);
        assert!(!world.is_resource_changed::<R2>());

        world
            .entity_mut(entity)
            .get_mut::<TableTracked>()
            .unwrap()
            .0 = 1;
        world.entity_mut(entity).remove::<SparseTracked>();
        world.resource_mut::<R2>().set_if_neq(R2(1));

        assert_eq!(count_filtered::<Added<TableTracked>>(&mut world), 0);
        assert_eq!(count_filtered::<Changed<TableTracked>>(&mut world), 1);
        assert_eq!(count_filtered::<Added<SparseTracked>>(&mut world), 0);
        assert_eq!(count_filtered::<Changed<SparseTracked>>(&mut world), 0);
        assert!(world.is_resource_changed::<R2>());

        world.clear_trackers();
        world.entity_mut(entity).insert(SparseTracked(1));

        assert_eq!(count_filtered::<Added<TableTracked>>(&mut world), 0);
        assert_eq!(count_filtered::<Changed<TableTracked>>(&mut world), 0);
        assert_eq!(count_filtered::<Added<SparseTracked>>(&mut world), 1);
        assert_eq!(count_filtered::<Changed<SparseTracked>>(&mut world), 1);
        assert!(!world.is_resource_changed::<R2>());
    }

    #[test]
    fn interior_mutability_requires_manual_set_changed() {
        let mut world = World::new();
        world.insert_resource(InteriorResource(AtomicU32::new(0)));
        world.clear_trackers();

        world
            .resource::<InteriorResource>()
            .0
            .store(1, Ordering::Relaxed);
        assert!(!world.is_resource_changed::<InteriorResource>());

        world.resource_mut::<InteriorResource>().set_changed();
        assert!(world.is_resource_changed::<InteriorResource>());
    }

    #[test]
    fn last_change_tick_scope_controls_resource_detection_and_restores_tick() {
        let mut world = World::new();
        world.insert_resource(R2(0));
        world.clear_trackers();
        let saved_last_change_tick = world.last_change_tick();

        world.resource_mut::<R2>().set_if_neq(R2(1));
        assert!(world.is_resource_changed::<R2>());

        let this_run = world.change_tick();
        assert!(!world.last_change_tick_scope(this_run, |world| world.is_resource_changed::<R2>()));
        assert_eq!(world.last_change_tick(), saved_last_change_tick);
    }

    #[test]
    fn mut_from_res_mut() {
        let mut component_ticks = ComponentTicks {
            added: Tick::new(1),
            changed: Tick::new(2),
        };
        let mut caller = MaybeLocation::caller();
        let ticks = ComponentTicksMut {
            added: &mut component_ticks.added,
            changed: &mut component_ticks.changed,
            changed_by: caller.as_mut(),
            last_run: Tick::new(3),
            this_run: Tick::new(4),
        };
        let mut res = R {};

        let res_mut = ResMut {
            value: &mut res,
            ticks,
        };

        let into_mut: Mut<R> = res_mut.into();
        assert_eq!(1, into_mut.ticks.added.get());
        assert_eq!(2, into_mut.ticks.changed.get());
        assert_eq!(3, into_mut.ticks.last_run.get());
        assert_eq!(4, into_mut.ticks.this_run.get());
    }

    #[test]
    fn mut_new() {
        let mut component_ticks = ComponentTicks {
            added: Tick::new(1),
            changed: Tick::new(3),
        };
        let mut res = R {};
        let mut caller = MaybeLocation::caller();

        let val = Mut::new(
            &mut res,
            &mut component_ticks.added,
            &mut component_ticks.changed,
            Tick::new(2), // last_run
            Tick::new(4), // this_run
            caller.as_mut(),
        );

        assert!(!val.is_added());
        assert!(val.is_changed());
    }

    #[test]
    fn mut_from_non_send_mut() {
        let mut component_ticks = ComponentTicks {
            added: Tick::new(1),
            changed: Tick::new(2),
        };
        let mut caller = MaybeLocation::caller();
        let ticks = ComponentTicksMut {
            added: &mut component_ticks.added,
            changed: &mut component_ticks.changed,
            changed_by: caller.as_mut(),
            last_run: Tick::new(3),
            this_run: Tick::new(4),
        };
        let mut res = R {};

        let non_send_mut = NonSendMut {
            value: &mut res,
            ticks,
        };

        let into_mut: Mut<R> = non_send_mut.into();
        assert_eq!(1, into_mut.ticks.added.get());
        assert_eq!(2, into_mut.ticks.changed.get());
        assert_eq!(3, into_mut.ticks.last_run.get());
        assert_eq!(4, into_mut.ticks.this_run.get());
    }

    #[test]
    fn map_mut() {
        use super::*;
        struct Outer(i64);

        let last_run = Tick::new(2);
        let this_run = Tick::new(3);
        let mut component_ticks = ComponentTicks {
            added: Tick::new(1),
            changed: Tick::new(2),
        };
        let mut caller = MaybeLocation::caller();
        let ticks = ComponentTicksMut {
            added: &mut component_ticks.added,
            changed: &mut component_ticks.changed,
            changed_by: caller.as_mut(),
            last_run,
            this_run,
        };

        let mut outer = Outer(0);

        let ptr = Mut {
            value: &mut outer,
            ticks,
        };
        assert!(!ptr.is_changed());

        // Perform a mapping operation.
        let mut inner = ptr.map_unchanged(|x| &mut x.0);
        assert!(!inner.is_changed());

        // Mutate the inner value.
        *inner = 64;
        assert!(inner.is_changed());
        // Modifying one field of a component should flag a change for the entire component.
        assert!(component_ticks.is_changed(last_run, this_run));
    }

    #[test]
    fn set_if_neq() {
        let mut world = World::new();

        world.insert_resource(R2(0));
        // Resources are Changed when first added
        world.increment_change_tick();
        // This is required to update world::last_change_tick
        world.clear_trackers();

        let mut r = world.resource_mut::<R2>();
        assert!(!r.is_changed(), "Resource must begin unchanged.");

        r.set_if_neq(R2(0));
        assert!(
            !r.is_changed(),
            "Resource must not be changed after setting to the same value."
        );

        r.set_if_neq(R2(3));
        assert!(
            r.is_changed(),
            "Resource must be changed after setting to a different value."
        );
    }

    #[test]
    fn as_deref_mut() {
        let mut world = World::new();

        world.insert_resource(R2(0));
        // Resources are Changed when first added
        world.increment_change_tick();
        // This is required to update world::last_change_tick
        world.clear_trackers();

        let mut r = world.resource_mut::<R2>();
        assert!(!r.is_changed(), "Resource must begin unchanged.");

        let mut r = r.as_deref_mut();
        assert!(
            !r.is_changed(),
            "Dereferencing should not mark the item as changed yet"
        );

        r.set_if_neq(3);
        assert!(
            r.is_changed(),
            "Resource must be changed after setting to a different value."
        );
    }

    #[test]
    fn mut_untyped_to_reflect() {
        let last_run = Tick::new(2);
        let this_run = Tick::new(3);
        let mut component_ticks = ComponentTicks {
            added: Tick::new(1),
            changed: Tick::new(2),
        };
        let mut caller = MaybeLocation::caller();
        let ticks = ComponentTicksMut {
            added: &mut component_ticks.added,
            changed: &mut component_ticks.changed,
            changed_by: caller.as_mut(),
            last_run,
            this_run,
        };

        let mut value: i32 = 5;

        let value = MutUntyped {
            value: PtrMut::from(&mut value),
            ticks,
        };

        let reflect_from_ptr = <ReflectFromPtr as FromType<i32>>::from_type();

        let mut new = value.map_unchanged(|ptr| {
            // SAFETY: The underlying type of `ptr` matches `reflect_from_ptr`.
            unsafe { reflect_from_ptr.as_reflect_mut(ptr) }
        });

        assert!(!new.is_changed());

        new.reflect_mut();

        assert!(new.is_changed());
    }

    #[test]
    fn mut_untyped_from_mut() {
        let mut component_ticks = ComponentTicks {
            added: Tick::new(1),
            changed: Tick::new(2),
        };
        let mut caller = MaybeLocation::caller();
        let ticks = ComponentTicksMut {
            added: &mut component_ticks.added,
            changed: &mut component_ticks.changed,
            changed_by: caller.as_mut(),
            last_run: Tick::new(3),
            this_run: Tick::new(4),
        };
        let mut c = C {};

        let mut_typed = Mut {
            value: &mut c,
            ticks,
        };

        let into_mut: MutUntyped = mut_typed.into();
        assert_eq!(1, into_mut.ticks.added.get());
        assert_eq!(2, into_mut.ticks.changed.get());
        assert_eq!(3, into_mut.ticks.last_run.get());
        assert_eq!(4, into_mut.ticks.this_run.get());
    }
}
