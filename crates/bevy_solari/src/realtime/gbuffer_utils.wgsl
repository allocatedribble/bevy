#define_import_path bevy_solari::gbuffer_utils

#import bevy_pbr::pbr_deferred_types::unpack_24bit_normal
#import bevy_pbr::rgb9e5::rgb9e5_to_vec3_
#import bevy_pbr::utils::octahedral_decode
#import bevy_render::view::{View, depth_ndc_to_view_z}
#import bevy_solari::scene_bindings::ResolvedMaterial

struct ResolvedGPixel {
    world_position: vec3<f32>,
    world_normal: vec3<f32>,
    material: ResolvedMaterial,
}

fn gpixel_resolve(gpixel: vec4<u32>, depth: f32, pixel_id: vec2<u32>, view_size: vec2<f32>, world_from_clip: mat4x4<f32>) -> ResolvedGPixel {
    let world_position = reconstruct_world_position(pixel_id, depth, view_size, world_from_clip);
    let world_normal = octahedral_decode(unpack_24bit_normal(gpixel.a));

    let base_rough = unpack4x8unorm(gpixel.r);
    let base_color = pow(base_rough.rgb, vec3(2.2));
    let perceptual_roughness = base_rough.a;
    let roughness = perceptual_roughness * perceptual_roughness;
    let props = unpack4x8unorm(gpixel.b);
    let reflectance = props.r;
    let metallic = props.g;
    let emissive = rgb9e5_to_vec3_(gpixel.g);
    let material = ResolvedMaterial(base_color, emissive, reflectance, perceptual_roughness, roughness, metallic);

    return ResolvedGPixel(world_position, world_normal, material);
}

fn reconstruct_world_position(pixel_id: vec2<u32>, depth: f32, view_size: vec2<f32>, world_from_clip: mat4x4<f32>) -> vec3<f32> {
    let uv = (vec2<f32>(pixel_id) + 0.5) / view_size;
    let xy_ndc = (uv - vec2(0.5)) * vec2(2.0, -2.0);
    let world_pos = world_from_clip * vec4(xy_ndc, depth, 1.0);
    return world_pos.xyz / world_pos.w;
}

// Reject if tangent plane differs more than 0.3% or angle between normals more than 90 degrees
fn pixel_dissimilar(depth: f32, world_position: vec3<f32>, other_world_position: vec3<f32>, normal: vec3<f32>, other_normal: vec3<f32>, view: View) -> bool {
    // https://developer.download.nvidia.com/video/gputechconf/gtc/2020/presentations/s22699-fast-denoising-with-self-stabilizing-recurrent-blurs.pdf#page=45
    let tangent_plane_distance = abs(dot(normal, other_world_position - world_position));
    let view_z = -depth_ndc_to_view_z(depth, view.clip_from_view, view.view_from_clip);

    return tangent_plane_distance / view_z > 0.003 || dot(normal, other_normal) < 0.0;
}

fn pixel_dissimilar_di(
    depth: f32,
    world_position: vec3<f32>,
    other_world_position: vec3<f32>,
    normal: vec3<f32>,
    other_normal: vec3<f32>,
    material: ResolvedMaterial,
    other_material: ResolvedMaterial,
    view: View,
) -> bool {
    return pixel_dissimilar(depth, world_position, other_world_position, normal, other_normal, view)
        || material_dissimilar(material, other_material, 0.08, 0.06, 0.12, 0.24);
}

fn pixel_dissimilar_gi(
    depth: f32,
    world_position: vec3<f32>,
    other_world_position: vec3<f32>,
    normal: vec3<f32>,
    other_normal: vec3<f32>,
    material: ResolvedMaterial,
    other_material: ResolvedMaterial,
    view: View,
) -> bool {
    return pixel_dissimilar(depth, world_position, other_world_position, normal, other_normal, view)
        || material_dissimilar(material, other_material, 0.18, 0.12, 0.25, 0.45);
}

fn material_dissimilar(
    material: ResolvedMaterial,
    other_material: ResolvedMaterial,
    roughness_delta_max: f32,
    metallic_delta_max: f32,
    base_luminance_delta_max: f32,
    base_color_distance_max: f32,
) -> bool {
    if abs(material.roughness - other_material.roughness) > roughness_delta_max {
        return true;
    }
    if abs(material.metallic - other_material.metallic) > metallic_delta_max {
        return true;
    }

    let base_luminance = material_luminance(material.base_color);
    let other_base_luminance = material_luminance(other_material.base_color);
    if abs(base_luminance - other_base_luminance) > base_luminance_delta_max {
        return true;
    }
    if length(material.base_color - other_material.base_color) > base_color_distance_max {
        return true;
    }

    return (material_luminance(material.emissive) > 0.001) != (material_luminance(other_material.emissive) > 0.001);
}

fn material_luminance(color: vec3<f32>) -> f32 {
    return dot(color, vec3(0.2126, 0.7152, 0.0722));
}

fn permute_pixel(pixel_id: vec2<u32>, frame_index: u32, view_size: vec2<f32>) -> vec2<u32> {
    let offset = vec2<i32>(i32(frame_index & 3u), i32((frame_index >> 2u) & 3u));
    let max_pixel = max(vec2<i32>(view_size) - vec2<i32>(1), vec2<i32>(0));

    let shifted = vec2<i32>(pixel_id) + offset;
    let permuted = (shifted ^ vec2<i32>(3)) - offset;

    return vec2<u32>(clamp(permuted, vec2<i32>(0), max_pixel));
}
