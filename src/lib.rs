mod bind_group;
mod device_utils;
mod preprocessing;

use bind_group::BindGroups;
use thiserror::Error;
use wgpu::{
    naga::front::{self, glsl::ParseErrors as GlslParseError, wgsl::ParseError as WgslParseError},
    BindGroupLayoutDescriptor,
};

#[derive(Error, Debug)]
pub enum Error {
    #[error("Wgpu: Out of Memory")]
    Oom,
    #[error("Wgpu Validation Error: {0}")]
    Validation(String),
    #[error("Wgpu internal error: {0}")]
    Wgpu(String),
    #[error("Unsupported shader source type. Pass either kinnara enriched wgsl or glsl.")]
    UnsupportedSourceType,
    #[error("Shader compilation error: {0}")]
    WgslCompilationError(#[from] WgslParseError),
    #[error("Shader compilation error: {0}")]
    GlslCompilationError(#[from] GlslParseError),
    #[error("Bind Group Error: {0}")]
    BindGroupError(#[from] bind_group::BindGroupError),
    #[error("Preprocessing Error : {0}")]
    PreprocessingError(#[from] preprocessing::PreprocessingError),
}

impl From<wgpu::Error> for Error {
    fn from(value: wgpu::Error) -> Self {
        match value {
            wgpu::Error::OutOfMemory { .. } => Error::Oom,
            wgpu::Error::Validation { description, .. } => Error::Validation(description),
            wgpu::Error::Internal { description, .. } => Error::Wgpu(description),
        }
    }
}

/// A structure holding user enriched reflection info and book keeping
/// utilities on a given Shader, derived from it's source.
pub struct ReflectionContext {
    pub bind_groups: BindGroups,
}

impl ReflectionContext {
    //pub fn new_pixel_reflector(frag: &wgpu::naga::Module, vert: &wgpu::naga::Module) -> Result<Self, Error> {
    //    Ok(Self {})
    //}

    pub fn new_compute_reflector(source: wgpu::ShaderSource) -> Result<Self, Error> {
        let (directives, modified_source) = preprocessing::process(&source)?;

        let naga_mod = match modified_source {
            #[cfg(feature = "wgsl")]
            wgpu::ShaderSource::Wgsl(src) => {
                let mut parser = front::wgsl::Frontend::new();
                parser.parse(&src)?
            }
            #[cfg(feature = "glsl")]
            wgpu::ShaderSource::Glsl {
                shader,
                stage,
                defines,
            } => {
                let mut options = front::glsl::Options::from(stage);
                options.defines = defines;
                let mut parser = front::glsl::Frontend::default();
                parser.parse(&options, &shader)?
            }
            _ => return Err(Error::UnsupportedSourceType),
        };

        let bind_groups = BindGroups::new(&naga_mod, &directives)?;

        Ok(Self { bind_groups })
    }

    pub fn push_constant_range(&self) -> Option<wgpu::PushConstantRange> {
        self.bind_groups.push_constant_range.clone()
    }

    pub fn create_bind_group_layout(
        &self,
        device: &wgpu::Device,
        set: u32,
    ) -> Option<wgpu::BindGroup> {
        todo!();
    }

    pub fn get_bind_group_layout_descriptor(
        &self,
        set: u32,
    ) -> BindGroupLayoutDescriptor<'_> {
        let entries = self.bind_groups.get_bind_group_layout_entry_vector(set);
        BindGroupLayoutDescriptor {
            label: None,
            entries,
        }
    }

    pub fn get_bind_group_layout_entry(
        &self,
        set: u32,
        binding: u32,
    ) -> Option<wgpu::BindGroupLayoutEntry> {
        self.bind_groups.get_bind_group_layout_entry(set, binding)
    }
}
