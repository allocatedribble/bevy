use super::{prepare::SolariLightingResources, SolariLighting};
use bevy_camera::Camera;
use bevy_ecs::{
    entity::Entity,
    lifecycle::RemovedComponents,
    query::Has,
    system::{Commands, Query, ResMut},
};
use bevy_pbr::deferred::SkipDeferredLighting;
use bevy_render::{sync_world::RenderEntity, Extract, MainWorld};

pub fn extract_solari_lighting(
    mut main_world: ResMut<MainWorld>,
    render_state: Query<(Option<&SolariLighting>, Has<SkipDeferredLighting>)>,
    mut commands: Commands,
) {
    let mut cameras_3d = main_world.query::<(RenderEntity, &Camera, Option<&mut SolariLighting>)>();

    for (entity, camera, solari_lighting) in cameras_3d.iter_mut(&mut main_world) {
        let mut entity_commands = commands
            .get_entity(entity)
            .expect("Camera entity wasn't synced.");
        if let Some(mut solari_lighting) = solari_lighting {
            if camera.is_active {
                if !render_solari_state_matches(&render_state, entity, solari_lighting.reset) {
                    entity_commands.insert((solari_lighting.clone(), SkipDeferredLighting));
                }
                solari_lighting.reset = false;
            } else if render_solari_state_present(&render_state, entity) {
                entity_commands.remove::<(
                    SolariLighting,
                    SolariLightingResources,
                    SkipDeferredLighting,
                )>();
            }
        }
    }
}

fn render_solari_state_matches(
    render_state: &Query<(Option<&SolariLighting>, Has<SkipDeferredLighting>)>,
    entity: Entity,
    reset: bool,
) -> bool {
    render_state.get(entity).is_ok_and(|(lighting, skip)| {
        skip && lighting.is_some_and(|lighting| lighting.reset == reset)
    })
}

fn render_solari_state_present(
    render_state: &Query<(Option<&SolariLighting>, Has<SkipDeferredLighting>)>,
    entity: Entity,
) -> bool {
    render_state
        .get(entity)
        .is_ok_and(|(lighting, skip)| skip || lighting.is_some())
}

pub fn extract_removed_solari_lighting(
    mut removed_lighting: Extract<RemovedComponents<SolariLighting>>,
    render_entities: Extract<Query<&RenderEntity>>,
    mut commands: Commands,
) {
    for entity in removed_lighting.read() {
        if let Ok(render_entity) = render_entities.get(entity) {
            commands.entity(render_entity.id()).remove::<(
                SolariLighting,
                SolariLightingResources,
                SkipDeferredLighting,
            )>();
        }
    }
}
