use super::{blas::BlasManager, extract::StandardMaterialAssets, RaytracingMesh3d};
use bevy_asset::{AssetId, Handle};
use bevy_color::{ColorToComponents, LinearRgba};
use bevy_ecs::{
    entity::{Entity, EntityHashMap},
    resource::Resource,
    system::{Query, Res, ResMut},
};
use bevy_image::Image;
use bevy_math::{ops::cos, Mat4, Vec3};
use bevy_mesh::Mesh;
use bevy_pbr::{
    DfgLut, ExtractedDirectionalLight, MeshMaterial3d, PreviousGlobalTransform, StandardMaterial,
};
use bevy_platform::{collections::HashMap, hash::FixedHasher};
use bevy_render::{
    mesh::allocator::MeshAllocator,
    render_asset::RenderAssets,
    render_resource::{binding_types::*, *},
    renderer::{RenderDevice, RenderQueue},
    texture::{FallbackImage, GpuImage},
};
use bevy_transform::components::GlobalTransform;
use core::{f32::consts::TAU, hash::Hash, num::NonZeroU32, ops::Deref};

const MAX_MESH_SLAB_COUNT: NonZeroU32 = NonZeroU32::new(500).unwrap();
const MAX_TEXTURE_COUNT: NonZeroU32 = NonZeroU32::new(5_000).unwrap();

const TEXTURE_MAP_NONE: u32 = u32::MAX;
const LIGHT_NOT_PRESENT_THIS_FRAME: u32 = u32::MAX;

#[derive(Resource)]
pub struct RaytracingSceneBindings {
    pub bind_group: Option<BindGroup>,
    pub bind_group_layout: BindGroupLayoutDescriptor,
    previous_frame_light_entities: Vec<Entity>,
    last_scene_key: Option<RaytracingSceneKey>,
    last_binding_key: Option<RaytracingBindingKey>,
    instance_cache: RaytracingInstanceCache,
    material_cache: RaytracingMaterialCache,
    _texture_cache: RaytracingTextureCache,
    light_cache: RaytracingLightCache,
    tlas_cache: RaytracingTlasCache,
}

#[derive(Default)]
struct RaytracingInstanceCache {
    transforms: StorageBufferList<Mat4>,
    previous_frame_transforms: StorageBufferList<Mat4>,
    geometry_ids: StorageBufferList<GpuInstanceGeometryIds>,
    material_ids: StorageBufferList<u32>,
}

#[derive(Default)]
struct RaytracingMaterialCache {
    materials: StorageBufferList<GpuMaterial>,
}

#[derive(Default)]
struct RaytracingTextureCache;

#[derive(Default)]
struct RaytracingLightCache {
    light_sources: StorageBufferList<GpuLightSource>,
    directional_lights: StorageBufferList<GpuDirectionalLight>,
    previous_frame_light_id_translations: StorageBufferList<u32>,
}

#[derive(Default)]
struct RaytracingTlasCache {
    tlas: Option<Tlas>,
    capacity: u32,
    generation: u64,
}

#[derive(Clone, PartialEq, Eq)]
struct RaytracingSceneKey {
    instances: Vec<RaytracingInstanceKey>,
    directional_lights: Vec<RaytracingDirectionalLightKey>,
    material_generation: u64,
    blas_generation: u64,
}

#[derive(Clone, PartialEq, Eq)]
struct RaytracingInstanceKey {
    entity: Entity,
    mesh: AssetId<Mesh>,
    material: AssetId<StandardMaterial>,
    transform: [u32; 16],
    previous_frame_transform: [u32; 16],
}

#[derive(Clone, PartialEq, Eq)]
struct RaytracingDirectionalLightKey {
    entity: Entity,
    direction_to_light: [u32; 3],
    color: [u32; 4],
    illuminance: u32,
    sun_disk_angular_size: u32,
}

#[derive(Clone, PartialEq, Eq)]
struct RaytracingBindingKey {
    vertex_buffers: Vec<BufferId>,
    index_buffers: Vec<BufferId>,
    textures: Vec<AssetId<Image>>,
    material_capacity: u64,
    instance_capacity: u32,
    light_source_count: u32,
    directional_light_count: u32,
    previous_light_translation_count: u32,
    dfg_view: TextureViewId,
    dfg_sampler: SamplerId,
    tlas_generation: u64,
}

pub fn prepare_raytracing_scene_bindings(
    instances_query: Query<(
        Entity,
        &RaytracingMesh3d,
        &MeshMaterial3d<StandardMaterial>,
        &GlobalTransform,
        Option<&PreviousGlobalTransform>,
    )>,
    directional_lights_query: Query<(Entity, &ExtractedDirectionalLight)>,
    mesh_allocator: Res<MeshAllocator>,
    blas_manager: Res<BlasManager>,
    material_assets: Res<StandardMaterialAssets>,
    texture_assets: Res<RenderAssets<GpuImage>>,
    fallback_texture: Res<FallbackImage>,
    dfg_lut: Res<DfgLut>,
    render_device: Res<RenderDevice>,
    pipeline_cache: Res<PipelineCache>,
    render_queue: Res<RenderQueue>,
    mut raytracing_scene_bindings: ResMut<RaytracingSceneBindings>,
) {
    let scene_key = build_scene_key(
        &instances_query,
        &directional_lights_query,
        material_assets.generation(),
        blas_manager.generation(),
    );
    if raytracing_scene_bindings.bind_group.is_some()
        && raytracing_scene_bindings.last_scene_key.as_ref() == Some(&scene_key)
    {
        return;
    }

    let previous_frame_light_entities = raytracing_scene_bindings
        .previous_frame_light_entities
        .clone();
    let mut next_frame_light_entities = Vec::new();
    let mut this_frame_entity_to_light_id = EntityHashMap::<u32>::default();

    let mut vertex_buffers = CachedBindingArray::new();
    let mut index_buffers = CachedBindingArray::new();
    let mut textures = CachedBindingArray::new();
    let mut samplers = Vec::new();
    let mut binding_key = RaytracingBindingKey {
        vertex_buffers: Vec::new(),
        index_buffers: Vec::new(),
        textures: Vec::new(),
        material_capacity: 0,
        instance_capacity: 0,
        light_source_count: 0,
        directional_light_count: 0,
        previous_light_translation_count: 0,
        dfg_view: fallback_texture.d2.texture_view.id(),
        dfg_sampler: fallback_texture.d2.sampler.id(),
        tlas_generation: raytracing_scene_bindings.tlas_cache.generation,
    };

    let mut material_id_map: HashMap<AssetId<StandardMaterial>, u32, FixedHasher> =
        HashMap::default();
    let mut process_texture = |texture_handle: &Option<Handle<_>>| -> Option<u32> {
        match texture_handle {
            Some(texture_handle) => match texture_assets.get(texture_handle.id()) {
                Some(texture) => {
                    let (texture_id, is_new) =
                        textures.push_if_absent(texture.texture_view.deref(), texture_handle.id());
                    if is_new {
                        samplers.push(texture.sampler.deref());
                        binding_key.textures.push(texture_handle.id());
                    }
                    Some(texture_id)
                }
                None => None,
            },
            None => Some(TEXTURE_MAP_NONE),
        }
    };

    let mut materials = Vec::new();
    for (_, _, material_handle, _, _) in &instances_query {
        let asset_id = material_handle.id();
        if material_id_map.contains_key(&asset_id) {
            continue;
        }

        let Some(material) = material_assets.get(&asset_id) else {
            continue;
        };
        let Some(base_color_texture_id) = process_texture(&material.base_color_texture) else {
            continue;
        };
        let Some(normal_map_texture_id) = process_texture(&material.normal_map_texture) else {
            continue;
        };
        let Some(emissive_texture_id) = process_texture(&material.emissive_texture) else {
            continue;
        };
        let Some(metallic_roughness_texture_id) =
            process_texture(&material.metallic_roughness_texture)
        else {
            continue;
        };

        let material_id = materials.len() as u32;
        materials.push(GpuMaterial {
            normal_map_texture_id,
            base_color_texture_id,
            emissive_texture_id,
            metallic_roughness_texture_id,

            base_color: LinearRgba::from(material.base_color).to_vec3(),
            perceptual_roughness: material.perceptual_roughness,
            emissive: material.emissive.to_vec3(),
            metallic: material.metallic,
            reflectance: material.reflectance,
            _padding: Default::default(),
        });

        material_id_map.insert(asset_id, material_id);
    }

    if materials.is_empty() {
        raytracing_scene_bindings.bind_group = None;
        raytracing_scene_bindings.last_scene_key = Some(scene_key);
        return;
    }

    if textures.is_empty() {
        textures.vec.push(fallback_texture.d2.texture_view.deref());
        samplers.push(fallback_texture.d2.sampler.deref());
    }

    let mut transforms = Vec::new();
    let mut previous_frame_transforms = Vec::new();
    let mut geometry_ids = Vec::new();
    let mut material_ids = Vec::new();
    let mut entity_to_instance_id = EntityHashMap::<u32>::default();
    let max_instances = instances_query.iter().len() as u32;
    let tlas_recreated = raytracing_scene_bindings
        .tlas_cache
        .ensure_capacity(max_instances, &render_device);
    if tlas_recreated {
        binding_key.tlas_generation = raytracing_scene_bindings.tlas_cache.generation;
    }
    if raytracing_scene_bindings.tlas_cache.tlas.is_none() {
        raytracing_scene_bindings.bind_group = None;
        raytracing_scene_bindings.last_scene_key = Some(scene_key);
        return;
    }
    for i in 0..raytracing_scene_bindings.tlas_cache.capacity {
        let tlas = raytracing_scene_bindings.tlas_cache.tlas.as_mut().unwrap();
        *tlas.get_mut_single(i as usize).unwrap() = None;
    }

    for (entity, mesh, material, transform, previous_frame_transform) in &instances_query {
        let Some(blas) = blas_manager.get(&mesh.id()) else {
            continue;
        };
        let Some(vertex_slice) = mesh_allocator.mesh_vertex_slice(&mesh.id()) else {
            continue;
        };
        let Some(index_slice) = mesh_allocator.mesh_index_slice(&mesh.id()) else {
            continue;
        };
        let Some(material_id) = material_id_map.get(&material.id()).copied() else {
            continue;
        };
        let Some(_material) = materials.get(material_id as usize) else {
            continue;
        };

        let transform = transform.to_matrix();
        let instance_id = transforms.len() as u32;
        let tlas = raytracing_scene_bindings.tlas_cache.tlas.as_mut().unwrap();
        *tlas.get_mut_single(instance_id as usize).unwrap() = Some(TlasInstance::new(
            blas,
            tlas_transform(&transform),
            Default::default(),
            0xFF,
        ));

        transforms.push(transform);
        previous_frame_transforms.push(
            previous_frame_transform
                .map(|t| Mat4::from(t.0))
                .unwrap_or(transform),
        );

        let (vertex_buffer_id, _) = vertex_buffers.push_if_absent(
            vertex_slice.buffer.as_entire_buffer_binding(),
            vertex_slice.buffer.id(),
        );
        if !binding_key
            .vertex_buffers
            .contains(&vertex_slice.buffer.id())
        {
            binding_key.vertex_buffers.push(vertex_slice.buffer.id());
        }
        let (index_buffer_id, _) = index_buffers.push_if_absent(
            index_slice.buffer.as_entire_buffer_binding(),
            index_slice.buffer.id(),
        );
        if !binding_key.index_buffers.contains(&index_slice.buffer.id()) {
            binding_key.index_buffers.push(index_slice.buffer.id());
        }

        geometry_ids.push(GpuInstanceGeometryIds {
            vertex_buffer_id,
            vertex_buffer_offset: vertex_slice.range.start,
            index_buffer_id,
            index_buffer_offset: index_slice.range.start,
            triangle_count: (index_slice.range.len() / 3) as u32,
        });

        material_ids.push(material_id);
        entity_to_instance_id.insert(entity, instance_id);
    }

    if transforms.is_empty() {
        raytracing_scene_bindings.bind_group = None;
        raytracing_scene_bindings.last_scene_key = Some(scene_key);
        return;
    }

    let mut light_sources = Vec::new();
    for (entity, mesh, material, _, _) in &instances_query {
        let Some(material_id) = material_id_map.get(&material.id()).copied() else {
            continue;
        };
        let Some(material) = materials.get(material_id as usize) else {
            continue;
        };
        if material.emissive == Vec3::ZERO {
            continue;
        }
        let Some(index_slice) = mesh_allocator.mesh_index_slice(&mesh.id()) else {
            continue;
        };
        let Some(instance_id) = entity_to_instance_id.get(&entity).copied() else {
            continue;
        };
        light_sources.push(GpuLightSource::new_emissive_mesh_light(
            instance_id,
            (index_slice.range.len() / 3) as u32,
        ));
        this_frame_entity_to_light_id.insert(entity, light_sources.len() as u32 - 1);
        next_frame_light_entities.push(entity);
    }

    let mut directional_lights = Vec::new();
    for (entity, directional_light) in &directional_lights_query {
        let directional_light_id = directional_lights.len() as u32;

        directional_lights.push(GpuDirectionalLight::new(directional_light));

        light_sources.push(GpuLightSource::new_directional_light(directional_light_id));

        this_frame_entity_to_light_id.insert(entity, light_sources.len() as u32 - 1);
        next_frame_light_entities.push(entity);
    }

    let mut previous_frame_light_id_translations = Vec::new();
    for previous_frame_light_entity in previous_frame_light_entities {
        let current_frame_index = this_frame_entity_to_light_id
            .get(&previous_frame_light_entity)
            .copied()
            .unwrap_or(LIGHT_NOT_PRESENT_THIS_FRAME);
        previous_frame_light_id_translations.push(current_frame_index);
    }

    if light_sources.len() > u16::MAX as usize {
        panic!("Too many light sources in the scene, maximum is 65535.");
    }

    binding_key.material_capacity = materials.len() as u64;
    binding_key.instance_capacity = transforms.len() as u32;
    binding_key.light_source_count = light_sources.len() as u32;
    binding_key.directional_light_count = directional_lights.len() as u32;
    binding_key.previous_light_translation_count =
        previous_frame_light_id_translations.len() as u32;

    raytracing_scene_bindings
        .material_cache
        .materials
        .set(materials);
    raytracing_scene_bindings
        .instance_cache
        .transforms
        .set(transforms);
    raytracing_scene_bindings
        .instance_cache
        .previous_frame_transforms
        .set(previous_frame_transforms);
    raytracing_scene_bindings
        .instance_cache
        .geometry_ids
        .set(geometry_ids);
    raytracing_scene_bindings
        .instance_cache
        .material_ids
        .set(material_ids);
    raytracing_scene_bindings
        .light_cache
        .light_sources
        .set(light_sources);
    raytracing_scene_bindings
        .light_cache
        .directional_lights
        .set(directional_lights);
    raytracing_scene_bindings
        .light_cache
        .previous_frame_light_id_translations
        .set(previous_frame_light_id_translations);

    raytracing_scene_bindings
        .material_cache
        .materials
        .write_buffer(&render_device, &render_queue);
    raytracing_scene_bindings
        .instance_cache
        .transforms
        .write_buffer(&render_device, &render_queue);
    raytracing_scene_bindings
        .instance_cache
        .previous_frame_transforms
        .write_buffer(&render_device, &render_queue);
    raytracing_scene_bindings
        .instance_cache
        .geometry_ids
        .write_buffer(&render_device, &render_queue);
    raytracing_scene_bindings
        .instance_cache
        .material_ids
        .write_buffer(&render_device, &render_queue);
    raytracing_scene_bindings
        .light_cache
        .light_sources
        .write_buffer(&render_device, &render_queue);
    raytracing_scene_bindings
        .light_cache
        .directional_lights
        .write_buffer(&render_device, &render_queue);
    raytracing_scene_bindings
        .light_cache
        .previous_frame_light_id_translations
        .write_buffer(&render_device, &render_queue);

    let mut command_encoder = render_device.create_command_encoder(&CommandEncoderDescriptor {
        label: Some("build_tlas_command_encoder"),
    });
    let tlas = raytracing_scene_bindings.tlas_cache.tlas.as_ref().unwrap();
    command_encoder.build_acceleration_structures(&[], [tlas]);
    render_queue.submit([command_encoder.finish()]);

    let (dfg_view, dfg_sampler) = texture_assets
        .get(&dfg_lut.texture)
        .map(|img| (&img.texture_view, &img.sampler))
        .unwrap_or((
            &fallback_texture.d2.texture_view,
            &fallback_texture.d2.sampler,
        ));
    binding_key.dfg_view = dfg_view.id();
    binding_key.dfg_sampler = dfg_sampler.id();

    let recreate_bind_group = raytracing_scene_bindings.bind_group.is_none()
        || raytracing_scene_bindings.last_binding_key.as_ref() != Some(&binding_key);

    if recreate_bind_group {
        let tlas = raytracing_scene_bindings.tlas_cache.tlas.as_ref().unwrap();
        raytracing_scene_bindings.bind_group = Some(
            render_device.create_bind_group(
                "raytracing_scene_bind_group",
                &pipeline_cache.get_bind_group_layout(&raytracing_scene_bindings.bind_group_layout),
                &BindGroupEntries::sequential((
                    vertex_buffers.as_slice(),
                    index_buffers.as_slice(),
                    textures.as_slice(),
                    samplers.as_slice(),
                    raytracing_scene_bindings
                        .material_cache
                        .materials
                        .binding()
                        .unwrap(),
                    tlas.as_binding(),
                    raytracing_scene_bindings
                        .instance_cache
                        .transforms
                        .binding()
                        .unwrap(),
                    raytracing_scene_bindings
                        .instance_cache
                        .previous_frame_transforms
                        .binding()
                        .unwrap(),
                    raytracing_scene_bindings
                        .instance_cache
                        .geometry_ids
                        .binding()
                        .unwrap(),
                    raytracing_scene_bindings
                        .instance_cache
                        .material_ids
                        .binding()
                        .unwrap(),
                    raytracing_scene_bindings
                        .light_cache
                        .light_sources
                        .binding()
                        .unwrap(),
                    raytracing_scene_bindings
                        .light_cache
                        .directional_lights
                        .binding()
                        .unwrap(),
                    raytracing_scene_bindings
                        .light_cache
                        .previous_frame_light_id_translations
                        .binding()
                        .unwrap(),
                    dfg_view,
                    dfg_sampler,
                )),
            ),
        );
        raytracing_scene_bindings.last_binding_key = Some(binding_key);
    }

    raytracing_scene_bindings.previous_frame_light_entities = next_frame_light_entities;
    raytracing_scene_bindings.last_scene_key = Some(scene_key);
}

impl RaytracingSceneBindings {
    pub(crate) fn light_source_count(&self) -> u32 {
        self.light_cache.light_sources.get().len() as u32
    }

    pub fn new() -> Self {
        Self {
            bind_group: None,
            bind_group_layout: BindGroupLayoutDescriptor::new(
                "raytracing_scene_bind_group_layout",
                &BindGroupLayoutEntries::sequential(
                    ShaderStages::COMPUTE,
                    (
                        storage_buffer_read_only_sized(false, None).count(MAX_MESH_SLAB_COUNT),
                        storage_buffer_read_only_sized(false, None).count(MAX_MESH_SLAB_COUNT),
                        texture_2d(TextureSampleType::Float { filterable: true })
                            .count(MAX_TEXTURE_COUNT),
                        sampler(SamplerBindingType::Filtering).count(MAX_TEXTURE_COUNT),
                        storage_buffer_read_only_sized(false, None),
                        acceleration_structure(),
                        storage_buffer_read_only_sized(false, None),
                        storage_buffer_read_only_sized(false, None),
                        storage_buffer_read_only_sized(false, None),
                        storage_buffer_read_only_sized(false, None),
                        storage_buffer_read_only_sized(false, None),
                        storage_buffer_read_only_sized(false, None),
                        storage_buffer_read_only_sized(false, None),
                        texture_2d(TextureSampleType::Float { filterable: true }),
                        sampler(SamplerBindingType::Filtering),
                    ),
                ),
            ),
            previous_frame_light_entities: Vec::new(),
            last_scene_key: None,
            last_binding_key: None,
            instance_cache: RaytracingInstanceCache::default(),
            material_cache: RaytracingMaterialCache::default(),
            _texture_cache: RaytracingTextureCache,
            light_cache: RaytracingLightCache::default(),
            tlas_cache: RaytracingTlasCache::default(),
        }
    }
}

impl Default for RaytracingSceneBindings {
    fn default() -> Self {
        Self::new()
    }
}

impl RaytracingTlasCache {
    fn ensure_capacity(&mut self, max_instances: u32, render_device: &RenderDevice) -> bool {
        if max_instances == 0 {
            let had_tlas = self.tlas.take().is_some();
            self.capacity = 0;
            if had_tlas {
                self.generation = self.generation.wrapping_add(1);
            }
            return had_tlas;
        }

        if self.tlas.is_none() || self.capacity != max_instances {
            self.tlas = Some(
                render_device
                    .wgpu_device()
                    .create_tlas(&CreateTlasDescriptor {
                        label: Some("tlas"),
                        flags: AccelerationStructureFlags::PREFER_FAST_TRACE,
                        update_mode: AccelerationStructureUpdateMode::Build,
                        max_instances,
                    }),
            );
            self.capacity = max_instances;
            self.generation = self.generation.wrapping_add(1);
            return true;
        }

        false
    }
}

fn build_scene_key(
    instances_query: &Query<(
        Entity,
        &RaytracingMesh3d,
        &MeshMaterial3d<StandardMaterial>,
        &GlobalTransform,
        Option<&PreviousGlobalTransform>,
    )>,
    directional_lights_query: &Query<(Entity, &ExtractedDirectionalLight)>,
    material_generation: u64,
    blas_generation: u64,
) -> RaytracingSceneKey {
    let mut instances = instances_query
        .iter()
        .map(
            |(entity, mesh, material, transform, previous_frame_transform)| {
                let transform = transform.to_matrix();
                RaytracingInstanceKey {
                    entity,
                    mesh: mesh.id(),
                    material: material.id(),
                    transform: mat4_key(transform),
                    previous_frame_transform: mat4_key(
                        previous_frame_transform
                            .map(|t| Mat4::from(t.0))
                            .unwrap_or(transform),
                    ),
                }
            },
        )
        .collect::<Vec<_>>();
    instances.sort_by_key(|key| key.entity);

    let mut directional_lights = directional_lights_query
        .iter()
        .map(|(entity, light)| RaytracingDirectionalLightKey {
            entity,
            direction_to_light: vec3_key(light.transform.back().into()),
            color: light.color.to_vec4().to_array().map(f32::to_bits),
            illuminance: light.illuminance.to_bits(),
            sun_disk_angular_size: light.sun_disk_angular_size.to_bits(),
        })
        .collect::<Vec<_>>();
    directional_lights.sort_by_key(|key| key.entity);

    RaytracingSceneKey {
        instances,
        directional_lights,
        material_generation,
        blas_generation,
    }
}

fn mat4_key(matrix: Mat4) -> [u32; 16] {
    matrix.to_cols_array().map(f32::to_bits)
}

fn vec3_key(vector: Vec3) -> [u32; 3] {
    vector.to_array().map(f32::to_bits)
}

struct CachedBindingArray<T, I: Eq + Hash> {
    map: HashMap<I, u32>,
    vec: Vec<T>,
}

impl<T, I: Eq + Hash> CachedBindingArray<T, I> {
    fn new() -> Self {
        Self {
            map: HashMap::default(),
            vec: Vec::default(),
        }
    }

    fn push_if_absent(&mut self, item: T, item_id: I) -> (u32, bool) {
        let mut is_new = false;
        let i = *self.map.entry(item_id).or_insert_with(|| {
            is_new = true;
            let i = self.vec.len() as u32;
            self.vec.push(item);
            i
        });
        (i, is_new)
    }

    fn is_empty(&self) -> bool {
        self.vec.is_empty()
    }

    fn as_slice(&self) -> &[T] {
        self.vec.as_slice()
    }
}

type StorageBufferList<T> = StorageBuffer<Vec<T>>;

#[derive(ShaderType)]
struct GpuInstanceGeometryIds {
    vertex_buffer_id: u32,
    vertex_buffer_offset: u32,
    index_buffer_id: u32,
    index_buffer_offset: u32,
    triangle_count: u32,
}

#[derive(ShaderType)]
struct GpuMaterial {
    normal_map_texture_id: u32,
    base_color_texture_id: u32,
    emissive_texture_id: u32,
    metallic_roughness_texture_id: u32,

    base_color: Vec3,
    perceptual_roughness: f32,
    emissive: Vec3,
    metallic: f32,
    _padding: Vec3,
    reflectance: f32,
}

#[derive(ShaderType)]
struct GpuLightSource {
    kind: u32,
    id: u32,
}

impl GpuLightSource {
    fn new_emissive_mesh_light(instance_id: u32, triangle_count: u32) -> GpuLightSource {
        if triangle_count > u16::MAX as u32 {
            panic!("Too many triangles ({triangle_count}) in an emissive mesh, maximum is 65535.");
        }

        Self {
            kind: triangle_count << 1,
            id: instance_id,
        }
    }

    fn new_directional_light(directional_light_id: u32) -> GpuLightSource {
        Self {
            kind: 1,
            id: directional_light_id,
        }
    }
}

#[derive(ShaderType, Default)]
struct GpuDirectionalLight {
    direction_to_light: Vec3,
    cos_theta_max: f32,
    luminance: Vec3,
    inverse_pdf: f32,
}

impl GpuDirectionalLight {
    fn new(directional_light: &ExtractedDirectionalLight) -> Self {
        let cos_theta_max = cos(directional_light.sun_disk_angular_size / 2.0);
        let solid_angle = TAU * (1.0 - cos_theta_max);
        let luminance =
            (directional_light.color.to_vec3() * directional_light.illuminance) / solid_angle;

        Self {
            direction_to_light: directional_light.transform.back().into(),
            cos_theta_max,
            luminance,
            inverse_pdf: solid_angle,
        }
    }
}

fn tlas_transform(transform: &Mat4) -> [f32; 12] {
    transform.transpose().to_cols_array()[..12]
        .try_into()
        .unwrap()
}
