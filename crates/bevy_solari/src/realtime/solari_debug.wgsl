#define_import_path bevy_solari::solari_debug

const SOLARI_DEBUG_VALIDATE_RESERVOIRS = 1u << 0u;
const SOLARI_DEBUG_VALIDATE_LIGHT_IDS = 1u << 1u;
const SOLARI_DEBUG_VALIDATE_NAN_INF = 1u << 2u;
const SOLARI_DEBUG_VISUALIZE_TEMPORAL_SOURCES = 1u << 3u;
const SOLARI_DEBUG_VISUALIZE_WORLD_CACHE = 1u << 4u;

struct SolariDebugMode {
    flags: u32,
    _padding_a: u32,
    _padding_b: u32,
    _padding_c: u32,
}

struct SolariDebugCounters {
    nan_radiance_count: atomic<u32>,
    inf_radiance_count: atomic<u32>,
    invalid_di_temporal_count: atomic<u32>,
    invalid_gi_reuse_count: atomic<u32>,
    oob_light_translation_count: atomic<u32>,
    world_cache_probe_wrap_count: atomic<u32>,
    zero_distance_visibility_count: atomic<u32>,
    empty_light_table_sample_count: atomic<u32>,
}

@group(1) @binding(24) var<uniform> solari_debug_mode: SolariDebugMode;
@group(1) @binding(25) var<storage, read_write> solari_debug_counters: SolariDebugCounters;

fn solari_debug_enabled(flag: u32) -> bool {
    return (solari_debug_mode.flags & flag) != 0u;
}

fn solari_debug_validate_reservoirs() -> bool {
    return solari_debug_enabled(SOLARI_DEBUG_VALIDATE_RESERVOIRS);
}

fn solari_debug_validate_light_ids() -> bool {
    return solari_debug_enabled(SOLARI_DEBUG_VALIDATE_LIGHT_IDS);
}

fn solari_debug_validate_nan_inf() -> bool {
    return solari_debug_enabled(SOLARI_DEBUG_VALIDATE_NAN_INF);
}

fn solari_debug_visualize_temporal_sources() -> bool {
    return solari_debug_enabled(SOLARI_DEBUG_VISUALIZE_TEMPORAL_SOURCES);
}

fn solari_debug_visualize_world_cache() -> bool {
    return solari_debug_enabled(SOLARI_DEBUG_VISUALIZE_WORLD_CACHE);
}

fn solari_debug_is_nan(x: f32) -> bool {
    return (bitcast<u32>(x) & 0x7fffffffu) > 0x7f800000u;
}

fn solari_debug_is_inf(x: f32) -> bool {
    return (bitcast<u32>(x) & 0x7fffffffu) == 0x7f800000u;
}

fn solari_debug_vec_has_nan(v: vec3<f32>) -> bool {
    let bits = bitcast<vec3<u32>>(v) & vec3(0x7fffffffu);
    return any(bits > vec3(0x7f800000u));
}

fn solari_debug_vec_has_inf(v: vec3<f32>) -> bool {
    let bits = bitcast<vec3<u32>>(v) & vec3(0x7fffffffu);
    return any(bits == vec3(0x7f800000u));
}

fn solari_debug_validate_radiance(radiance: vec3<f32>) -> vec3<f32> {
    if solari_debug_validate_nan_inf() {
        if solari_debug_vec_has_nan(radiance) {
            atomicAdd(&solari_debug_counters.nan_radiance_count, 1u);
            return vec3(0.0);
        }
        if solari_debug_vec_has_inf(radiance) {
            atomicAdd(&solari_debug_counters.inf_radiance_count, 1u);
            return vec3(0.0);
        }
    }
    return radiance;
}

fn solari_debug_count_invalid_di_temporal() {
    if solari_debug_validate_reservoirs() {
        atomicAdd(&solari_debug_counters.invalid_di_temporal_count, 1u);
    }
}

fn solari_debug_count_invalid_gi_reuse() {
    if solari_debug_validate_reservoirs() {
        atomicAdd(&solari_debug_counters.invalid_gi_reuse_count, 1u);
    }
}

fn solari_debug_count_oob_light_translation() {
    if solari_debug_validate_light_ids() {
        atomicAdd(&solari_debug_counters.oob_light_translation_count, 1u);
    }
}

fn solari_debug_count_world_cache_probe_wrap() {
    if solari_debug_validate_reservoirs() {
        atomicAdd(&solari_debug_counters.world_cache_probe_wrap_count, 1u);
    }
}

fn solari_debug_count_zero_distance_visibility() {
    if solari_debug_validate_light_ids() {
        atomicAdd(&solari_debug_counters.zero_distance_visibility_count, 1u);
    }
}

fn solari_debug_count_empty_light_table_sample() {
    if solari_debug_validate_light_ids() {
        atomicAdd(&solari_debug_counters.empty_light_table_sample_count, 1u);
    }
}
