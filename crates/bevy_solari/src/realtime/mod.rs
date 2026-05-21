mod extract;
mod node;
mod prepare;

use crate::SolariPlugins;
use bevy_app::{App, Plugin, PostUpdate};
use bevy_asset::embedded_asset;
use bevy_camera::Hdr;
use bevy_core_pipeline::{
    core_3d::main_opaque_pass_3d,
    prepass::{
        DeferredPrepass, DeferredPrepassDoubleBuffer, DepthPrepass, DepthPrepassDoubleBuffer,
        MotionVectorPrepass,
    },
    schedule::{Core3d, Core3dSystems},
};
use bevy_diagnostic::{DiagnosticPath, DiagnosticsStore};
use bevy_ecs::{
    component::Component,
    reflect::{ReflectComponent, ReflectResource},
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::Res,
};
use bevy_pbr::deferred::deferred_lighting;
use bevy_pbr::DefaultOpaqueRendererMethod;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    extract_resource::{ExtractResource, ExtractResourcePlugin},
    renderer::RenderDevice,
    ExtractSchedule, Render, RenderApp, RenderStartup, RenderSystems,
};
use bevy_shader::load_shader_library;
use extract::{extract_removed_solari_lighting, extract_solari_lighting};
use node::{init_solari_lighting_pipelines, solari_lighting};
use prepare::prepare_solari_lighting_resources;
use tracing::warn;

/// Raytraced direct and indirect lighting.
///
/// When using this plugin, it's highly recommended to set `shadow_maps_enabled: false` on all lights, as Solari replaces
/// traditional shadow mapping.
pub struct SolariLightingPlugin;

impl Plugin for SolariLightingPlugin {
    fn build(&self, app: &mut App) {
        load_shader_library!(app, "solari_debug.wgsl");
        load_shader_library!(app, "gbuffer_utils.wgsl");
        load_shader_library!(app, "realtime_bindings.wgsl");
        load_shader_library!(app, "presample_light_tiles.wgsl");
        embedded_asset!(app, "restir_di.wgsl");
        embedded_asset!(app, "restir_gi.wgsl");
        load_shader_library!(app, "specular_gi.wgsl");
        load_shader_library!(app, "world_cache_query.wgsl");
        embedded_asset!(app, "world_cache_compact.wgsl");
        embedded_asset!(app, "world_cache_update.wgsl");

        load_shader_library!(app, "resolve_dlss_rr_textures.wgsl");

        app.insert_resource(DefaultOpaqueRendererMethod::deferred());
        app.init_resource::<SolariDebugMode>()
            .add_plugins(ExtractResourcePlugin::<SolariDebugMode>::default())
            .add_systems(PostUpdate, panic_on_solari_debug_counters);
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);

        let render_device = render_app.world().resource::<RenderDevice>();
        let features = render_device.features();
        if !features.contains(SolariPlugins::required_wgpu_features()) {
            warn!(
                "SolariLightingPlugin not loaded. GPU lacks support for required features: {:?}.",
                SolariPlugins::required_wgpu_features().difference(features)
            );
            return;
        }

        render_app
            .add_systems(RenderStartup, init_solari_lighting_pipelines)
            .add_systems(
                ExtractSchedule,
                (extract_removed_solari_lighting, extract_solari_lighting).chain(),
            )
            .add_systems(
                Render,
                prepare_solari_lighting_resources.in_set(RenderSystems::PrepareResources),
            )
            .add_systems(
                Core3d,
                solari_lighting
                    .before(deferred_lighting)
                    .before(main_opaque_pass_3d)
                    .in_set(Core3dSystems::MainPass),
            );
    }
}

const SOLARI_DEBUG_VALIDATE_RESERVOIRS: u32 = 1 << 0;
const SOLARI_DEBUG_VALIDATE_LIGHT_IDS: u32 = 1 << 1;
const SOLARI_DEBUG_VALIDATE_NAN_INF: u32 = 1 << 2;
const SOLARI_DEBUG_VISUALIZE_TEMPORAL_SOURCES: u32 = 1 << 3;
const SOLARI_DEBUG_VISUALIZE_WORLD_CACHE: u32 = 1 << 4;

/// GPU-side Solari validation and visualization controls.
#[derive(Resource, ExtractResource, Reflect, Clone, Copy, Debug, Default)]
#[reflect(Resource, Default, Clone)]
pub struct SolariDebugMode {
    pub validate_reservoirs: bool,
    pub validate_light_ids: bool,
    pub validate_nan_inf: bool,
    pub visualize_temporal_sources: bool,
    pub visualize_world_cache: bool,
}

impl SolariDebugMode {
    /// Enables validation counters without replacing the rendered image with a debug view.
    pub const fn validation() -> Self {
        Self {
            validate_reservoirs: true,
            validate_light_ids: true,
            validate_nan_inf: true,
            visualize_temporal_sources: false,
            visualize_world_cache: false,
        }
    }

    pub(crate) const fn bits(self) -> u32 {
        (self.validate_reservoirs as u32 * SOLARI_DEBUG_VALIDATE_RESERVOIRS)
            | (self.validate_light_ids as u32 * SOLARI_DEBUG_VALIDATE_LIGHT_IDS)
            | (self.validate_nan_inf as u32 * SOLARI_DEBUG_VALIDATE_NAN_INF)
            | (self.visualize_temporal_sources as u32 * SOLARI_DEBUG_VISUALIZE_TEMPORAL_SOURCES)
            | (self.visualize_world_cache as u32 * SOLARI_DEBUG_VISUALIZE_WORLD_CACHE)
    }

    pub(crate) const fn any(self) -> bool {
        self.bits() != 0
    }

    const fn validates(self) -> bool {
        self.validate_reservoirs || self.validate_light_ids || self.validate_nan_inf
    }
}

/// Diagnostic counter names, in GPU buffer order.
pub const SOLARI_DEBUG_COUNTER_NAMES: [&str; 8] = [
    "nan_radiance_count",
    "inf_radiance_count",
    "invalid_di_temporal_count",
    "invalid_gi_reuse_count",
    "oob_light_translation_count",
    "world_cache_probe_wrap_count",
    "zero_distance_visibility_count",
    "empty_light_table_sample_count",
];

fn panic_on_solari_debug_counters(
    mode: Res<SolariDebugMode>,
    diagnostics: Option<Res<DiagnosticsStore>>,
) {
    if !mode.validates() {
        return;
    }

    let Some(diagnostics) = diagnostics else {
        return;
    };

    for name in SOLARI_DEBUG_COUNTER_NAMES {
        let path = DiagnosticPath::from_components(["render", "solari_lighting", "debug", name]);
        if let Some(value) = diagnostics
            .get(&path)
            .and_then(|diagnostic| diagnostic.value())
            && value > 0.0
        {
            panic!("Solari debug validation failed: {name}={}", value as u32);
        }
    }
}

/// A component for a 3d camera entity to enable the Solari raytraced lighting system.
///
/// Must be used with `CameraMainTextureUsages::default().with(TextureUsages::STORAGE_BINDING)`, and
/// `Msaa::Off`.
#[derive(Component, Reflect, Clone)]
#[reflect(Component, Default, Clone)]
#[require(
    Hdr,
    DeferredPrepass,
    DepthPrepass,
    MotionVectorPrepass,
    DeferredPrepassDoubleBuffer,
    DepthPrepassDoubleBuffer
)]
pub struct SolariLighting {
    /// Set to true to delete the saved temporal history (past frames).
    ///
    /// Useful for preventing ghosting when the history is no longer
    /// representative of the current frame, such as in sudden camera cuts.
    ///
    /// After setting this to true, it will automatically be toggled
    /// back to false at the end of the frame.
    pub reset: bool,
}

impl Default for SolariLighting {
    fn default() -> Self {
        Self {
            reset: true, // No temporal history on the first frame
        }
    }
}

#[cfg(test)]
mod tests {
    fn permute_pixel_cpu(pixel: (u32, u32), frame_index: u32, view_size: (u32, u32)) -> (u32, u32) {
        let offset = ((frame_index & 3) as i32, ((frame_index >> 2) & 3) as i32);
        let max_pixel = (
            view_size.0.saturating_sub(1) as i32,
            view_size.1.saturating_sub(1) as i32,
        );
        let shifted = (pixel.0 as i32 + offset.0, pixel.1 as i32 + offset.1);
        let permuted = ((shifted.0 ^ 3) - offset.0, (shifted.1 ^ 3) - offset.1);
        (
            permuted.0.clamp(0, max_pixel.0) as u32,
            permuted.1.clamp(0, max_pixel.1) as u32,
        )
    }

    #[test]
    fn temporal_permutation_does_not_underflow_to_far_edge() {
        let view_size = (64, 64);

        for frame_index in 0..16 {
            for x in 0..4 {
                for y in 0..4 {
                    let pixel = permute_pixel_cpu((x, y), frame_index, view_size);
                    assert!(
                        pixel.0 < view_size.0 - 4 && pixel.1 < view_size.1 - 4,
                        "frame {frame_index} mapped ({x}, {y}) to {pixel:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn world_cache_linear_probe_wraps_inside_table() {
        const WORLD_CACHE_SIZE: u32 = 8;
        const WORLD_CACHE_MAX_SEARCH_STEPS: u32 = 16;

        let mut key = WORLD_CACHE_SIZE - 2;
        for _ in 0..WORLD_CACHE_MAX_SEARCH_STEPS {
            assert!(key < WORLD_CACHE_SIZE);
            key = (key + 1) & (WORLD_CACHE_SIZE - 1);
        }
    }
}
