use alloc::vec::Vec;

use bevy_ecs::resource::Resource;
use bevy_math::{UVec4, Vec3};

/// Optional Mirage-facing receiver export for cloud shadow and sky relevance.
///
/// This deliberately describes visible receiver bounds, not meshlet internals. Meshlet and
/// non-meshlet render paths can populate the same resource without exposing private renderer data.
#[derive(Resource, Default, Debug, Clone)]
pub struct VisibleReceiverBoundsBuffer {
    pub frame_index: u64,
    pub receivers: Vec<VisibleReceiverBounds>,
    pub source: VisibleReceiverSource,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VisibleReceiverSource {
    #[default]
    None,
    MeshletVisibility,
    OrdinaryMeshVisibility,
    DepthClassification,
}

#[derive(Default, Debug, Clone, Copy)]
pub struct VisibleReceiverBounds {
    pub center: Vec3,
    pub radius_m: f32,
    pub half_extents: Vec3,
    pub lod_level: u32,
    pub screen_tile_rect: UVec4,
    pub receiver_flags: u32,
    pub entity_bits: u64,
    pub material_bits: u64,
}
