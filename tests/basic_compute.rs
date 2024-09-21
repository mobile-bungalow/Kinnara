use kinnara::*;
use wgpu::{BindGroupLayoutDescriptor, PushConstantRange, ShaderSource, ShaderStages};

fn compute_stage(src: &str) -> ShaderSource {
    wgpu::ShaderSource::Glsl {
        shader: src.into(),
        stage: wgpu::naga::ShaderStage::Compute,
        defines: Default::default(),
    }
}

const BASIC_SRC: &str = r"
#version 450

struct Base {
    float a;
    float b;
};

layout(set=0, binding=0) uniform Base name;
layout(push_constant) uniform Base next;

layout(local_size_x=16, local_size_y=16, local_size_z=1) in;
void main() {}
";

#[test]
fn basic_reflection() {
    let source = compute_stage(BASIC_SRC);
    let refl = ReflectionContext::new_compute_reflector(source).unwrap();

    let binding_0 = wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    };

    let pc_range = PushConstantRange {
        stages: ShaderStages::COMPUTE,
        range: 0..8,
    };

    let desc = refl.get_bind_group_layout_entry(0, 0).unwrap();
    let ranges = refl.push_constant_range().unwrap();
    let not_desc = refl.get_bind_group_layout_entry(0, 1);

    assert_eq!(pc_range, ranges);
    assert_eq!(binding_0, desc);
    assert!(not_desc.is_none());

    let bind_group = refl.get_bind_group_layout_descriptor(0);
    let bind_group_2 = refl.get_bind_group_layout_descriptor(1);

    let reference = BindGroupLayoutDescriptor {
        label: None,
        entries: &[binding_0],
    };

    assert_eq!(bind_group.entries, reference.entries);
    assert!(bind_group_2.entries.is_empty());
    assert!(bind_group_2.label.is_none());
}

const STORAGE_BUFFER_SRC: &str = r"
#version 450
struct Data {
    float values[4];
};
layout(set=0, binding=0) buffer DataBuffer {
    Data data[];
} buf;
layout(local_size_x=16, local_size_y=1, local_size_z=1) in;
void main() {}
";

#[test]
fn storage_buffer_reflection() {
    let source = compute_stage(STORAGE_BUFFER_SRC);
    let refl = ReflectionContext::new_compute_reflector(source).unwrap();

    let binding_0 = wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    };

    let desc = refl.get_bind_group_layout_entry(0, 0).unwrap();
    assert_eq!(binding_0, desc);

    let bind_group = refl.get_bind_group_layout_descriptor(0);
    let reference = BindGroupLayoutDescriptor {
        label: None,
        entries: &[binding_0],
    };
    assert_eq!(bind_group.entries, reference.entries);

    let pc_range = refl.push_constant_range();
    assert!(pc_range.is_none());
}

const MULTIPLE_BINDINGS_SRC: &str = r"
#version 450
layout(set=0, binding=0) uniform texture2D tex1;
layout(set=0, binding=1) uniform texture2D tex2;
layout(set=0, binding=2) uniform sampler samp;
layout(set=1, binding=0) uniform UniformBuffer {
    vec4 color;
} ubo;
layout(local_size_x=16, local_size_y=16, local_size_z=1) in;
void main() {}
";

#[test]
fn multiple_bindings_reflection() {
    let source = compute_stage(MULTIPLE_BINDINGS_SRC);
    let refl = ReflectionContext::new_compute_reflector(source).unwrap();

    let binding_0 = wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    };
    let binding_1 = wgpu::BindGroupLayoutEntry {
        binding: 1,
        visibility: ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    };
    let binding_2 = wgpu::BindGroupLayoutEntry {
        binding: 2,
        visibility: ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
        count: None,
    };

    let desc_0 = refl.get_bind_group_layout_entry(0, 0).unwrap();
    let desc_1 = refl.get_bind_group_layout_entry(0, 1).unwrap();
    let desc_2 = refl.get_bind_group_layout_entry(0, 2).unwrap();
    assert_eq!(binding_0, desc_0);
    assert_eq!(binding_1, desc_1);
    assert_eq!(binding_2, desc_2);

    let binding_3 = wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    };

    let desc_3 = refl.get_bind_group_layout_entry(1, 0).unwrap();
    assert_eq!(binding_3, desc_3);

    let bind_group_0 = refl.get_bind_group_layout_descriptor(0);
    let bind_group_1 = refl.get_bind_group_layout_descriptor(1);

    assert_eq!(bind_group_0.entries.len(), 3);
    assert_eq!(bind_group_1.entries.len(), 1);

    let pc_range = refl.push_constant_range();
    assert!(pc_range.is_none());
}
