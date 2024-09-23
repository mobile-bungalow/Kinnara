mod bind_group;
mod preprocessing;
mod wgpu_utils;

use bind_group::BindGroups;
use thiserror::Error;
use wgpu::{
    naga::front::{self, glsl::ParseErrors as GlslParseError, wgsl::ParseError as WgslParseError},
    BindGroupLayoutDescriptor, ComputePipeline, ComputePipelineDescriptor, ErrorFilter,
    PipelineCache, ShaderModuleDescriptor, ShaderSource,
};
pub use wgpu_utils::DeviceUtils;

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
/// utilities on a given shader, derived from it's source.
pub struct ComputeReflectionContext {
    bind_groups: BindGroups,
    naga_mod: wgpu::naga::Module,
    pub build_cache: Option<PipelineCache>,
}

// TODO: Add Pixel reflection context
impl ComputeReflectionContext {
    pub fn new_compute(source: wgpu::ShaderSource) -> Result<Self, Error> {
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

        Ok(Self {
            bind_groups,
            naga_mod,
            build_cache: None,
        })
    }

    pub fn work_group_size(&self, entry_point: &str) -> Option<[u32; 3]> {
        self.bind_groups.work_group_size(entry_point)
    }

    pub fn entry_points(&self) -> impl Iterator<Item = &String> {
        self.bind_groups.entry_points()
    }

    pub fn push_constant_range(&self) -> Option<wgpu::PushConstantRange> {
        self.bind_groups.push_constant_range.clone()
    }

    pub fn create_compute_pipeline(
        &mut self,
        device: &wgpu::Device,
        entry_point: &str,
        options: wgpu::PipelineCompilationOptions,
    ) -> Result<ComputePipeline, Error> {
        let module_desc = ShaderModuleDescriptor {
            label: None,
            source: ShaderSource::Naga(std::borrow::Cow::Owned(self.naga_mod.clone())),
        };

        let is_cache_capable = device.features().contains(wgpu::Features::PIPELINE_CACHE);

        if is_cache_capable && self.build_cache.is_none() {
            let desc = wgpu::PipelineCacheDescriptor {
                label: None,
                data: None,
                fallback: true,
            };
            unsafe {
                self.build_cache = Some(device.create_pipeline_cache(&desc));
            }
        }

        let layout = self.create_pipeline_layout(device);

        device
            .wgpu_try(ErrorFilter::Validation, |dev| {
                let module = dev.create_shader_module(module_desc);
                let pipeline_desc = ComputePipelineDescriptor {
                    label: None,
                    layout: Some(&layout),
                    module: &module,
                    entry_point,
                    compilation_options: options,
                    cache: self.build_cache.as_ref(),
                };

                dev.create_compute_pipeline(&pipeline_desc)
            })
            .map_err(Error::from)
    }

    pub fn create_pipeline_layout(&self, device: &wgpu::Device) -> wgpu::PipelineLayout {
        let push_constant_range = &self
            .push_constant_range()
            .map_or(vec![], |r| vec![r.clone()]);

        let bind_groups_ct = self.bind_groups.bind_group_count();
        let bind_group_layouts: Vec<_> = (0..=bind_groups_ct)
            .map(|set| self.create_bind_group_layout(device, set as u32))
            .collect();

        let bind_group_layouts: Vec<_> = bind_group_layouts.iter().collect();

        let desc = wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: bind_group_layouts.as_slice(),
            push_constant_ranges: push_constant_range,
        };

        device.create_pipeline_layout(&desc)
    }

    pub fn bind_group_count(&self) -> usize {
        self.bind_groups.bind_group_count()
    }

    pub fn create_bind_group_layout(
        &self,
        device: &wgpu::Device,
        set: u32,
    ) -> wgpu::BindGroupLayout {
        let layout_desc = &self.get_bind_group_layout_descriptor(set);
        device.create_bind_group_layout(layout_desc)
    }

    pub fn get_bind_group_layout_descriptor(&self, set: u32) -> BindGroupLayoutDescriptor<'_> {
        let entries = self.bind_groups.get_bind_group_layout_entries(set);
        BindGroupLayoutDescriptor {
            label: None,
            entries,
        }
    }

    pub fn bind_group_entries_count(&self, set: u32) -> usize {
        self.bind_groups.bind_group_entries_count(set)
    }

    pub fn get_bind_group_layout_entry(
        &self,
        set: u32,
        binding: u32,
    ) -> Option<wgpu::BindGroupLayoutEntry> {
        self.bind_groups.get_bind_group_layout_entry(set, binding)
    }
}
