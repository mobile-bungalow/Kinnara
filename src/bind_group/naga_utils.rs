use thiserror::Error;
use wgpu::naga::{self, *};

#[derive(Error, Debug)]
pub enum Error {
    #[error("No such handle found in module.")]
    NoSuchHandle,
}

/// resolve the minimum size of a type
pub fn type_size(module: &Module, ty: Handle<Type>) -> Option<std::num::NonZeroU64> {
    let type_actual = module.types.get_handle(ty).ok()?;
    let size = type_actual.inner.size(module.to_ctx());
    std::num::NonZeroU64::new(size as u64)
}

/// Get array count of a type
pub fn type_array_ct(module: &Module, ty: &Handle<Type>) -> Option<std::num::NonZeroU32> {
    let type_actual = module.types.get_handle(*ty).ok()?;
    if let TypeInner::BindingArray { size, .. } = type_actual.inner {
        match size {
            ArraySize::Constant(non_zero) => Some(non_zero),
            ArraySize::Dynamic => std::num::NonZeroU32::new(1),
        }
    } else {
        None
    }
}

pub fn get_image_information(
    module: &Module,
    ty: Handle<Type>,
) -> Option<(wgpu::TextureFormat, wgpu::TextureViewDimension)> {
    let type_actual = module
        .types
        .get_handle(ty)
        .map_err(|_| Error::NoSuchHandle)
        .ok()?;

    match type_actual.inner {
        TypeInner::Image {
            class: ImageClass::Storage { format, .. },
            dim,
            ..
        } => Some((texture_fmt(&format), image_dim(&dim))),
        TypeInner::BindingArray { base, .. } => get_image_information(module, base),
        _ => None,
    }
}

pub fn is_image(module: &Module, ty: Handle<Type>) -> Result<bool, Error> {
    let type_actual = module
        .types
        .get_handle(ty)
        .map_err(|_| Error::NoSuchHandle)?;

    let out = match type_actual.inner {
        TypeInner::Image { .. } => true,
        TypeInner::BindingArray { base, .. } => is_image(module, base)?,
        _ => false,
    };

    Ok(out)
}

pub fn storage_access(access: &naga::StorageAccess) -> wgpu::StorageTextureAccess {
    let r = access.contains(StorageAccess::LOAD);
    let w = access.contains(StorageAccess::STORE);
    match (r, w) {
        (true, true) => wgpu::StorageTextureAccess::ReadWrite,
        (false, true) => wgpu::StorageTextureAccess::WriteOnly,
        (false | true, false) => wgpu::StorageTextureAccess::ReadOnly,
    }
}

pub fn sample_kind(kind: &naga::ScalarKind) -> wgpu::TextureSampleType {
    match kind {
        naga::ScalarKind::Sint => wgpu::TextureSampleType::Sint,
        naga::ScalarKind::Uint => wgpu::TextureSampleType::Uint,
        // TODO:error out here, f32 textures shouldn't be filterable
        naga::ScalarKind::Float => wgpu::TextureSampleType::Float { filterable: true },
        naga::ScalarKind::Bool => wgpu::TextureSampleType::Uint,
        ScalarKind::AbstractFloat | ScalarKind::AbstractInt => unreachable!(),
    }
}

pub fn image_dim(dim: &naga::ImageDimension) -> wgpu::TextureViewDimension {
    match dim {
        ImageDimension::D1 => wgpu::TextureViewDimension::D1,
        ImageDimension::D2 => wgpu::TextureViewDimension::D2,
        ImageDimension::D3 => wgpu::TextureViewDimension::D3,
        ImageDimension::Cube => wgpu::TextureViewDimension::Cube,
    }
}

pub fn texture_fmt(fmt: &naga::StorageFormat) -> wgpu::TextureFormat {
    match fmt {
        naga::StorageFormat::R8Unorm => wgpu::TextureFormat::R8Unorm,
        naga::StorageFormat::R8Snorm => wgpu::TextureFormat::R8Snorm,
        naga::StorageFormat::R8Uint => wgpu::TextureFormat::R8Uint,
        naga::StorageFormat::R8Sint => wgpu::TextureFormat::R8Sint,
        naga::StorageFormat::R16Uint => wgpu::TextureFormat::R16Uint,
        naga::StorageFormat::R16Sint => wgpu::TextureFormat::R16Sint,
        naga::StorageFormat::R16Float => wgpu::TextureFormat::R16Float,
        naga::StorageFormat::Rg8Unorm => wgpu::TextureFormat::Rg8Unorm,
        naga::StorageFormat::Rg8Snorm => wgpu::TextureFormat::Rg8Snorm,
        naga::StorageFormat::Rg8Uint => wgpu::TextureFormat::Rg8Uint,
        naga::StorageFormat::Rg8Sint => wgpu::TextureFormat::Rg8Sint,
        naga::StorageFormat::R32Uint => wgpu::TextureFormat::R32Uint,
        naga::StorageFormat::R32Sint => wgpu::TextureFormat::R32Sint,
        naga::StorageFormat::R32Float => wgpu::TextureFormat::R32Float,
        naga::StorageFormat::Rg16Uint => wgpu::TextureFormat::Rg16Uint,
        naga::StorageFormat::Rg16Sint => wgpu::TextureFormat::Rg16Sint,
        naga::StorageFormat::Rg16Float => wgpu::TextureFormat::Rg16Float,
        naga::StorageFormat::Rgba8Unorm => wgpu::TextureFormat::Rgba8Unorm,
        naga::StorageFormat::Rgba8Snorm => wgpu::TextureFormat::Rgba8Snorm,
        naga::StorageFormat::Rgba8Uint => wgpu::TextureFormat::Rgba8Uint,
        naga::StorageFormat::Rgba8Sint => wgpu::TextureFormat::Rgba8Sint,
        naga::StorageFormat::Rgb10a2Unorm => wgpu::TextureFormat::Rgb10a2Unorm,
        naga::StorageFormat::Rg11b10Float => wgpu::TextureFormat::Rg11b10Float,
        naga::StorageFormat::Rg32Uint => wgpu::TextureFormat::Rg32Uint,
        naga::StorageFormat::Rg32Sint => wgpu::TextureFormat::Rg32Sint,
        naga::StorageFormat::Rg32Float => wgpu::TextureFormat::Rg32Float,
        naga::StorageFormat::Rgba16Uint => wgpu::TextureFormat::Rgba16Uint,
        naga::StorageFormat::Rgba16Sint => wgpu::TextureFormat::Rgba16Sint,
        naga::StorageFormat::Rgba16Float => wgpu::TextureFormat::Rgba16Float,
        naga::StorageFormat::Rgba32Uint => wgpu::TextureFormat::Rgba32Uint,
        naga::StorageFormat::Rgba32Sint => wgpu::TextureFormat::Rgba32Sint,
        naga::StorageFormat::Rgba32Float => wgpu::TextureFormat::Rgba32Float,
        naga::StorageFormat::R16Unorm => wgpu::TextureFormat::R16Unorm,
        naga::StorageFormat::R16Snorm => wgpu::TextureFormat::R16Snorm,
        naga::StorageFormat::Rg16Unorm => wgpu::TextureFormat::Rg16Unorm,
        naga::StorageFormat::Rg16Snorm => wgpu::TextureFormat::Rg16Snorm,
        naga::StorageFormat::Rgba16Unorm => wgpu::TextureFormat::Rgba16Unorm,
        naga::StorageFormat::Rgba16Snorm => wgpu::TextureFormat::Rgba16Snorm,
        naga::StorageFormat::Bgra8Unorm => wgpu::TextureFormat::Bgra8Unorm,
        naga::StorageFormat::Rgb10a2Uint => wgpu::TextureFormat::Rgb10a2Uint,
    }
}
