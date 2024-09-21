use struct_patch::Patch;

#[derive(Debug, Clone, Copy, Patch)]
#[patch(attribute(derive(Debug, Default, Clone)))]
pub struct SamplerHint {
    pub filter: wgpu::FilterMode,
    pub wrap: wgpu::AddressMode,
    pub comparison: Option<wgpu::CompareFunction>,
}

impl Default for SamplerHint {
    fn default() -> Self {
        Self {
            // TODO:  Document this behavior somewhere
            filter: wgpu::FilterMode::Nearest,
            wrap: wgpu::AddressMode::ClampToEdge,
            comparison: None,
        }
    }
}
