enable wgpu_ray_query;

#define_import_path bevy_solari::world_cache

#import bevy_pbr::utils::{rand_f, rand_vec2f}
#import bevy_render::maths::orthonormalize
#import bevy_solari::solari_debug::solari_debug_count_world_cache_probe_wrap
#import bevy_solari::realtime_bindings::{
    world_cache_life,
    world_cache_checksums,
    world_cache_radiance,
    world_cache_geometry_data,
    world_cache_luminance_deltas,
    world_cache_a,
    world_cache_active_cell_indices,
    world_cache_active_cells_count,
    constants,
    WorldCacheGeometryData,
}

/// How responsive the world cache is to changes in lighting (higher is less responsive but more stable, lower is more responsive but less stable)
const WORLD_CACHE_MAX_TEMPORAL_SAMPLES: f32 = 32.0;
/// How many direct light samples each cell takes when updating each frame
const WORLD_CACHE_DIRECT_LIGHT_SAMPLE_COUNT: u32 = 32u;
/// Maximum amount of distance to trace GI rays between two cache cells
const WORLD_CACHE_MAX_GI_RAY_DISTANCE: f32 = 50.0;
/// Soft upper limit on the amount of cache cells to update each frame
const WORLD_CACHE_CELL_UPDATES_SOFT_CAP: u32 = 40000u;

/// Maximum amount of frames a cell can live for without being queried
const WORLD_CACHE_CELL_LIFETIME: u32 = 10u;
/// Maximum amount of attempts to find a cache entry after a hash collision
const WORLD_CACHE_MAX_SEARCH_STEPS: u32 = 3u;

/// Size of a cache cell at the lowest LOD in meters
const WORLD_CACHE_POSITION_BASE_CELL_SIZE: f32 = 0.15;
/// How fast the world cache transitions between LODs as a function of distance to the camera
const WORLD_CACHE_POSITION_LOD_SCALE: f32 = 15.0;

/// Marker value for an empty cell
const WORLD_CACHE_EMPTY_CELL: u32 = 0u;

fn query_world_cache(world_position_in: vec3<f32>, world_normal: vec3<f32>, view_position: vec3<f32>, ray_t: f32, cell_lifetime: u32, append_active: bool, rng: ptr<function, u32>) -> vec3<f32> {
    var world_position = world_position_in;
    var cell_size = get_cell_size(world_position, view_position, rng);

#ifdef WORLD_CACHE_FIRST_BOUNCE_LIGHT_LEAK_PREVENTION
    if ray_t < cell_size {
        // Prevent light leaks
        cell_size = WORLD_CACHE_POSITION_BASE_CELL_SIZE;
    }
#endif

#ifndef NO_JITTER_WORLD_CACHE
    // Jitter query point, which essentially blurs the cache a bit so it's not so grid-like
    // https://tomclabault.github.io/blog/2025/regir, jitter_world_position_tangent_plane
    let TBN = orthonormalize(world_normal);
    let offset = (rand_vec2f(rng) * 2.0 - 1.0) * cell_size * 0.5;
    world_position += offset.x * TBN[0] + offset.y * TBN[1];
    cell_size = get_cell_size(world_position, view_position, rng);
#endif

    let world_position_quantized = bitcast<vec3<u32>>(quantize_position(world_position, cell_size));
    let world_normal_quantized = bitcast<vec3<u32>>(quantize_normal(world_normal));
    var key = compute_key(world_position_quantized, world_normal_quantized);
    let checksum = compute_checksum(world_position_quantized, world_normal_quantized);

    for (var i = 0u; i < WORLD_CACHE_MAX_SEARCH_STEPS; i++) {
        let r = atomicCompareExchangeWeak(&world_cache_checksums[key], WORLD_CACHE_EMPTY_CELL, checksum);

        if r.old_value == checksum {
            if expire_world_cache_cell_if_stale(key, checksum, cell_lifetime) {
                continue;
            }

            // Cache entry already exists - get radiance
            refresh_world_cache_lifetime(key, append_active);
            return world_cache_radiance[key].rgb;
        }

        if r.old_value == WORLD_CACHE_EMPTY_CELL {
            if r.exchanged {
                // Cell is empty and this invocation claimed it.
                world_cache_geometry_data[key].world_position = world_position;
                world_cache_geometry_data[key].world_normal = world_normal;
                world_cache_radiance[key] = vec4(0.0);
                world_cache_luminance_deltas[key] = 0.0;
                refresh_world_cache_lifetime(key, append_active);
                return vec3(0.0);
            }

            // Weak CAS can fail spuriously; retry the same key within the bounded search budget.
            continue;
        }

        if expire_world_cache_cell_if_stale(key, r.old_value, cell_lifetime) {
            continue;
        }

        // Collision - linear probe to next entry, wrapping inside the table.
        let next_key = wrap_key(key + 1u);
        if next_key < key {
            solari_debug_count_world_cache_probe_wrap();
        }
        key = next_key;
    }

    return vec3(0.0);
}

fn refresh_world_cache_lifetime(key: u32, append_active: bool) {
    let epoch = world_cache_frame_epoch();
    atomicStore(&world_cache_life[key], epoch);
    if !append_active {
        return;
    }

    let previous_epoch = atomicExchange(&world_cache_a[key], epoch);
    if previous_epoch != epoch {
        let active_index = atomicAdd(&world_cache_active_cells_count, 1u);
        if active_index < #{WORLD_CACHE_SIZE} {
            world_cache_active_cell_indices[active_index] = key;
        }
    }
}

fn expire_world_cache_cell_if_stale(key: u32, checksum: u32, cell_lifetime: u32) -> bool {
    let last_seen_frame = atomicLoad(&world_cache_life[key]);
    if last_seen_frame != 0u && world_cache_frame_epoch() - last_seen_frame <= cell_lifetime {
        return false;
    }

    let r = atomicCompareExchangeWeak(&world_cache_checksums[key], checksum, WORLD_CACHE_EMPTY_CELL);
    if r.old_value == checksum && r.exchanged {
        atomicStore(&world_cache_life[key], 0u);
        atomicStore(&world_cache_a[key], 0u);
        world_cache_radiance[key] = vec4(0.0);
        world_cache_luminance_deltas[key] = 0.0;
    }
    return r.old_value == checksum;
}

fn world_cache_frame_epoch() -> u32 {
    return constants.frame_number + 1u;
}

fn get_cell_size(world_position: vec3<f32>, view_position: vec3<f32>, rng: ptr<function, u32>) -> f32 {
    let camera_distance = distance(view_position, world_position) / WORLD_CACHE_POSITION_LOD_SCALE;
    let lod_f = log2(1.0 + camera_distance);
    let lod_fract = fract(lod_f);
    let lod = floor(lod_f) + select(0.0, 1.0, rand_f(rng) < lod_fract * lod_fract * lod_fract);
    return WORLD_CACHE_POSITION_BASE_CELL_SIZE * exp2(lod);
}

fn quantize_position(world_position: vec3<f32>, quantization_factor: f32) -> vec3<f32> {
    return floor(world_position / quantization_factor + 0.0001);
}

fn quantize_normal(world_normal: vec3<f32>) -> vec3<f32> {
    return floor(world_normal * 2.0 + 0.0001);
}

// TODO: Clustering
fn compute_key(world_position: vec3<u32>, world_normal: vec3<u32>) -> u32 {
    var key = pcg_hash(world_position.x);
    key = pcg_hash(key + world_position.y);
    key = pcg_hash(key + world_position.z);
    key = pcg_hash(key + world_normal.x);
    key = pcg_hash(key + world_normal.y);
    key = pcg_hash(key + world_normal.z);
    return wrap_key(key);
}

fn compute_checksum(world_position: vec3<u32>, world_normal: vec3<u32>) -> u32 {
    var key = iqint_hash(world_position.x);
    key = iqint_hash(key + world_position.y);
    key = iqint_hash(key + world_position.z);
    key = iqint_hash(key + world_normal.x);
    key = iqint_hash(key + world_normal.y);
    key = iqint_hash(key + world_normal.z);
    return max(key, 1u); // 0u is reserved for WORLD_CACHE_EMPTY_CELL
}

fn pcg_hash(input: u32) -> u32 {
    let state = input * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

fn iqint_hash(input: u32) -> u32 {
    let n = (input << 13u) ^ input;
    return n * (n * n * 15731u + 789221u) + 1376312589u;
}

fn wrap_key(key: u32) -> u32 {
    return key & (#{WORLD_CACHE_SIZE} - 1u);
}
