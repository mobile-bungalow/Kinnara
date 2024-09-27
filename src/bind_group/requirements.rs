use std::num::NonZeroU32;

use std::cell::RefCell;
use wgpu::{BindGroupLayoutEntry, BindingResource, PushConstantRange};

pub enum PassSlot<'a> {
    DynamicOffset {
        loc: (u32, u32),
        offset: RefCell<Option<u32>>,
    },
    PushConstantRange {
        visibility: wgpu::ShaderStages,
        range: std::ops::Range<u32>,
        buffer: RefCell<Option<&'a [u8]>>,
    },
}

impl<'a> From<&PushConstantRange> for PassSlot<'a> {
    fn from(value: &PushConstantRange) -> Self {
        Self::PushConstantRange {
            visibility: value.stages,
            range: value.range.clone(),
            buffer: None.into(),
        }
    }
}

impl<'a> PassSlot<'a> {
    // this may need offsets later, unlikely.
    // the use case for push constants is %99.9
    // all at once.
    pub fn push_const_slice(self) -> Option<(u32, &'a [u8])> {
        match self {
            PassSlot::PushConstantRange { buffer, range, .. } => {
                Some((range.start, buffer.take()?))
            }
            _ => None,
        }
    }

    pub fn offset_for(set: u32, binding: u32) -> Self {
        Self::DynamicOffset {
            loc: (set, binding),
            offset: None.into(),
        }
    }

    pub fn offset(self) -> Option<u32> {
        match self {
            PassSlot::DynamicOffset { offset, .. } => offset.take(),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum BindSlot<'a> {
    StorageBuffer {
        loc: (u32, u32),
        slot: RefCell<Option<wgpu::BufferBinding<'a>>>,
    },
    UniformBuffer {
        loc: (u32, u32),
        slot: RefCell<Option<wgpu::BufferBinding<'a>>>,
    },
    StorageBufferArray {
        loc: (u32, u32),
        slots: RefCell<Option<&'a [wgpu::BufferBinding<'a>]>>,
        entry_count: u32,
    },
    UniformBufferArray {
        loc: (u32, u32),
        slots: RefCell<Option<&'a [wgpu::BufferBinding<'a>]>>,
        entry_count: u32,
    },
    Texture {
        loc: (u32, u32),
        slot: RefCell<Option<&'a wgpu::TextureView>>,
    },
    TextureArray {
        loc: (u32, u32),
        slots: RefCell<Option<&'a [&'a wgpu::TextureView]>>,
        entry_count: u32,
    },
    Sampler {
        loc: (u32, u32),
        slot: RefCell<Option<&'a wgpu::Sampler>>,
    },
    SamplerArray {
        loc: (u32, u32),
        slots: RefCell<Option<&'a [&'a wgpu::Sampler]>>,
        entry_count: u32,
    },
}

macro_rules! create_bind_slot {
    ($fn_name:ident, $single:ident, $array:ident) => {
        #[inline(always)]
        fn $fn_name(set: u32, binding: u32, ct: &Option<NonZeroU32>) -> Self {
            match ct {
                Some(ct) => Self::$array {
                    loc: (set, binding),
                    slots: None.into(),
                    entry_count: ct.get(),
                },
                None => Self::$single {
                    loc: (set, binding),
                    slot: None.into(),
                },
            }
        }
    };
}

impl<'a> BindSlot<'a> {
    pub fn from_entry(set: u32, entry: &BindGroupLayoutEntry) -> Self {
        let BindGroupLayoutEntry {
            binding, ty, count, ..
        } = entry;

        match ty {
            wgpu::BindingType::Buffer { ty, .. } => match ty {
                wgpu::BufferBindingType::Uniform => Self::uniform_buf(set, *binding, count),
                wgpu::BufferBindingType::Storage { .. } => Self::storage_buf(set, *binding, count),
            },
            wgpu::BindingType::Sampler(_) => Self::sampler(set, *binding, count),
            wgpu::BindingType::Texture { .. } => Self::texture(set, *binding, count),
            wgpu::BindingType::StorageTexture { .. } => Self::texture(set, *binding, count),
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
            Self::StorageBuffer { loc, .. }
            | Self::UniformBuffer { loc, .. }
            | Self::Texture { loc, .. }
            | Self::Sampler { loc, .. }
            | Self::StorageBufferArray { loc, .. }
            | Self::UniformBufferArray { loc, .. }
            | Self::TextureArray { loc, .. }
            | Self::SamplerArray { loc, .. } => loc.1,
        }
    }

    pub fn is_some(&self) -> bool {
        match self {
            Self::StorageBuffer { slot, .. } | Self::UniformBuffer { slot, .. } => {
                slot.borrow().is_some()
            }
            Self::Texture { slot, .. } => slot.borrow().is_some(),
            Self::Sampler { slot, .. } => slot.borrow().is_some(),
            Self::StorageBufferArray { slots, .. } | Self::UniformBufferArray { slots, .. } => {
                slots.borrow().is_some()
            }
            Self::TextureArray { slots, .. } => slots.borrow().is_some(),
            Self::SamplerArray { slots, .. } => slots.borrow().is_some(),
        }
    }
}

impl<'a> TryFrom<BindSlot<'a>> for BindingResource<'a> {
    type Error = super::BindGroupError;

    fn try_from(value: BindSlot<'a>) -> Result<Self, Self::Error> {
        macro_rules! get_reqt {
            ($slot:expr, $binding:expr, $constructor:expr) => {
                match $slot.borrow_mut().take() {
                    Some(s) => Ok($constructor(s)),
                    None => Err(Self::Error::MissingBindGroupEntry($binding.0, $binding.1)),
                }
            };
        }

        match value {
            BindSlot::StorageBuffer { loc, slot, .. }
            | BindSlot::UniformBuffer { loc, slot, .. } => {
                get_reqt!(slot, loc, BindingResource::Buffer)
            }
            BindSlot::Texture { loc, slot, .. } => {
                get_reqt!(slot, loc, BindingResource::TextureView)
            }
            BindSlot::Sampler { loc, slot, .. } => {
                get_reqt!(slot, loc, BindingResource::Sampler)
            }
            BindSlot::StorageBufferArray { loc, slots, .. }
            | BindSlot::UniformBufferArray { loc, slots, .. } => {
                get_reqt!(slots, loc, BindingResource::BufferArray)
            }
            BindSlot::TextureArray { loc, slots, .. } => {
                get_reqt!(slots, loc, BindingResource::TextureViewArray)
            }
            BindSlot::SamplerArray { loc, slots, .. } => {
                get_reqt!(slots, loc, BindingResource::SamplerArray)
            }
        }
    }
}
