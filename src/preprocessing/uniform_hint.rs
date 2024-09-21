use struct_patch::Patch;

#[derive(Debug, Clone, Copy, Patch)]
#[patch(attribute(derive(Debug, Default, Clone)))]
pub struct UniformHint {
    pub dynamic_offset: bool,
    pub calculate_min_binding_size: bool,
}

impl Default for UniformHint {
    fn default() -> Self {
        Self {
            dynamic_offset: false,
            calculate_min_binding_size: false,
        }
    }
}
