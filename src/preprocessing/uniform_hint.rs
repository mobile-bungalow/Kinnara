use struct_patch::Patch;

#[derive(Debug, Clone, Copy, Patch)]
#[patch(attribute(derive(Debug, Default, Clone)))]
#[derive(Default)]
pub struct UniformHint {
    pub dynamic_offset: bool,
    pub calculate_min_binding_size: bool,
}

