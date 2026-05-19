enable wgpu_ray_query;

#import bevy_solari::realtime_bindings::{
    world_cache_active_cells_count,
}

@group(2) @binding(0) var<storage, read_write> world_cache_active_cells_dispatch: array<atomic<u32>, 3u>;

@compute @workgroup_size(1, 1, 1)
fn prepare_world_cache_dispatch() {
    let active_cell_count = atomicLoad(&world_cache_active_cells_count);
    atomicStore(&world_cache_active_cells_dispatch[0u], (active_cell_count + 63u) / 64u);
    atomicStore(&world_cache_active_cells_dispatch[1u], 1u);
    atomicStore(&world_cache_active_cells_dispatch[2u], 1u);
}

@compute @workgroup_size(1, 1, 1)
fn clear_world_cache_active_cells() {
    atomicStore(&world_cache_active_cells_count, 0u);
    atomicStore(&world_cache_active_cells_dispatch[0u], 0u);
    atomicStore(&world_cache_active_cells_dispatch[1u], 1u);
    atomicStore(&world_cache_active_cells_dispatch[2u], 1u);
}
