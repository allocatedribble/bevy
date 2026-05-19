enable wgpu_ray_query;

#define_import_path bevy_solari::scene_bindings

#import bevy_pbr::lighting::perceptualRoughnessToRoughness
#import bevy_pbr::pbr_functions::calculate_tbn_mikktspace

struct InstanceGeometryIds {
    vertex_buffer_id: u32,
    vertex_buffer_offset: u32,
    index_buffer_id: u32,
    index_buffer_offset: u32,
    triangle_count: u32,
}

struct VertexBuffer { vertices: array<PackedVertex> }

struct IndexBuffer { indices: array<u32> }

struct PackedVertex {
    a: vec4<f32>,
    b: vec4<f32>,
    tangent: vec4<f32>,
}

struct Vertex {
    position: vec3<f32>,
    normal: vec3<f32>,
    uv: vec2<f32>,
    tangent: vec4<f32>,
}

fn unpack_vertex(packed: PackedVertex) -> Vertex {
    var vertex: Vertex;
    vertex.position = packed.a.xyz;
    vertex.normal = vec3(packed.a.w, packed.b.xy);
    vertex.uv = packed.b.zw;
    vertex.tangent = packed.tangent;
    return vertex;
}

struct Material {
    normal_map_texture_id: u32,
    base_color_texture_id: u32,
    emissive_texture_id: u32,
    metallic_roughness_texture_id: u32,

    base_color: vec3<f32>,
    perceptual_roughness: f32,
    emissive: vec3<f32>,
    metallic: f32,
    _padding: vec3<f32>,
    reflectance: f32,
}

const TEXTURE_MAP_NONE = 0xFFFFFFFFu;

const MIRROR_ROUGHNESS_THRESHOLD = 0.001f;

struct LightSource {
    kind: u32, // 1 bit for kind, 31 bits for extra data
    id: u32,
}

const LIGHT_SOURCE_KIND_EMISSIVE_MESH = 0u;
const LIGHT_SOURCE_KIND_DIRECTIONAL = 1u;

struct DirectionalLight {
    direction_to_light: vec3<f32>,
    cos_theta_max: f32,
    luminance: vec3<f32>,
    inverse_pdf: f32,
}

const LIGHT_NOT_PRESENT_THIS_FRAME = 0xFFFFFFFFu;

@group(0) @binding(0) var<storage> vertex_buffers: binding_array<VertexBuffer>;
@group(0) @binding(1) var<storage> index_buffers: binding_array<IndexBuffer>;
@group(0) @binding(2) var textures: binding_array<texture_2d<f32>>;
@group(0) @binding(3) var samplers: binding_array<sampler>;
@group(0) @binding(4) var<storage> materials: array<Material>;
@group(0) @binding(5) var tlas: acceleration_structure;
@group(0) @binding(6) var<storage> transforms: array<mat4x4<f32>>; // TODO: Use mat3x4<f32>?
@group(0) @binding(7) var<storage> previous_frame_transforms: array<mat4x4<f32>>; // TODO: Use mat3x4<f32>?
@group(0) @binding(8) var<storage> geometry_ids: array<InstanceGeometryIds>;
@group(0) @binding(9) var<storage> material_ids: array<u32>; // TODO: Store material_id in instance_custom_index instead?
@group(0) @binding(10) var<storage> light_sources: array<LightSource>;
@group(0) @binding(11) var<storage> directional_lights: array<DirectionalLight>;
@group(0) @binding(12) var<storage> previous_frame_light_id_translations: array<u32>;
@group(0) @binding(13) var brdf_dfg_lut: texture_2d<f32>;
@group(0) @binding(14) var brdf_dfg_lut_sampler: sampler;

const RAY_T_MIN = 0.001f;
const RAY_T_MAX = 100000.0f;

const RAY_NO_CULL = 0xFFu;

fn trace_ray(ray_origin: vec3<f32>, ray_direction: vec3<f32>, ray_t_min: f32, ray_t_max: f32, ray_flag: u32) -> RayIntersection {
    let ray = RayDesc(ray_flag, RAY_NO_CULL, ray_t_min, ray_t_max, ray_origin, ray_direction);
    var rq: ray_query;
    rayQueryInitialize(&rq, tlas, ray);
    rayQueryProceed(&rq);
    return rayQueryGetCommittedIntersection(&rq);
}

fn sample_texture(id: u32, uv: vec2<f32>) -> vec3<f32> {
    return sample_texture_lod(id, uv, 0.0);
}

fn sample_texture_lod(id: u32, uv: vec2<f32>, lod: f32) -> vec3<f32> {
    let max_lod = f32(textureNumLevels(textures[id]) - 1u);
    return textureSampleLevel(textures[id], samplers[id], uv, clamp(lod, 0.0, max_lod)).rgb;
}

fn ray_hit_texture_lod(ray_t: f32, perceptual_roughness: f32) -> f32 {
    let distance_lod = max(log2(max(ray_t, 1.0)) - 4.0, 0.0);
    let roughness_lod = perceptual_roughness * perceptual_roughness * 4.0;
    return distance_lod + roughness_lod;
}

fn safe_normalize_or_zero(v: vec3<f32>) -> vec3<f32> {
    let len2 = dot(v, v);
    if !(len2 > 0.0) || (bitcast<u32>(len2) & 0x7fffffffu) >= 0x7f800000u {
        return vec3(0.0);
    }
    return v * inverseSqrt(len2);
}

struct ResolvedMaterial {
    base_color: vec3<f32>,
    emissive: vec3<f32>,
    reflectance: f32,
    perceptual_roughness: f32,
    roughness: f32,
    metallic: f32,
}

struct ResolvedRayHitFull {
    world_position: vec3<f32>,
    previous_frame_world_position: vec3<f32>,
    world_normal: vec3<f32>,
    geometric_world_normal: vec3<f32>,
    world_tangent: vec4<f32>,
    uv: vec2<f32>,
    triangle_area: f32,
    triangle_count: u32,
    material: ResolvedMaterial,
}

fn resolve_material(material: Material, uv: vec2<f32>, mip_lod: f32) -> ResolvedMaterial {
    var m: ResolvedMaterial;

    m.base_color = material.base_color.rgb;
    if material.base_color_texture_id != TEXTURE_MAP_NONE {
        m.base_color *= sample_texture_lod(material.base_color_texture_id, uv, mip_lod);
    }

    m.emissive = material.emissive.rgb;
    if material.emissive_texture_id != TEXTURE_MAP_NONE {
        m.emissive *= sample_texture_lod(material.emissive_texture_id, uv, mip_lod);
    }

    m.reflectance = material.reflectance;

    m.perceptual_roughness = material.perceptual_roughness;
    m.metallic = material.metallic;
    if material.metallic_roughness_texture_id != TEXTURE_MAP_NONE {
        let metallic_roughness = sample_texture_lod(material.metallic_roughness_texture_id, uv, mip_lod);
        m.perceptual_roughness *= metallic_roughness.g;
        m.metallic *= metallic_roughness.b;
    }

    m.roughness = m.perceptual_roughness * m.perceptual_roughness;

    return m;
}

fn resolve_ray_hit_full(ray_hit: RayIntersection) -> ResolvedRayHitFull {
    let barycentrics = vec3(1.0 - ray_hit.barycentrics.x - ray_hit.barycentrics.y, ray_hit.barycentrics);
    return resolve_triangle_data_full(ray_hit.instance_index, ray_hit.primitive_index, barycentrics, ray_hit.t);
}

fn load_vertices(instance_geometry_ids: InstanceGeometryIds, triangle_id: u32) -> array<Vertex, 3> {
    let index_buffer = &index_buffers[instance_geometry_ids.index_buffer_id].indices;
    let vertex_buffer = &vertex_buffers[instance_geometry_ids.vertex_buffer_id].vertices;

    let indices_i = (triangle_id * 3u) + vec3(0u, 1u, 2u) + instance_geometry_ids.index_buffer_offset;
    let indices = vec3((*index_buffer)[indices_i.x], (*index_buffer)[indices_i.y], (*index_buffer)[indices_i.z]) + instance_geometry_ids.vertex_buffer_offset;

    return array<Vertex, 3>(
        unpack_vertex((*vertex_buffer)[indices.x]),
        unpack_vertex((*vertex_buffer)[indices.y]),
        unpack_vertex((*vertex_buffer)[indices.z])
    );
}

fn transform_positions(transform: mat4x4<f32>, vertices: array<Vertex, 3>) -> array<vec3<f32>, 3> {
    return array<vec3<f32>, 3>(
        (transform * vec4(vertices[0].position, 1.0)).xyz,
        (transform * vec4(vertices[1].position, 1.0)).xyz,
        (transform * vec4(vertices[2].position, 1.0)).xyz
    );
}

fn resolve_triangle_data_full(instance_id: u32, triangle_id: u32, barycentrics: vec3<f32>, ray_t: f32) -> ResolvedRayHitFull {
    let material_id = material_ids[instance_id];
    let material = materials[material_id];

    let transform = transforms[instance_id];
    let previous_frame_transform = previous_frame_transforms[instance_id];

    let instance_geometry_ids = geometry_ids[instance_id];
    let vertices = load_vertices(instance_geometry_ids, triangle_id);

    let world_vertices = transform_positions(transform, vertices);
    let world_position = mat3x3(world_vertices[0], world_vertices[1], world_vertices[2]) * barycentrics;

    let previous_frame_world_vertices = transform_positions(previous_frame_transform, vertices);
    let previous_frame_world_position = mat3x3(previous_frame_world_vertices[0], previous_frame_world_vertices[1], previous_frame_world_vertices[2]) * barycentrics;

    let uv = mat3x2(vertices[0].uv, vertices[1].uv, vertices[2].uv) * barycentrics;

    let local_tangent = mat3x3(vertices[0].tangent.xyz, vertices[1].tangent.xyz, vertices[2].tangent.xyz) * barycentrics;
    let world_tangent = vec4(
        normalize(mat3x3(transform[0].xyz, transform[1].xyz, transform[2].xyz) * local_tangent),
        vertices[0].tangent.w,
    );

    let local_normal = mat3x3(vertices[0].normal, vertices[1].normal, vertices[2].normal) * barycentrics;
    var world_normal = safe_normalize_or_zero(mat3x3(transform[0].xyz, transform[1].xyz, transform[2].xyz) * local_normal);

    let e0 = world_vertices[1] - world_vertices[0];
    let e1 = world_vertices[2] - world_vertices[0];
    var geometric_world_normal = safe_normalize_or_zero(cross(e0, e1));
    if all(geometric_world_normal == vec3(0.0)) {
        geometric_world_normal = world_normal;
    } else if dot(geometric_world_normal, world_normal) < 0.0 {
        geometric_world_normal = -geometric_world_normal;
    }

    if material.normal_map_texture_id != TEXTURE_MAP_NONE {
        let TBN = calculate_tbn_mikktspace(world_normal, world_tangent);
        let T = TBN[0];
        let B = TBN[1];
        let N = TBN[2];
        let normal_mip_lod = ray_hit_texture_lod(ray_t, material.perceptual_roughness);
        let Nt = safe_normalize_or_zero(sample_texture_lod(material.normal_map_texture_id, uv, normal_mip_lod) * 2.0 - 1.0);
        if any(Nt != vec3(0.0)) {
            world_normal = safe_normalize_or_zero(Nt.x * T + Nt.y * B + Nt.z * N);
        }
    }

    let triangle_area = length(cross(e0, e1)) / 2.0;

    let material_mip_lod = ray_hit_texture_lod(ray_t, material.perceptual_roughness);
    let resolved_material = resolve_material(material, uv, material_mip_lod);

    return ResolvedRayHitFull(
        world_position,
        previous_frame_world_position,
        world_normal,
        geometric_world_normal,
        world_tangent,
        uv,
        triangle_area,
        instance_geometry_ids.triangle_count,
        resolved_material,
    );
}
