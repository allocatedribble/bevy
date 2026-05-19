use super::RaytracingMesh3d;
use bevy_asset::{AssetEvent, AssetId, Assets};
use bevy_ecs::{
    lifecycle::RemovedComponents,
    message::MessageReader,
    resource::Resource,
    system::{Commands, Query, Res, ResMut},
};
use bevy_pbr::{MeshMaterial3d, PreviousGlobalTransform, StandardMaterial};
use bevy_platform::collections::{HashMap, HashSet};
use bevy_render::{sync_world::RenderEntity, Extract};
use bevy_transform::components::GlobalTransform;

pub fn extract_standard_material_events(
    mut material_assets: ResMut<StandardMaterialAssets>,
    mut events: Extract<MessageReader<AssetEvent<StandardMaterial>>>,
) {
    for event in events.read() {
        match *event {
            AssetEvent::Added { id }
            | AssetEvent::Modified { id }
            | AssetEvent::LoadedWithDependencies { id } => {
                material_assets.mark_dirty(id);
            }
            AssetEvent::Removed { id } | AssetEvent::Unused { id } => {
                material_assets.remove(id);
            }
        }
    }
}

pub fn extract_raytracing_scene(
    instances: Extract<
        Query<(
            RenderEntity,
            &RaytracingMesh3d,
            &MeshMaterial3d<StandardMaterial>,
            &GlobalTransform,
            Option<&PreviousGlobalTransform>,
        )>,
    >,
    mut removed_instances: Extract<RemovedComponents<RaytracingMesh3d>>,
    render_entities: Extract<Query<&RenderEntity>>,
    source_materials: Extract<Res<Assets<StandardMaterial>>>,
    mut material_assets: ResMut<StandardMaterialAssets>,
    mut commands: Commands,
) {
    for entity in removed_instances.read() {
        if let Ok(render_entity) = render_entities.get(entity) {
            commands.entity(render_entity.id()).remove::<(
                RaytracingMesh3d,
                MeshMaterial3d<StandardMaterial>,
                GlobalTransform,
                PreviousGlobalTransform,
            )>();
        }
    }

    let mut referenced_materials = HashSet::default();
    for (_, _, material, _, _) in &instances {
        referenced_materials.insert(material.id());
    }
    material_assets.retain_referenced(&referenced_materials);
    for material_id in referenced_materials {
        if material_assets.needs_refresh(material_id)
            && let Some(material) = source_materials.get(material_id)
        {
            material_assets.insert(material_id, material.clone());
        }
    }
    material_assets.finish_extract();

    for (render_entity, mesh, material, transform, previous_frame_transform) in &instances {
        let mut commands = commands.entity(render_entity);

        match previous_frame_transform.cloned() {
            Some(previous_frame_transform) => commands.insert((
                mesh.clone(),
                material.clone(),
                *transform,
                previous_frame_transform,
            )),
            None => commands.insert((mesh.clone(), material.clone(), *transform)),
        };
    }
}

#[derive(Resource, Default)]
pub struct StandardMaterialAssets {
    materials: HashMap<AssetId<StandardMaterial>, StandardMaterial>,
    dirty: HashSet<AssetId<StandardMaterial>>,
    generation: u64,
}

impl StandardMaterialAssets {
    pub fn get(&self, asset_id: &AssetId<StandardMaterial>) -> Option<&StandardMaterial> {
        self.materials.get(asset_id)
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    fn mark_dirty(&mut self, asset_id: AssetId<StandardMaterial>) {
        self.dirty.insert(asset_id);
    }

    fn needs_refresh(&self, asset_id: AssetId<StandardMaterial>) -> bool {
        self.dirty.contains(&asset_id) || !self.materials.contains_key(&asset_id)
    }

    fn insert(&mut self, asset_id: AssetId<StandardMaterial>, material: StandardMaterial) {
        self.materials.insert(asset_id, material);
        self.generation = self.generation.wrapping_add(1);
    }

    fn remove(&mut self, asset_id: AssetId<StandardMaterial>) {
        self.dirty.remove(&asset_id);
        if self.materials.remove(&asset_id).is_some() {
            self.generation = self.generation.wrapping_add(1);
        }
    }

    fn retain_referenced(&mut self, referenced: &HashSet<AssetId<StandardMaterial>>) {
        let before = self.materials.len();
        self.materials
            .retain(|asset_id, _| referenced.contains(asset_id));
        if self.materials.len() != before {
            self.generation = self.generation.wrapping_add(1);
        }
    }

    fn finish_extract(&mut self) {
        self.dirty.clear();
    }
}
