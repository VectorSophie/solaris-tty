//! Rendering quality configuration system

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderQuality {
    Basic,
    Standard,
    Advanced,
}

#[derive(Clone, Copy, Debug)]
pub struct QualityParams {
    pub star_count: usize,
    pub ring_segments: usize,
    pub old_trail_stride: usize,
    pub motion_tick_count: usize,
    pub ambient_light: f32,
    pub surface_detail: SurfaceDetail,
    pub hard_eclipses: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceDetail {
    None,
    Broad,
    Detailed,
}

impl RenderQuality {
    pub fn params(&self) -> QualityParams {
        match self {
            RenderQuality::Basic => QualityParams {
                star_count: 200,
                ring_segments: 48,
                old_trail_stride: 4,
                motion_tick_count: 0,
                ambient_light: 0.16,
                surface_detail: SurfaceDetail::None,
                hard_eclipses: false,
            },
            RenderQuality::Standard => QualityParams {
                star_count: 500,
                ring_segments: 96,
                old_trail_stride: 2,
                motion_tick_count: 8,
                ambient_light: 0.10,
                surface_detail: SurfaceDetail::Broad,
                hard_eclipses: false,
            },
            RenderQuality::Advanced => QualityParams {
                star_count: 900,
                ring_segments: 192,
                old_trail_stride: 1,
                motion_tick_count: 12,
                ambient_light: 0.06,
                surface_detail: SurfaceDetail::Detailed,
                hard_eclipses: true,
            },
        }
    }
}
