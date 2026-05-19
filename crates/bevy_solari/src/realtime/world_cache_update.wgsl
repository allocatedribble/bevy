enable wgpu_ray_query;

#import bevy_core_pipeline::tonemapping::tonemapping_luminance as luminance
#import bevy_pbr::utils::{rand_f, rand_range_u, sample_cosine_hemisphere}
#import bevy_render::view::View
#import bevy_solari::presample_light_tiles::{ResolvedLightSamplePacked, unpack_resolved_light_sample}
#import bevy_solari::sampling::{calculate_resolved_light_contribution, trace_light_visibility}
#import bevy_solari::scene_bindings::{trace_ray, resolve_ray_hit_full, RAY_T_MIN}
#import bevy_solari::solari_debug::solari_debug_validate_radiance
#import bevy_solari::world_cache::{
    WORLD_CACHE_MAX_TEMPORAL_SAMPLES,
    WORLD_CACHE_DIRECT_LIGHT_SAMPLE_COUNT,
    WORLD_CACHE_MAX_GI_RAY_DISTANCE,
    WORLD_CACHE_CELL_UPDATES_SOFT_CAP,
    WORLD_CACHE_CELL_LIFETIME,
    query_world_cache,
}
#import bevy_solari::realtime_bindings::{
    light_tile_resolved_samples,
    view,
    constants,
    world_cache_active_cells_count,
    world_cache_active_cell_indices,
    world_cache_geometry_data,
    world_cache_radiance,
    world_cache_luminance_deltas,
    world_cache_active_cells_new_radiance,
}

@compute @workgroup_size(64, 1, 1)
fn sample_di(@builtin(workgroup_id) workgroup_id: vec3<u32>, @builtin(global_invocation_id) active_cell_id: vec3<u32>) {
    let active_cell_count = atomicLoad(&world_cache_active_cells_count);
    if active_cell_id.x >= active_cell_count { return; }

    let cell_index = world_cache_active_cell_indices[active_cell_id.x];
    let geometry_data = world_cache_geometry_data[cell_index];
    var rng = cell_index + constants.frame_index;

    if rand_f(&rng) >= min(1.0, f32(WORLD_CACHE_CELL_UPDATES_SOFT_CAP) / f32(active_cell_count)) { return; }

    let new_radiance = solari_debug_validate_radiance(sample_random_light_ris(geometry_data.world_position, geometry_data.world_normal, workgroup_id.xy, &rng));

    world_cache_active_cells_new_radiance[active_cell_id.x] = new_radiance;
}

@compute @workgroup_size(64, 1, 1)
fn sample_gi(@builtin(workgroup_id) workgroup_id: vec3<u32>, @builtin(global_invocation_id) active_cell_id: vec3<u32>) {
    let active_cell_count = atomicLoad(&world_cache_active_cells_count);
    if active_cell_id.x >= active_cell_count { return; }

    let cell_index = world_cache_active_cell_indices[active_cell_id.x];
    let geometry_data = world_cache_geometry_data[cell_index];
    var rng = cell_index + constants.frame_index;

    if rand_f(&rng) >= min(1.0, f32(WORLD_CACHE_CELL_UPDATES_SOFT_CAP) / f32(active_cell_count)) { return; }

    let ray_direction = sample_cosine_hemisphere(geometry_data.world_normal, &rng);
    let ray = trace_ray(geometry_data.world_position + (geometry_data.world_normal * RAY_T_MIN), ray_direction, RAY_T_MIN, WORLD_CACHE_MAX_GI_RAY_DISTANCE, RAY_FLAG_NONE);
    if ray.kind != RAY_QUERY_INTERSECTION_NONE {
        let ray_hit = resolve_ray_hit_full(ray);
        let radiance = solari_debug_validate_radiance(query_world_cache(ray_hit.world_position, ray_hit.geometric_world_normal, view.world_position, ray.t, WORLD_CACHE_CELL_LIFETIME, false, &rng));
        world_cache_active_cells_new_radiance[active_cell_id.x] = solari_debug_validate_radiance(world_cache_active_cells_new_radiance[active_cell_id.x] + ray_hit.material.base_color * radiance);
    }
}

@compute @workgroup_size(64, 1, 1)
fn blend_new_samples(@builtin(global_invocation_id) active_cell_id: vec3<u32>) {
    let active_cell_count = atomicLoad(&world_cache_active_cells_count);
    if active_cell_id.x >= active_cell_count { return; }

    let cell_index = world_cache_active_cell_indices[active_cell_id.x];
    var rng = cell_index + constants.frame_index;

    if rand_f(&rng) >= min(1.0, f32(WORLD_CACHE_CELL_UPDATES_SOFT_CAP) / f32(active_cell_count)) { return; }

    let old_radiance = world_cache_radiance[cell_index];
    let luminance_delta = world_cache_luminance_deltas[cell_index];
    let new_radiance = clamp_world_cache_radiance(old_radiance, world_cache_active_cells_new_radiance[active_cell_id.x], luminance_delta);

    // https://bsky.app/profile/gboisse.bsky.social/post/3m5blga3ftk2a
    var sample_count = min(old_radiance.a + 1.0, WORLD_CACHE_MAX_TEMPORAL_SAMPLES);
    let relative_luminance_jump = abs(luminance(new_radiance) - luminance(old_radiance.rgb)) / max(luminance(old_radiance.rgb), 0.001);
    if relative_luminance_jump > 4.0 || bool(constants.reset) {
        sample_count = 1.0;
    }
    let alpha = abs(luminance_delta) / max(luminance(old_radiance.rgb), 0.001);
    let max_sample_count = mix(WORLD_CACHE_MAX_TEMPORAL_SAMPLES, 1.0, pow(saturate(alpha), 1.0 / 8.0));
    var blend_amount = 1.0 / min(sample_count, max_sample_count);
    if bool(constants.reset) {
        blend_amount = 1.0;
    }

    let blended_radiance = solari_debug_validate_radiance(mix(old_radiance.rgb, new_radiance, blend_amount));
    let blended_luminance_delta = select(mix(luminance_delta, luminance(blended_radiance) - luminance(old_radiance.rgb), 1.0 / 8.0), 0.0, bool(constants.reset));

    world_cache_radiance[cell_index] = vec4(blended_radiance, sample_count);
    world_cache_luminance_deltas[cell_index] = blended_luminance_delta;
}

fn clamp_world_cache_radiance(old_radiance: vec4<f32>, new_radiance: vec3<f32>, luminance_delta: f32) -> vec3<f32> {
    if old_radiance.a < 2.0 {
        return new_radiance;
    }

    let old_luminance = luminance(old_radiance.rgb);
    let new_luminance = luminance(new_radiance);
    if !(new_luminance > 0.0) {
        return vec3(0.0);
    }

    let confidence = saturate(old_radiance.a / WORLD_CACHE_MAX_TEMPORAL_SAMPLES);
    let history_width = max(abs(luminance_delta) * 4.0, old_luminance * mix(2.0, 0.35, confidence) + 0.05);
    let min_luminance = max(0.0, old_luminance - history_width);
    let max_luminance = old_luminance + history_width;
    let clamped_luminance = clamp(new_luminance, min_luminance, max_luminance);
    return new_radiance * (clamped_luminance / new_luminance);
}

fn sample_random_light_ris(world_position: vec3<f32>, world_normal: vec3<f32>, workgroup_id: vec2<u32>, rng: ptr<function, u32>) -> vec3<f32> {
    if constants.light_tile_budget == 0u {
        return vec3(0.0);
    }

    var workgroup_rng = (workgroup_id.x * 5782582u) + workgroup_id.y;
    let light_tile_start = rand_range_u(constants.light_tile_budget, &workgroup_rng) * 1024u;

    var weight_sum = 0.0;
    var selected_sample_radiance = vec3(0.0);
    var selected_sample_target_function = 0.0;
    var selected_sample_world_position = vec4(0.0);
    var selected_valid = false;
    let mis_weight = 1.0 / f32(WORLD_CACHE_DIRECT_LIGHT_SAMPLE_COUNT);
    for (var i = 0u; i < WORLD_CACHE_DIRECT_LIGHT_SAMPLE_COUNT; i++) {
        let tile_sample = light_tile_start + rand_range_u(1024u, rng);
        let resolved_light_sample = unpack_resolved_light_sample(light_tile_resolved_samples[tile_sample], view.exposure);
        let light_contribution = calculate_resolved_light_contribution(resolved_light_sample, world_position, world_normal);

        let contribution = light_contribution.radiance * saturate(dot(light_contribution.wi, world_normal));
        let target_function = luminance(contribution);
        let resampling_weight = mis_weight * (target_function * light_contribution.inverse_pdf);

        weight_sum += resampling_weight;

        if resampling_weight > 0.0 && rand_f(rng) < resampling_weight / weight_sum {
            selected_sample_radiance = contribution;
            selected_sample_target_function = target_function;
            selected_sample_world_position = resolved_light_sample.world_position;
            selected_valid = target_function > 0.0;
        }
    }

    var unbiased_contribution_weight = 0.0;
    if selected_valid {
        let inverse_target_function = select(0.0, 1.0 / selected_sample_target_function, selected_sample_target_function > 0.0);
        unbiased_contribution_weight = weight_sum * inverse_target_function;

        unbiased_contribution_weight *= trace_light_visibility(world_position + (world_normal * RAY_T_MIN), selected_sample_world_position);
    }

    return selected_sample_radiance * unbiased_contribution_weight;
}
