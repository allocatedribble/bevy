//! Centralized storage for observers, allowing for efficient look-ups.
//!
//! This has multiple levels:
//! - [`World::observers`](crate::world::World::observers) provides access to [`Observers`], which is a central storage for all observers.
//! - [`Observers`] contains multiple distinct caches in the form of [`CachedObservers`].
//!     - Most observers are looked up by the [`ComponentId`] of the event they are observing
//!     - Lifecycle observers have their own fields to save lookups.
//! - [`CachedObservers`] contains maps of [`ObserverRunner`]s, which are the actual functions that will be run when the observer is triggered.
//!     - These are split by target type, in order to allow for different lookup strategies.
//!     - [`CachedComponentObservers`] is one of these maps, which contains observers that are specifically targeted at a component.

use bevy_platform::collections::HashMap;

use crate::{
    archetype::ArchetypeFlags, component::ComponentId, entity::EntityHashMap, event::EventKey,
    observer::ObserverRunner,
};
#[cfg(feature = "bevy_ecs_audit")]
use core::mem::size_of;

/// An internal lookup table tracking all of the observers in the world.
///
/// Stores a cache mapping event ids to their registered observers.
/// Some observer kinds (like [lifecycle](crate::lifecycle) observers) have a dedicated field,
/// saving lookups for the most common triggers.
///
/// This can be accessed via [`World::observers`](crate::world::World::observers).
#[derive(Default, Debug)]
pub struct Observers {
    // Cached ECS observers to save a lookup for high-traffic built-in event types.
    add: CachedObservers,
    insert: CachedObservers,
    discard: CachedObservers,
    remove: CachedObservers,
    despawn: CachedObservers,
    // Map from event type to set of observers watching for that event
    cache: HashMap<EventKey, CachedObservers>,
}

impl Observers {
    pub(crate) fn get_observers_mut(&mut self, event_key: EventKey) -> &mut CachedObservers {
        use crate::lifecycle::*;

        match event_key {
            ADD => &mut self.add,
            INSERT => &mut self.insert,
            DISCARD => &mut self.discard,
            REMOVE => &mut self.remove,
            DESPAWN => &mut self.despawn,
            _ => self.cache.entry(event_key).or_default(),
        }
    }

    /// Attempts to get the observers for the given `event_key`.
    ///
    /// When accessing the observers for lifecycle events, such as [`Add`], [`Insert`], [`Discard`], [`Remove`], and [`Despawn`],
    /// use the [`EventKey`] constants from the [`lifecycle`](crate::lifecycle) module.
    ///
    /// [`Add`]: crate::lifecycle::Add
    /// [`Insert`]: crate::lifecycle::Insert
    /// [`Discard`]: crate::lifecycle::Discard
    /// [`Remove`]: crate::lifecycle::Remove
    /// [`Despawn`]: crate::lifecycle::Despawn
    pub fn try_get_observers(&self, event_key: EventKey) -> Option<&CachedObservers> {
        use crate::lifecycle::*;

        match event_key {
            ADD => Some(&self.add),
            INSERT => Some(&self.insert),
            DISCARD => Some(&self.discard),
            REMOVE => Some(&self.remove),
            DESPAWN => Some(&self.despawn),
            _ => self.cache.get(&event_key),
        }
    }

    pub(crate) fn is_archetype_cached(event_key: EventKey) -> Option<ArchetypeFlags> {
        use crate::lifecycle::*;

        match event_key {
            ADD => Some(ArchetypeFlags::ON_ADD_OBSERVER),
            INSERT => Some(ArchetypeFlags::ON_INSERT_OBSERVER),
            DISCARD => Some(ArchetypeFlags::ON_DISCARD_OBSERVER),
            REMOVE => Some(ArchetypeFlags::ON_REMOVE_OBSERVER),
            DESPAWN => Some(ArchetypeFlags::ON_DESPAWN_OBSERVER),
            _ => None,
        }
    }

    pub(crate) fn update_archetype_flags(
        &self,
        component_id: ComponentId,
        flags: &mut ArchetypeFlags,
    ) {
        if self.add.component_observers.contains_key(&component_id) {
            flags.insert(ArchetypeFlags::ON_ADD_OBSERVER);
        }

        if self.insert.component_observers.contains_key(&component_id) {
            flags.insert(ArchetypeFlags::ON_INSERT_OBSERVER);
        }

        if self.discard.component_observers.contains_key(&component_id) {
            flags.insert(ArchetypeFlags::ON_DISCARD_OBSERVER);
        }

        if self.remove.component_observers.contains_key(&component_id) {
            flags.insert(ArchetypeFlags::ON_REMOVE_OBSERVER);
        }

        if self.despawn.component_observers.contains_key(&component_id) {
            flags.insert(ArchetypeFlags::ON_DESPAWN_OBSERVER);
        }
    }

    pub(crate) fn remove_empty_cache(&mut self, event_key: EventKey) {
        use crate::lifecycle::*;

        if matches!(event_key, ADD | INSERT | DISCARD | REMOVE | DESPAWN) {
            return;
        }

        if self
            .cache
            .get(&event_key)
            .is_some_and(CachedObservers::is_empty)
        {
            self.cache.remove(&event_key);
        }
    }

    #[cfg(feature = "bevy_ecs_audit")]
    pub(crate) fn audit_event_cache_entries(&self) -> usize {
        self.cache.len()
    }

    #[cfg(feature = "bevy_ecs_audit")]
    pub(crate) fn audit_event_cache_capacity(&self) -> usize {
        self.cache.capacity()
    }

    #[cfg(feature = "bevy_ecs_audit")]
    pub(crate) fn audit_runner_entries(&self) -> usize {
        self.add
            .audit_runner_entries()
            .saturating_add(self.insert.audit_runner_entries())
            .saturating_add(self.discard.audit_runner_entries())
            .saturating_add(self.remove.audit_runner_entries())
            .saturating_add(self.despawn.audit_runner_entries())
            .saturating_add(
                self.cache
                    .values()
                    .map(CachedObservers::audit_runner_entries)
                    .sum(),
            )
    }

    #[cfg(feature = "bevy_ecs_audit")]
    pub(crate) fn audit_runner_capacity(&self) -> usize {
        self.add
            .audit_runner_capacity()
            .saturating_add(self.insert.audit_runner_capacity())
            .saturating_add(self.discard.audit_runner_capacity())
            .saturating_add(self.remove.audit_runner_capacity())
            .saturating_add(self.despawn.audit_runner_capacity())
            .saturating_add(
                self.cache
                    .values()
                    .map(CachedObservers::audit_runner_capacity)
                    .sum(),
            )
    }

    #[cfg(feature = "bevy_ecs_audit")]
    pub(crate) fn audit_retained_bytes(&self) -> usize {
        self.cache
            .capacity()
            .saturating_mul(size_of::<(EventKey, CachedObservers)>())
            .saturating_add(
                self.add
                    .audit_retained_bytes()
                    .saturating_add(self.insert.audit_retained_bytes())
                    .saturating_add(self.discard.audit_retained_bytes())
                    .saturating_add(self.remove.audit_retained_bytes())
                    .saturating_add(self.despawn.audit_retained_bytes()),
            )
            .saturating_add(
                self.cache
                    .values()
                    .map(CachedObservers::audit_retained_bytes)
                    .sum(),
            )
    }
}

/// Collection of [`ObserverRunner`] for [`Observer`](crate::observer::Observer) registered to a particular event.
///
/// This is stored inside of [`Observers`], specialized for each kind of observer.
#[derive(Default, Debug)]
pub struct CachedObservers {
    /// Observers watching for any time this event is triggered, regardless of target.
    /// These will also respond to events targeting specific components or entities
    pub(super) global_observers: ObserverMap,
    /// Observers watching for triggers of events for a specific component
    pub(super) component_observers: HashMap<ComponentId, CachedComponentObservers>,
    /// Observers watching for triggers of events for a specific entity
    pub(super) entity_observers: EntityHashMap<ObserverMap>,
}

impl CachedObservers {
    /// Observers watching for any time this event is triggered, regardless of target.
    /// These will also respond to events targeting specific components or entities
    pub fn global_observers(&self) -> &ObserverMap {
        &self.global_observers
    }

    /// Returns observers watching for triggers of events for a specific component.
    pub fn component_observers(&self) -> &HashMap<ComponentId, CachedComponentObservers> {
        &self.component_observers
    }

    /// Returns observers watching for triggers of events for a specific entity.
    pub fn entity_observers(&self) -> &EntityHashMap<ObserverMap> {
        &self.entity_observers
    }

    /// Returns `true` if no observer runners are cached for this event.
    pub fn is_empty(&self) -> bool {
        self.global_observers.is_empty()
            && self.component_observers.is_empty()
            && self.entity_observers.is_empty()
    }

    #[cfg(feature = "bevy_ecs_audit")]
    fn audit_runner_entries(&self) -> usize {
        self.global_observers
            .len()
            .saturating_add(
                self.component_observers
                    .values()
                    .map(CachedComponentObservers::audit_runner_entries)
                    .sum(),
            )
            .saturating_add(self.entity_observers.values().map(|map| map.len()).sum())
    }

    #[cfg(feature = "bevy_ecs_audit")]
    fn audit_runner_capacity(&self) -> usize {
        self.global_observers
            .capacity()
            .saturating_add(
                self.component_observers
                    .values()
                    .map(CachedComponentObservers::audit_runner_capacity)
                    .sum(),
            )
            .saturating_add(
                self.entity_observers
                    .values()
                    .map(|map| map.capacity())
                    .sum(),
            )
    }

    #[cfg(feature = "bevy_ecs_audit")]
    fn audit_retained_bytes(&self) -> usize {
        self.global_observers
            .capacity()
            .saturating_mul(size_of::<(crate::entity::Entity, ObserverRunner)>())
            .saturating_add(
                self.component_observers
                    .capacity()
                    .saturating_mul(size_of::<(ComponentId, CachedComponentObservers)>()),
            )
            .saturating_add(
                self.component_observers
                    .values()
                    .map(CachedComponentObservers::audit_retained_bytes)
                    .sum(),
            )
            .saturating_add(
                self.entity_observers
                    .capacity()
                    .saturating_mul(size_of::<(crate::entity::Entity, ObserverMap)>()),
            )
            .saturating_add(
                self.entity_observers
                    .values()
                    .map(|observers| {
                        observers
                            .capacity()
                            .saturating_mul(size_of::<(crate::entity::Entity, ObserverRunner)>())
                    })
                    .sum(),
            )
    }
}

/// Map between an observer entity and its [`ObserverRunner`]
pub type ObserverMap = EntityHashMap<ObserverRunner>;

/// Collection of [`ObserverRunner`] for [`Observer`](crate::observer::Observer) registered to a particular event targeted at a specific component.
///
/// This is stored inside of [`CachedObservers`].
#[derive(Default, Debug)]
pub struct CachedComponentObservers {
    // Observers watching for events targeting this component, but not a specific entity
    pub(super) global_observers: ObserverMap,
    // Observers watching for events targeting this component on a specific entity
    pub(super) entity_component_observers: EntityHashMap<ObserverMap>,
}

impl CachedComponentObservers {
    /// Returns observers watching for events targeting this component, but not a specific entity
    pub fn global_observers(&self) -> &ObserverMap {
        &self.global_observers
    }

    /// Returns observers watching for events targeting this component on a specific entity
    pub fn entity_component_observers(&self) -> &EntityHashMap<ObserverMap> {
        &self.entity_component_observers
    }

    /// Returns `true` if no observer runners are cached for this component.
    pub fn is_empty(&self) -> bool {
        self.global_observers.is_empty() && self.entity_component_observers.is_empty()
    }

    #[cfg(feature = "bevy_ecs_audit")]
    fn audit_runner_entries(&self) -> usize {
        self.global_observers.len().saturating_add(
            self.entity_component_observers
                .values()
                .map(|map| map.len())
                .sum(),
        )
    }

    #[cfg(feature = "bevy_ecs_audit")]
    fn audit_runner_capacity(&self) -> usize {
        self.global_observers.capacity().saturating_add(
            self.entity_component_observers
                .values()
                .map(|map| map.capacity())
                .sum(),
        )
    }

    #[cfg(feature = "bevy_ecs_audit")]
    fn audit_retained_bytes(&self) -> usize {
        self.global_observers
            .capacity()
            .saturating_mul(size_of::<(crate::entity::Entity, ObserverRunner)>())
            .saturating_add(
                self.entity_component_observers
                    .capacity()
                    .saturating_mul(size_of::<(crate::entity::Entity, ObserverMap)>()),
            )
            .saturating_add(
                self.entity_component_observers
                    .values()
                    .map(|observers| {
                        observers
                            .capacity()
                            .saturating_mul(size_of::<(crate::entity::Entity, ObserverRunner)>())
                    })
                    .sum(),
            )
    }
}
