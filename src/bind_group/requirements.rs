use std::num::NonZeroU32;

use wgpu::{BindGroupLayoutEntry, BindingResource};

// TODO: make these entries macros,
#[derive(Debug)]
pub enum BindSlot<'a> {
    StorageBuffer {
        binding: u32,
        slot: Option<wgpu::BufferBinding<'a>>,
    },
    UniformBuffer {
        binding: u32,
        slot: Option<wgpu::BufferBinding<'a>>,
    },
    StorageBufferArray {
        binding: u32,
        slots: Option<&'a [wgpu::BufferBinding<'a>]>,
        entry_count: u32,
    },
    UniformBufferArray {
        binding: u32,
        slots: Option<&'a [wgpu::BufferBinding<'a>]>,
        entry_count: u32,
    },
    Texture {
        binding: u32,
        slot: Option<&'a wgpu::TextureView>,
    },
    TextureArray {
        binding: u32,
        slots: Option<&'a [&'a wgpu::TextureView]>,
        entry_count: u32,
    },
    Sampler {
        binding: u32,
        slot: Option<&'a wgpu::Sampler>,
    },
    SamplerArray {
        binding: u32,
        slots: Option<&'a [&'a wgpu::Sampler]>,
        entry_count: u32,
    },
}

macro_rules! create_bind_slot {
    ($fn_name:ident, $single:ident, $array:ident) => {
        #[inline(always)]
        fn $fn_name(set: u32, ct: &Option<NonZeroU32>) -> Self {
            match ct {
                Some(ct) => Self::$array {
                    binding: set,
                    slots: None,
                    entry_count: ct.get(),
                },
                None => Self::$single {
                    binding: set,
                    slot: None,
                },
            }
        }
    };
}

impl<'a> BindSlot<'a> {
    pub fn from_entry(entry: &BindGroupLayoutEntry) -> Self {
        let BindGroupLayoutEntry {
            binding, ty, count, ..
        } = entry;

        match ty {
            wgpu::BindingType::Buffer { ty, .. } => match ty {
                wgpu::BufferBindingType::Uniform => Self::uniform_buf(*binding, count),
                wgpu::BufferBindingType::Storage { .. } => Self::storage_buf(*binding, count),
            },
            wgpu::BindingType::Sampler(_) => Self::sampler(*binding, count),
            wgpu::BindingType::Texture { .. } => Self::texture(*binding, count),
            wgpu::BindingType::StorageTexture { .. } => Self::texture(*binding, count),
            wgpu::BindingType::AccelerationStructure => {
                todo!("I'm not sure if these are widely supported.")
            }
        }
    }

    create_bind_slot!(uniform_buf, UniformBuffer, UniformBufferArray);
    create_bind_slot!(storage_buf, StorageBuffer, StorageBufferArray);
    create_bind_slot!(sampler, Sampler, SamplerArray);
    create_bind_slot!(texture, Texture, TextureArray);

    pub fn binding(&self) -> u32 {
        match self {
            Self::StorageBuffer { binding, .. }
            | Self::UniformBuffer { binding, .. }
            | Self::Texture { binding, .. }
            | Self::Sampler { binding, .. }
            | Self::StorageBufferArray { binding, .. }
            | Self::UniformBufferArray { binding, .. }
            | Self::TextureArray { binding, .. }
            | Self::SamplerArray { binding, .. } => *binding,
        }
    }

    pub fn is_some(&self) -> bool {
        match self {
            Self::StorageBuffer { slot, .. } | Self::UniformBuffer { slot, .. } => slot.is_some(),
            Self::Texture { slot, .. } => slot.is_some(),
            Self::Sampler { slot, .. } => slot.is_some(),
            Self::StorageBufferArray { slots, .. } | Self::UniformBufferArray { slots, .. } => {
                slots.is_some()
            }
            Self::TextureArray { slots, .. } => slots.is_some(),
            Self::SamplerArray { slots, .. } => slots.is_some(),
        }
    }
}

impl<'a> TryFrom<BindSlot<'a>> for BindingResource<'a> {
    type Error = super::BindGroupError;

    fn try_from(value: BindSlot<'a>) -> Result<Self, Self::Error> {
        macro_rules! get_reqt {
            ($slot:expr, $binding:expr, $constructor:expr) => {
                match $slot {
                    Some(s) => Ok($constructor(s)),
                    None => Err(Self::Error::MissingBindGroupEntry($binding)),
                }
            };
        }

        match value {
            BindSlot::StorageBuffer { binding, slot }
            | BindSlot::UniformBuffer { binding, slot } => {
                get_reqt!(slot, binding, BindingResource::Buffer)
            }
            BindSlot::Texture { binding, slot } => {
                get_reqt!(slot, binding, BindingResource::TextureView)
            }
            BindSlot::Sampler { binding, slot } => {
                get_reqt!(slot, binding, BindingResource::Sampler)
            }
            BindSlot::StorageBufferArray { binding, slots, .. }
            | BindSlot::UniformBufferArray { binding, slots, .. } => {
                get_reqt!(slots, binding, BindingResource::BufferArray)
            }
            BindSlot::TextureArray { binding, slots, .. } => {
                get_reqt!(slots, binding, BindingResource::TextureViewArray)
            }
            BindSlot::SamplerArray { binding, slots, .. } => {
                get_reqt!(slots, binding, BindingResource::SamplerArray)
            }
        }
    }
}
