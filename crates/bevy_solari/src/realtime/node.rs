use super::{
    prepare::{SolariLightingResources, LIGHT_TILE_BLOCKS, WORLD_CACHE_SIZE},
    SolariDebugMode, SolariLighting, SOLARI_DEBUG_COUNTER_NAMES,
};
use crate::scene::RaytracingSceneBindings;
#[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
use bevy_anti_alias::dlss::ViewDlssRayReconstructionTextures;
use bevy_asset::{load_embedded_asset, AssetServer, Handle};
use bevy_core_pipeline::prepass::{
    PreviousViewData, PreviousViewUniformOffset, PreviousViewUniforms, ViewPrepassTextures,
};
use bevy_diagnostic::FrameCount;
use bevy_ecs::{prelude::*, resource::Resource, system::Commands};
use bevy_render::{
    diagnostic::RecordDiagnostics as _,
    render_resource::{
        binding_types::{
            storage_buffer_sized, texture_2d, texture_depth_2d, texture_storage_2d, uniform_buffer,
            uniform_buffer_sized,
        },
        BindGroup, BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries, BufferId,
        CachedComputePipelineId, ComputePassDescriptor, ComputePipelineDescriptor, LoadOp,
        PipelineCache, RenderPassDescriptor, ShaderStages, StorageTextureAccess, TextureFormat,
        TextureSampleType, TextureView, TextureViewId,
    },
    renderer::{RenderContext, RenderDevice, ViewQuery},
    view::{ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
};
use bevy_shader::{Shader, ShaderDefVal};
use bevy_utils::default;

/// Resource holding the Solari lighting pipeline configuration.
#[derive(Resource)]
pub struct SolariLightingPipelines {
    bind_group_layout: BindGroupLayoutDescriptor,
    bind_group_layout_world_cache_active_cells_dispatch: BindGroupLayoutDescriptor,
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    bind_group_layout_resolve_dlss_rr_textures: BindGroupLayoutDescriptor,
    prepare_world_cache_dispatch_pipeline: CachedComputePipelineId,
    clear_world_cache_active_cells_pipeline: CachedComputePipelineId,
    sample_di_for_world_cache_pipeline: CachedComputePipelineId,
    sample_gi_for_world_cache_pipeline: CachedComputePipelineId,
    blend_new_world_cache_samples_pipeline: CachedComputePipelineId,
    presample_light_tiles_pipeline: CachedComputePipelineId,
    di_initial_and_temporal_pipeline: CachedComputePipelineId,
    di_spatial_and_shade_pipeline: CachedComputePipelineId,
    gi_initial_and_temporal_pipeline: CachedComputePipelineId,
    gi_spatial_and_shade_pipeline: CachedComputePipelineId,
    specular_gi_pipeline: CachedComputePipelineId,
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    specular_gi_with_psr_pipeline: CachedComputePipelineId,
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    resolve_dlss_rr_textures_pipeline: CachedComputePipelineId,
}

#[cfg(any(not(feature = "dlss"), feature = "force_disable_dlss"))]
type SolariLightingViewQuery = (
    &'static SolariLighting,
    &'static SolariLightingResources,
    &'static ViewTarget,
    &'static ViewPrepassTextures,
    &'static ViewUniformOffset,
    &'static PreviousViewUniformOffset,
    Option<&'static mut SolariViewBindGroups>,
);

#[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
type SolariLightingViewQuery = (
    &'static SolariLighting,
    &'static SolariLightingResources,
    &'static ViewTarget,
    &'static ViewPrepassTextures,
    &'static ViewUniformOffset,
    &'static PreviousViewUniformOffset,
    Option<&'static ViewDlssRayReconstructionTextures>,
    Option<&'static mut SolariViewBindGroups>,
);

#[derive(Component)]
pub(crate) struct SolariViewBindGroups {
    key: SolariViewBindGroupKey,
    bind_group: BindGroup,
    bind_group_world_cache_active_cells_dispatch: BindGroup,
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    bind_group_resolve_dlss_rr_textures: Option<BindGroup>,
}

#[derive(Clone, PartialEq, Eq)]
struct SolariViewBindGroupKey {
    view_target_main: TextureViewId,
    view_target_other: TextureViewId,
    view_target_sampled: Option<TextureViewId>,
    gbuffer: TextureViewId,
    depth_buffer: TextureViewId,
    motion_vectors: TextureViewId,
    previous_gbuffer: TextureViewId,
    previous_depth_buffer: TextureViewId,
    view_uniforms: BufferId,
    previous_view_uniforms: BufferId,
    light_tile_samples: BufferId,
    light_tile_resolved_samples: BufferId,
    gi_reservoirs_a: BufferId,
    gi_reservoirs_b: BufferId,
    world_cache_checksums: BufferId,
    world_cache_life: BufferId,
    world_cache_radiance: BufferId,
    world_cache_geometry_data: BufferId,
    world_cache_luminance_deltas: BufferId,
    world_cache_active_cells_new_radiance: BufferId,
    world_cache_a: BufferId,
    world_cache_b: BufferId,
    world_cache_active_cell_indices: BufferId,
    world_cache_active_cells_count: BufferId,
    world_cache_active_cells_dispatch: BufferId,
    debug_mode: BufferId,
    debug_counters: BufferId,
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    dlss_guide_textures: Option<[TextureViewId; 4]>,
}

struct SolariViewBindGroupsOwned {
    bind_group: BindGroup,
    bind_group_world_cache_active_cells_dispatch: BindGroup,
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    bind_group_resolve_dlss_rr_textures: Option<BindGroup>,
}

impl SolariViewBindGroups {
    fn clone_bind_groups(&self) -> SolariViewBindGroupsOwned {
        SolariViewBindGroupsOwned {
            bind_group: self.bind_group.clone(),
            bind_group_world_cache_active_cells_dispatch: self
                .bind_group_world_cache_active_cells_dispatch
                .clone(),
            #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
            bind_group_resolve_dlss_rr_textures: self.bind_group_resolve_dlss_rr_textures.clone(),
        }
    }
}

impl SolariViewBindGroupKey {
    #[cfg(any(not(feature = "dlss"), feature = "force_disable_dlss"))]
    fn new(
        view_target: &ViewTarget,
        gbuffer: &TextureView,
        depth_buffer: &TextureView,
        motion_vectors: &TextureView,
        previous_gbuffer: &TextureView,
        previous_depth_buffer: &TextureView,
        view_uniforms: BufferId,
        previous_view_uniforms: BufferId,
        s: &SolariLightingResources,
    ) -> Self {
        Self {
            view_target_main: view_target.main_texture_view().id(),
            view_target_other: view_target.main_texture_other_view().id(),
            view_target_sampled: view_target.sampled_main_texture_view().map(TextureView::id),
            gbuffer: gbuffer.id(),
            depth_buffer: depth_buffer.id(),
            motion_vectors: motion_vectors.id(),
            previous_gbuffer: previous_gbuffer.id(),
            previous_depth_buffer: previous_depth_buffer.id(),
            view_uniforms,
            previous_view_uniforms,
            light_tile_samples: s.light_tile_samples.id(),
            light_tile_resolved_samples: s.light_tile_resolved_samples.id(),
            gi_reservoirs_a: s.gi_reservoirs_a.id(),
            gi_reservoirs_b: s.gi_reservoirs_b.id(),
            world_cache_checksums: s.world_cache_checksums.id(),
            world_cache_life: s.world_cache_life.id(),
            world_cache_radiance: s.world_cache_radiance.id(),
            world_cache_geometry_data: s.world_cache_geometry_data.id(),
            world_cache_luminance_deltas: s.world_cache_luminance_deltas.id(),
            world_cache_active_cells_new_radiance: s.world_cache_active_cells_new_radiance.id(),
            world_cache_a: s.world_cache_a.id(),
            world_cache_b: s.world_cache_b.id(),
            world_cache_active_cell_indices: s.world_cache_active_cell_indices.id(),
            world_cache_active_cells_count: s.world_cache_active_cells_count.id(),
            world_cache_active_cells_dispatch: s.world_cache_active_cells_dispatch.id(),
            debug_mode: s.debug_mode.id(),
            debug_counters: s.debug_counters.id(),
        }
    }

    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    fn new(
        view_target: &ViewTarget,
        gbuffer: &TextureView,
        depth_buffer: &TextureView,
        motion_vectors: &TextureView,
        previous_gbuffer: &TextureView,
        previous_depth_buffer: &TextureView,
        view_uniforms: BufferId,
        previous_view_uniforms: BufferId,
        s: &SolariLightingResources,
        view_dlss_rr_textures: Option<&ViewDlssRayReconstructionTextures>,
    ) -> Self {
        Self {
            view_target_main: view_target.main_texture_view().id(),
            view_target_other: view_target.main_texture_other_view().id(),
            view_target_sampled: view_target.sampled_main_texture_view().map(TextureView::id),
            gbuffer: gbuffer.id(),
            depth_buffer: depth_buffer.id(),
            motion_vectors: motion_vectors.id(),
            previous_gbuffer: previous_gbuffer.id(),
            previous_depth_buffer: previous_depth_buffer.id(),
            view_uniforms,
            previous_view_uniforms,
            light_tile_samples: s.light_tile_samples.id(),
            light_tile_resolved_samples: s.light_tile_resolved_samples.id(),
            gi_reservoirs_a: s.gi_reservoirs_a.id(),
            gi_reservoirs_b: s.gi_reservoirs_b.id(),
            world_cache_checksums: s.world_cache_checksums.id(),
            world_cache_life: s.world_cache_life.id(),
            world_cache_radiance: s.world_cache_radiance.id(),
            world_cache_geometry_data: s.world_cache_geometry_data.id(),
            world_cache_luminance_deltas: s.world_cache_luminance_deltas.id(),
            world_cache_active_cells_new_radiance: s.world_cache_active_cells_new_radiance.id(),
            world_cache_a: s.world_cache_a.id(),
            world_cache_b: s.world_cache_b.id(),
            world_cache_active_cell_indices: s.world_cache_active_cell_indices.id(),
            world_cache_active_cells_count: s.world_cache_active_cells_count.id(),
            world_cache_active_cells_dispatch: s.world_cache_active_cells_dispatch.id(),
            debug_mode: s.debug_mode.id(),
            debug_counters: s.debug_counters.id(),
            dlss_guide_textures: view_dlss_rr_textures.map(|d| {
                [
                    d.diffuse_albedo.default_view.id(),
                    d.specular_albedo.default_view.id(),
                    d.normal_roughness.default_view.id(),
                    d.specular_motion_vectors.default_view.id(),
                ]
            }),
        }
    }
}

pub fn solari_lighting(
    view: ViewQuery<SolariLightingViewQuery>,
    solari_pipelines: Option<Res<SolariLightingPipelines>>,
    pipeline_cache: Res<PipelineCache>,
    scene_bindings: Res<RaytracingSceneBindings>,
    view_uniforms: Res<ViewUniforms>,
    previous_view_uniforms: Res<PreviousViewUniforms>,
    frame_count: Res<FrameCount>,
    render_device: Res<RenderDevice>,
    debug_mode: Option<Res<SolariDebugMode>>,
    mut commands: Commands,
    mut ctx: RenderContext,
) {
    let view_entity = view.entity();

    #[cfg(any(not(feature = "dlss"), feature = "force_disable_dlss"))]
    let (
        solari_lighting,
        solari_lighting_resources,
        view_target,
        view_prepass_textures,
        view_uniform_offset,
        previous_view_uniform_offset,
        mut solari_view_bind_groups,
    ) = view.into_inner();

    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    let (
        solari_lighting,
        solari_lighting_resources,
        view_target,
        view_prepass_textures,
        view_uniform_offset,
        previous_view_uniform_offset,
        view_dlss_rr_textures,
        mut solari_view_bind_groups,
    ) = view.into_inner();

    let Some(pipelines) = solari_pipelines else {
        return;
    };

    #[cfg(not(all(feature = "dlss", not(feature = "force_disable_dlss"))))]
    let specular_gi_pipeline = pipelines.specular_gi_pipeline;
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    let specular_gi_pipeline = if view_dlss_rr_textures.is_some() {
        pipelines.specular_gi_with_psr_pipeline
    } else {
        pipelines.specular_gi_pipeline
    };

    let (
        Some(prepare_world_cache_dispatch_pipeline),
        Some(clear_world_cache_active_cells_pipeline),
        Some(sample_di_for_world_cache_pipeline),
        Some(sample_gi_for_world_cache_pipeline),
        Some(blend_new_world_cache_samples_pipeline),
        Some(presample_light_tiles_pipeline),
        Some(di_initial_and_temporal_pipeline),
        Some(di_spatial_and_shade_pipeline),
        Some(gi_initial_and_temporal_pipeline),
        Some(gi_spatial_and_shade_pipeline),
        Some(specular_gi_pipeline),
        Some(scene_bind_group),
        Some(gbuffer),
        Some(depth_buffer),
        Some(motion_vectors),
        Some(previous_gbuffer),
        Some(previous_depth_buffer),
        Some(view_uniforms_binding),
        Some(previous_view_uniforms_binding),
        Some(view_uniforms_buffer_id),
        Some(previous_view_uniforms_buffer_id),
    ) = (
        pipeline_cache.get_compute_pipeline(pipelines.prepare_world_cache_dispatch_pipeline),
        pipeline_cache.get_compute_pipeline(pipelines.clear_world_cache_active_cells_pipeline),
        pipeline_cache.get_compute_pipeline(pipelines.sample_di_for_world_cache_pipeline),
        pipeline_cache.get_compute_pipeline(pipelines.sample_gi_for_world_cache_pipeline),
        pipeline_cache.get_compute_pipeline(pipelines.blend_new_world_cache_samples_pipeline),
        pipeline_cache.get_compute_pipeline(pipelines.presample_light_tiles_pipeline),
        pipeline_cache.get_compute_pipeline(pipelines.di_initial_and_temporal_pipeline),
        pipeline_cache.get_compute_pipeline(pipelines.di_spatial_and_shade_pipeline),
        pipeline_cache.get_compute_pipeline(pipelines.gi_initial_and_temporal_pipeline),
        pipeline_cache.get_compute_pipeline(pipelines.gi_spatial_and_shade_pipeline),
        pipeline_cache.get_compute_pipeline(specular_gi_pipeline),
        &scene_bindings.bind_group,
        view_prepass_textures.deferred_view(),
        view_prepass_textures.depth_view(),
        view_prepass_textures.motion_vectors_view(),
        view_prepass_textures.previous_deferred_view(),
        view_prepass_textures.previous_depth_view(),
        view_uniforms.uniforms.binding(),
        previous_view_uniforms.uniforms.binding(),
        view_uniforms.uniforms.buffer().map(|buffer| buffer.id()),
        previous_view_uniforms
            .uniforms
            .buffer()
            .map(|buffer| buffer.id()),
    )
    else {
        return;
    };

    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    let Some(resolve_dlss_rr_textures_pipeline) =
        pipeline_cache.get_compute_pipeline(pipelines.resolve_dlss_rr_textures_pipeline)
    else {
        return;
    };

    let view_target_attachment = view_target.get_unsampled_color_attachment();

    let s = solari_lighting_resources;
    let bind_group_key = SolariViewBindGroupKey::new(
        view_target,
        gbuffer,
        depth_buffer,
        motion_vectors,
        previous_gbuffer,
        previous_depth_buffer,
        view_uniforms_buffer_id,
        previous_view_uniforms_buffer_id,
        s,
        #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
        view_dlss_rr_textures,
    );

    let create_bind_groups = |key: SolariViewBindGroupKey| {
        let bind_group = render_device.create_bind_group(
            "solari_lighting_bind_group",
            &pipeline_cache.get_bind_group_layout(&pipelines.bind_group_layout),
            &BindGroupEntries::sequential((
                view_target_attachment.view,
                s.light_tile_samples.as_entire_binding(),
                s.light_tile_resolved_samples.as_entire_binding(),
                &s.di_reservoirs_a,
                &s.di_reservoirs_b,
                s.gi_reservoirs_a.as_entire_binding(),
                s.gi_reservoirs_b.as_entire_binding(),
                gbuffer,
                depth_buffer,
                motion_vectors,
                previous_gbuffer,
                previous_depth_buffer,
                view_uniforms_binding.clone(),
                previous_view_uniforms_binding.clone(),
                s.world_cache_checksums.as_entire_binding(),
                s.world_cache_life.as_entire_binding(),
                s.world_cache_radiance.as_entire_binding(),
                s.world_cache_geometry_data.as_entire_binding(),
                s.world_cache_luminance_deltas.as_entire_binding(),
                s.world_cache_active_cells_new_radiance.as_entire_binding(),
                s.world_cache_a.as_entire_binding(),
                s.world_cache_b.as_entire_binding(),
                s.world_cache_active_cell_indices.as_entire_binding(),
                s.world_cache_active_cells_count.as_entire_binding(),
                s.debug_mode.as_entire_binding(),
                s.debug_counters.as_entire_binding(),
            )),
        );
        let bind_group_world_cache_active_cells_dispatch = render_device.create_bind_group(
            "solari_lighting_bind_group_world_cache_active_cells_dispatch",
            &pipeline_cache.get_bind_group_layout(
                &pipelines.bind_group_layout_world_cache_active_cells_dispatch,
            ),
            &BindGroupEntries::single(s.world_cache_active_cells_dispatch.as_entire_binding()),
        );

        #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
        let bind_group_resolve_dlss_rr_textures = view_dlss_rr_textures.map(|d| {
            render_device.create_bind_group(
                "solari_lighting_bind_group_resolve_dlss_rr_textures",
                &pipeline_cache
                    .get_bind_group_layout(&pipelines.bind_group_layout_resolve_dlss_rr_textures),
                &BindGroupEntries::sequential((
                    &d.diffuse_albedo.default_view,
                    &d.specular_albedo.default_view,
                    &d.normal_roughness.default_view,
                    &d.specular_motion_vectors.default_view,
                )),
            )
        });

        SolariViewBindGroups {
            key,
            bind_group,
            bind_group_world_cache_active_cells_dispatch,
            #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
            bind_group_resolve_dlss_rr_textures,
        }
    };

    let bind_groups = if let Some(cache) = solari_view_bind_groups.as_deref_mut() {
        if cache.key != bind_group_key {
            *cache = create_bind_groups(bind_group_key.clone());
        }
        cache.clone_bind_groups()
    } else {
        let bind_groups = create_bind_groups(bind_group_key);
        let clone = bind_groups.clone_bind_groups();
        commands.entity(view_entity).insert(bind_groups);
        clone
    };

    let bind_group = bind_groups.bind_group;
    let bind_group_world_cache_active_cells_dispatch =
        bind_groups.bind_group_world_cache_active_cells_dispatch;
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    let bind_group_resolve_dlss_rr_textures = bind_groups.bind_group_resolve_dlss_rr_textures;

    let light_tile_budget = adaptive_light_tile_budget(scene_bindings.light_source_count());
    let push_constants = [
        frame_count.0.wrapping_mul(5_782_582),
        solari_lighting.reset as u32,
        frame_count.0,
        light_tile_budget,
    ];

    let diagnostics = ctx.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();

    let command_encoder = ctx.command_encoder();
    let debug_mode = debug_mode.map(|mode| *mode).unwrap_or_default();
    if debug_mode.any() {
        command_encoder.clear_buffer(&s.debug_counters, 0, None);
    }

    // Clear the view target if we're the first node to write to it
    if matches!(view_target_attachment.ops.load, LoadOp::Clear(_)) {
        command_encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("solari_lighting_clear"),
            color_attachments: &[Some(view_target_attachment)],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    }

    let mut pass = command_encoder.begin_compute_pass(&ComputePassDescriptor {
        label: Some("solari_lighting"),
        timestamp_writes: None,
    });

    let dx = solari_lighting_resources.view_size.x.div_ceil(8);
    let dy = solari_lighting_resources.view_size.y.div_ceil(8);

    pass.set_bind_group(0, scene_bind_group, &[]);
    pass.set_bind_group(
        1,
        &bind_group,
        &[
            view_uniform_offset.offset,
            previous_view_uniform_offset.offset,
        ],
    );

    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    if let Some(bind_group_resolve_dlss_rr_textures) = &bind_group_resolve_dlss_rr_textures {
        pass.set_bind_group(2, bind_group_resolve_dlss_rr_textures, &[]);
        pass.set_pipeline(resolve_dlss_rr_textures_pipeline);
        pass.dispatch_workgroups(dx, dy, 1);
    }

    let d = diagnostics.time_span(&mut pass, "solari_lighting/presample_light_tiles");
    pass.set_pipeline(presample_light_tiles_pipeline);
    pass.set_immediates(0, bytemuck::cast_slice(&push_constants));
    if light_tile_budget != 0 {
        pass.dispatch_workgroups(light_tile_budget, 1, 1);
    }
    d.end(&mut pass);

    let d = diagnostics.time_span(&mut pass, "solari_lighting/world_cache");

    pass.set_bind_group(2, &bind_group_world_cache_active_cells_dispatch, &[]);

    pass.set_pipeline(prepare_world_cache_dispatch_pipeline);
    pass.dispatch_workgroups(1, 1, 1);

    pass.set_bind_group(2, None, &[]);

    pass.set_pipeline(sample_di_for_world_cache_pipeline);
    pass.set_immediates(0, bytemuck::cast_slice(&push_constants));
    pass.dispatch_workgroups_indirect(
        &solari_lighting_resources.world_cache_active_cells_dispatch,
        0,
    );

    pass.set_pipeline(sample_gi_for_world_cache_pipeline);
    pass.set_immediates(0, bytemuck::cast_slice(&push_constants));
    pass.dispatch_workgroups_indirect(
        &solari_lighting_resources.world_cache_active_cells_dispatch,
        0,
    );

    pass.set_pipeline(blend_new_world_cache_samples_pipeline);
    pass.dispatch_workgroups_indirect(
        &solari_lighting_resources.world_cache_active_cells_dispatch,
        0,
    );

    pass.set_bind_group(2, &bind_group_world_cache_active_cells_dispatch, &[]);
    pass.set_pipeline(clear_world_cache_active_cells_pipeline);
    pass.dispatch_workgroups(1, 1, 1);
    pass.set_bind_group(2, None, &[]);

    d.end(&mut pass);

    let d = diagnostics.time_span(&mut pass, "solari_lighting/direct_lighting");

    pass.set_pipeline(di_initial_and_temporal_pipeline);
    pass.set_immediates(0, bytemuck::cast_slice(&push_constants));
    pass.dispatch_workgroups(dx, dy, 1);

    pass.set_pipeline(di_spatial_and_shade_pipeline);
    pass.set_immediates(0, bytemuck::cast_slice(&push_constants));
    pass.dispatch_workgroups(dx, dy, 1);

    d.end(&mut pass);

    let d = diagnostics.time_span(&mut pass, "solari_lighting/diffuse_indirect_lighting");

    pass.set_pipeline(gi_initial_and_temporal_pipeline);
    pass.set_immediates(0, bytemuck::cast_slice(&push_constants));
    pass.dispatch_workgroups(dx, dy, 1);

    pass.set_pipeline(gi_spatial_and_shade_pipeline);
    pass.set_immediates(0, bytemuck::cast_slice(&push_constants));
    pass.dispatch_workgroups(dx, dy, 1);

    d.end(&mut pass);

    let d = diagnostics.time_span(&mut pass, "solari_lighting/specular_indirect_lighting");
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    if let Some(bind_group_resolve_dlss_rr_textures) = &bind_group_resolve_dlss_rr_textures {
        pass.set_bind_group(2, bind_group_resolve_dlss_rr_textures, &[]);
    }
    pass.set_pipeline(specular_gi_pipeline);
    pass.set_immediates(0, bytemuck::cast_slice(&push_constants));
    pass.dispatch_workgroups(dx, dy, 1);
    d.end(&mut pass);

    drop(pass);

    diagnostics.record_u32(
        ctx.command_encoder(),
        &s.world_cache_active_cells_count.slice(..),
        "solari_lighting/world_cache_active_cells_count",
    );
    if debug_mode.any() {
        for (index, name) in SOLARI_DEBUG_COUNTER_NAMES.iter().enumerate() {
            let offset = index as u64 * size_of::<u32>() as u64;
            diagnostics.record_u32(
                ctx.command_encoder(),
                &s.debug_counters
                    .slice(offset..offset + size_of::<u32>() as u64),
                format!("solari_lighting/debug/{name}"),
            );
        }
    }
}

fn adaptive_light_tile_budget(light_source_count: u32) -> u32 {
    match light_source_count {
        0 => 0,
        1..=16 => 8,
        17..=128 => 32,
        129..=512 => 64,
        _ => LIGHT_TILE_BLOCKS as u32,
    }
}

/// Initializes the Solari lighting pipelines at render startup.
pub fn init_solari_lighting_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    scene_bindings: Res<RaytracingSceneBindings>,
    asset_server: Res<AssetServer>,
) {
    let bind_group_layout = BindGroupLayoutDescriptor::new(
        "solari_lighting_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                texture_storage_2d(TextureFormat::Rgba16Float, StorageTextureAccess::ReadWrite),
                storage_buffer_sized(false, None),
                storage_buffer_sized(false, None),
                texture_storage_2d(TextureFormat::Rgba32Uint, StorageTextureAccess::ReadWrite),
                texture_storage_2d(TextureFormat::Rgba32Uint, StorageTextureAccess::ReadWrite),
                storage_buffer_sized(false, None),
                storage_buffer_sized(false, None),
                texture_2d(TextureSampleType::Uint),
                texture_depth_2d(),
                texture_2d(TextureSampleType::Float { filterable: true }),
                texture_2d(TextureSampleType::Uint),
                texture_depth_2d(),
                uniform_buffer::<ViewUniform>(true),
                uniform_buffer::<PreviousViewData>(true),
                storage_buffer_sized(false, None),
                storage_buffer_sized(false, None),
                storage_buffer_sized(false, None),
                storage_buffer_sized(false, None),
                storage_buffer_sized(false, None),
                storage_buffer_sized(false, None),
                storage_buffer_sized(false, None),
                storage_buffer_sized(false, None),
                storage_buffer_sized(false, None),
                storage_buffer_sized(false, None),
                uniform_buffer_sized(false, None),
                storage_buffer_sized(false, None),
            ),
        ),
    );

    let bind_group_layout_world_cache_active_cells_dispatch = BindGroupLayoutDescriptor::new(
        "solari_lighting_bind_group_layout_world_cache_active_cells_dispatch",
        &BindGroupLayoutEntries::single(ShaderStages::COMPUTE, storage_buffer_sized(false, None)),
    );

    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    let bind_group_layout_resolve_dlss_rr_textures = BindGroupLayoutDescriptor::new(
        "solari_lighting_bind_group_layout_resolve_dlss_rr_textures",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                texture_storage_2d(TextureFormat::Rgba8Unorm, StorageTextureAccess::WriteOnly),
                texture_storage_2d(TextureFormat::Rgba8Unorm, StorageTextureAccess::WriteOnly),
                texture_storage_2d(TextureFormat::Rgba16Float, StorageTextureAccess::WriteOnly),
                texture_storage_2d(TextureFormat::Rg16Float, StorageTextureAccess::WriteOnly),
            ),
        ),
    );

    let create_pipeline = |label: &'static str,
                           entry_point: &'static str,
                           shader: Handle<Shader>,
                           extra_bind_group_layout: Option<&BindGroupLayoutDescriptor>,
                           extra_shader_defs: Vec<ShaderDefVal>| {
        let mut layout = vec![
            scene_bindings.bind_group_layout.clone(),
            bind_group_layout.clone(),
        ];
        if let Some(extra_bind_group_layout) = extra_bind_group_layout {
            layout.push(extra_bind_group_layout.clone());
        }

        let mut shader_defs = vec![ShaderDefVal::UInt(
            "WORLD_CACHE_SIZE".into(),
            WORLD_CACHE_SIZE as u32,
        )];
        shader_defs.push("SOLARI_DEBUG_COUNTERS".into());
        shader_defs.extend_from_slice(&extra_shader_defs);

        pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some(label.into()),
            layout,
            immediate_size: 16,
            shader,
            shader_defs,
            entry_point: Some(entry_point.into()),
            ..default()
        })
    };

    commands.insert_resource(SolariLightingPipelines {
        bind_group_layout: bind_group_layout.clone(),
        bind_group_layout_world_cache_active_cells_dispatch:
            bind_group_layout_world_cache_active_cells_dispatch.clone(),
        #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
        bind_group_layout_resolve_dlss_rr_textures: bind_group_layout_resolve_dlss_rr_textures
            .clone(),
        prepare_world_cache_dispatch_pipeline: create_pipeline(
            "solari_lighting_prepare_world_cache_dispatch_pipeline",
            "prepare_world_cache_dispatch",
            load_embedded_asset!(asset_server.as_ref(), "world_cache_compact.wgsl"),
            Some(&bind_group_layout_world_cache_active_cells_dispatch),
            vec![],
        ),
        clear_world_cache_active_cells_pipeline: create_pipeline(
            "solari_lighting_clear_world_cache_active_cells_pipeline",
            "clear_world_cache_active_cells",
            load_embedded_asset!(asset_server.as_ref(), "world_cache_compact.wgsl"),
            Some(&bind_group_layout_world_cache_active_cells_dispatch),
            vec![],
        ),
        sample_di_for_world_cache_pipeline: create_pipeline(
            "solari_lighting_sample_di_for_world_cache_pipeline",
            "sample_di",
            load_embedded_asset!(asset_server.as_ref(), "world_cache_update.wgsl"),
            None,
            vec![],
        ),
        sample_gi_for_world_cache_pipeline: create_pipeline(
            "solari_lighting_sample_gi_for_world_cache_pipeline",
            "sample_gi",
            load_embedded_asset!(asset_server.as_ref(), "world_cache_update.wgsl"),
            None,
            vec!["WORLD_CACHE_QUERY_ATOMIC_MAX_LIFETIME".into()],
        ),
        blend_new_world_cache_samples_pipeline: create_pipeline(
            "solari_lighting_blend_new_world_cache_samples_pipeline",
            "blend_new_samples",
            load_embedded_asset!(asset_server.as_ref(), "world_cache_update.wgsl"),
            None,
            vec![],
        ),
        presample_light_tiles_pipeline: create_pipeline(
            "solari_lighting_presample_light_tiles_pipeline",
            "presample_light_tiles",
            load_embedded_asset!(asset_server.as_ref(), "presample_light_tiles.wgsl"),
            None,
            vec![],
        ),
        di_initial_and_temporal_pipeline: create_pipeline(
            "solari_lighting_di_initial_and_temporal_pipeline",
            "initial_and_temporal",
            load_embedded_asset!(asset_server.as_ref(), "restir_di.wgsl"),
            None,
            vec![],
        ),
        di_spatial_and_shade_pipeline: create_pipeline(
            "solari_lighting_di_spatial_and_shade_pipeline",
            "spatial_and_shade",
            load_embedded_asset!(asset_server.as_ref(), "restir_di.wgsl"),
            None,
            vec![],
        ),
        gi_initial_and_temporal_pipeline: create_pipeline(
            "solari_lighting_gi_initial_and_temporal_pipeline",
            "initial_and_temporal",
            load_embedded_asset!(asset_server.as_ref(), "restir_gi.wgsl"),
            None,
            vec!["WORLD_CACHE_FIRST_BOUNCE_LIGHT_LEAK_PREVENTION".into()],
        ),
        gi_spatial_and_shade_pipeline: create_pipeline(
            "solari_lighting_gi_spatial_and_shade_pipeline",
            "spatial_and_shade",
            load_embedded_asset!(asset_server.as_ref(), "restir_gi.wgsl"),
            None,
            vec![],
        ),
        specular_gi_pipeline: create_pipeline(
            "solari_lighting_specular_gi_pipeline",
            "specular_gi",
            load_embedded_asset!(asset_server.as_ref(), "specular_gi.wgsl"),
            None,
            vec![],
        ),
        #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
        specular_gi_with_psr_pipeline: create_pipeline(
            "solari_lighting_specular_gi_with_psr_pipeline",
            "specular_gi",
            load_embedded_asset!(asset_server.as_ref(), "specular_gi.wgsl"),
            Some(&bind_group_layout_resolve_dlss_rr_textures),
            vec!["DLSS_RR_GUIDE_BUFFERS".into()],
        ),
        #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
        resolve_dlss_rr_textures_pipeline: create_pipeline(
            "solari_lighting_resolve_dlss_rr_textures_pipeline",
            "resolve_dlss_rr_textures",
            load_embedded_asset!(asset_server.as_ref(), "resolve_dlss_rr_textures.wgsl"),
            Some(&bind_group_layout_resolve_dlss_rr_textures),
            vec!["DLSS_RR_GUIDE_BUFFERS".into()],
        ),
    });
}
