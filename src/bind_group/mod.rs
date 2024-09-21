mod naga_utils;

use crate::preprocessing::{Directives, UniformHint};
use thiserror::Error;
use wgpu::{
    naga::{self, AddressSpace, FastHashMap, ResourceBinding, StorageAccess, TypeInner},
    BindGroupLayoutEntry, BindingType, BufferBindingType, PushConstantRange, ShaderStages,
};

pub struct BindingInfo {
    pub entry: BindGroupLayoutEntry,
    pub binding: naga::ResourceBinding,
    pub name: Option<String>,
}

struct EntryMetaData {
    set_idx: usize,
    entry_idx: usize,
    name: Option<String>,
}

pub struct BindGroups {
    entry_map: FastHashMap<naga::ResourceBinding, EntryMetaData>,
    bindings: Vec<Vec<wgpu::BindGroupLayoutEntry>>,
    pub push_constant_range: Option<wgpu::PushConstantRange>,
}

#[derive(Debug, Error)]
pub enum BindGroupError {
    #[error("Wrong or ambiguous stage in module")]
    Stage,
    #[error("Invalid or unsupported global type {0}")]
    GlobalType(&'static str),
    #[error("Should never occur: type {0} was in unform address space.")]
    UniformAddressMismatch(&'static str),
    #[error("Missing Type Handle")]
    MissingTypeHandle,
    #[error("Unexpected Type for address space")]
    UnexpectedType,
    #[error("No entry point found in module.")]
    NoEntryPoint,
    #[error("Multiple Entry points in module")]
    TooManyEntryPoints,
}

impl BindGroups {
    pub fn new(module: &naga::Module, directives: &Directives) -> Result<Self, BindGroupError> {
        let stage = match module.entry_points.as_slice() {
            [single_ep] => single_ep.stage,
            [] => return Err(BindGroupError::NoEntryPoint),
            [..] => return Err(BindGroupError::TooManyEntryPoints),
        };

        // TODO: in the future, we should try to infer visibility from usage
        // and hints, for now it will be all or nothing
        let visibility = match stage {
            naga::ShaderStage::Vertex => ShaderStages::VERTEX_FRAGMENT,
            naga::ShaderStage::Fragment => ShaderStages::VERTEX_FRAGMENT,
            naga::ShaderStage::Compute => ShaderStages::COMPUTE,
        };

        let mut entry_map = FastHashMap::default();
        let mut bindings = Vec::new();
        let mut push_constant_range = None;

        for (_, global) in module.global_variables.iter() {
            match GlobalVar::process_global_var(directives, module, global, visibility)? {
                Some(GlobalVar::PushConstant(pc)) => push_constant_range = Some(pc),
                Some(GlobalVar::Uniform(uniform)) => {
                    update_entry_map(uniform, &mut bindings, &mut entry_map)
                }
                None => {}
            }
        }

        Ok(Self {
            bindings,
            entry_map,
            push_constant_range,
        })
    }

    pub fn get_bind_group_layout_entry_vector(
        &self,
        set: u32,
    ) -> &[wgpu::BindGroupLayoutEntry] {
        self.bindings
            .get(set as usize)
            .map_or(&[], |e| e.as_slice())
    }

    pub fn get_bind_group_layout_entry(
        &self,
        set: u32,
        binding: u32,
    ) -> Option<wgpu::BindGroupLayoutEntry> {
        let binding = ResourceBinding {
            group: set,
            binding,
        };

        self.entry_map
            .get(&binding)
            .map(|meta_data| self.bindings[meta_data.set_idx][meta_data.entry_idx])
    }
}

enum GlobalVar {
    PushConstant(PushConstantRange),
    Uniform(BindingInfo),
}

impl GlobalVar {
    pub fn process_global_var(
        directives: &Directives,
        module: &naga::Module,
        global: &wgpu::naga::GlobalVariable,
        visibility: ShaderStages,
    ) -> Result<Option<Self>, BindGroupError> {
        match global.space {
            AddressSpace::Function | AddressSpace::Private | AddressSpace::WorkGroup => Ok(None),
            AddressSpace::Uniform => {
                let Some(binding) = global.binding.as_ref() else {
                    return Ok(None);
                };

                let uniform_hint = directives.get_uniform_hint(binding);
                let binding = infer_uniform_binding_type(module, global, &uniform_hint);
                Ok(Self::new_uniform(binding, global, module, visibility))
            }
            AddressSpace::Storage { access } => {
                let Some(binding) = global.binding.as_ref() else {
                    return Ok(None);
                };

                let uniform_hint = directives.get_uniform_hint(binding);
                let binding = infer_storage_binding_type(access, uniform_hint, module, &global.ty)?;
                Ok(Self::new_uniform(binding, global, module, visibility))
            }
            AddressSpace::Handle => {
                let Some(binding) = global.binding.as_ref() else {
                    return Ok(None);
                };

                let binding = infer_handle_binding_type(directives, binding, module, &global.ty)?;
                Ok(Self::new_uniform(binding, global, module, visibility))
            }
            AddressSpace::PushConstant => {
                Ok(Some(push_constant_ranges(visibility, module, &global.ty)?))
            }
        }
    }

    pub fn new_uniform(
        ty: BindingType,
        global: &naga::GlobalVariable,
        module: &naga::Module,
        visibility: ShaderStages,
    ) -> Option<Self> {
        let count = naga_utils::type_array_ct(module, &global.ty);

        if let Some(binding) = global.binding.clone() {
            let entry = BindGroupLayoutEntry {
                binding: binding.binding,
                visibility,
                ty,
                count,
            };
            Some(Self::Uniform(BindingInfo {
                entry,
                binding,
                name: global.name.clone(),
            }))
        } else {
            None
        }
    }
}

fn update_entry_map(
    info: BindingInfo,
    bindings: &mut Vec<Vec<wgpu::BindGroupLayoutEntry>>,
    map: &mut FastHashMap<ResourceBinding, EntryMetaData>,
) {
    let BindingInfo {
        entry,
        binding,
        name,
    } = info;

    let needed_len = (binding.group + 1) as usize;

    if bindings.len() < needed_len {
        bindings.extend(vec![vec![]; needed_len - bindings.len()])
    }

    bindings[binding.group as usize].push(entry);
    let entry_idx = bindings[binding.group as usize].len() - 1;

    map.insert(
        binding.clone(),
        EntryMetaData {
            set_idx: binding.group as _,
            entry_idx,
            name,
        },
    );
}

fn infer_uniform_binding_type(
    module: &naga::Module,
    global: &naga::GlobalVariable,
    uniform_hint: &UniformHint,
) -> BindingType {
    let min_binding_size = if uniform_hint.calculate_min_binding_size {
        naga_utils::type_size(module, global.ty)
    } else {
        None
    };

    let type_actual = module.types.get_handle(global.ty).ok();

    let is_acc_struct = matches!(
        type_actual.map(|t| t.inner.clone()),
        Some(TypeInner::AccelerationStructure)
    );

    if is_acc_struct {
        BindingType::AccelerationStructure
    } else {
        BindingType::Buffer {
            ty: BufferBindingType::Uniform,
            has_dynamic_offset: uniform_hint.dynamic_offset,
            min_binding_size,
        }
    }
}

/// For a given global storage
/// uniform infer binding type
fn infer_storage_binding_type(
    access: StorageAccess,
    uniform_hint: UniformHint,
    module: &naga::Module,
    ty: &naga::Handle<naga::Type>,
) -> Result<BindingType, BindGroupError> {
    let is_image =
        naga_utils::is_image(module, *ty).map_err(|_| BindGroupError::MissingTypeHandle)?;

    if is_image {
        let access = naga_utils::storage_access(&access);
        let (fmt, view_dimension) = naga_utils::get_image_information(module, *ty)
            .ok_or(BindGroupError::MissingTypeHandle)?;

        Ok(BindingType::StorageTexture {
            access,
            format: fmt,
            view_dimension,
        })
    } else {
        let min_binding_size = if uniform_hint.calculate_min_binding_size {
            naga_utils::type_size(module, *ty)
        } else {
            None
        };

        Ok(BindingType::Buffer {
            ty: BufferBindingType::Storage {
                read_only: access == naga::StorageAccess::LOAD,
            },
            has_dynamic_offset: uniform_hint.dynamic_offset,
            min_binding_size,
        })
    }
}

fn infer_handle_binding_type(
    directives: &Directives,
    binding: &naga::ResourceBinding,
    module: &naga::Module,
    ty: &naga::Handle<naga::Type>,
) -> Result<BindingType, BindGroupError> {
    let type_actual = module
        .types
        .get_handle(*ty)
        .map_err(|_| BindGroupError::MissingTypeHandle)?;

    match type_actual.inner {
        TypeInner::Sampler { comparison } => {
            let sampler_hint = directives.get_sampler_hint(binding);
            if comparison {
                Ok(BindingType::Sampler(wgpu::SamplerBindingType::Comparison))
            } else {
                match sampler_hint.filter {
                    wgpu::FilterMode::Nearest => {
                        Ok(BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering))
                    }
                    wgpu::FilterMode::Linear => {
                        Ok(BindingType::Sampler(wgpu::SamplerBindingType::Filtering))
                    }
                }
            }
        }
        TypeInner::Image { dim, class, .. } => Ok(BindingType::Texture {
            view_dimension: naga_utils::image_dim(&dim),
            sample_type: naga_utils::sample_kind(&class),
            multisampled: matches!(class, naga::ImageClass::Sampled { multi: true, .. }),
        }),
        TypeInner::BindingArray { base, .. } => {
            infer_handle_binding_type(directives, binding, module, &base)
        }
        _ => Err(BindGroupError::UnexpectedType),
    }
}

fn push_constant_ranges(
    stages: ShaderStages,
    module: &naga::Module,
    ty: &naga::Handle<naga::Type>,
) -> Result<GlobalVar, BindGroupError> {
    let type_actual = module
        .types
        .get_handle(*ty)
        .map_err(|_| BindGroupError::MissingTypeHandle)?;

    let size = type_actual.inner.size(module.to_ctx()).next_multiple_of(4);

    Ok(GlobalVar::PushConstant(PushConstantRange {
        stages,
        range: 0..size,
    }))
}
