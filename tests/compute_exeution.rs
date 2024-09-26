use kinnara::{BindSlot, ComputeReflector, DeviceUtils, PassSlot, UnboundComputePipeline};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::BufferUsages;

const BASIC_EXEC: &str = r"
#version 450

layout(set=0, binding=0) buffer DataBuffer {
    float data[];
} buf;

layout(push_constant) uniform PushConstants {
    float add;
};

layout(local_size_x=32, local_size_y=1, local_size_z=1) in;
void main() {
    uint index = gl_GlobalInvocationID.x;
    data[index] += add;
}
";

#[test]
fn addition() -> Result<(), kinnara::Error> {
    let (device, queue) = set_up_wgpu();
    let source = compute_stage(BASIC_EXEC);

    let mut refl = ComputeReflector::new_compute(source)?;
    let wg_size = refl.work_group_size("main").unwrap();
    let compute_pipeline = refl.create_compute_pipeline("main", &device, Default::default())?;

    let length = 1048576;
    let add = 5.0f32;
    let data: Vec<_> = (0..length).flat_map(|i| (i as f32).to_le_bytes()).collect();

    let buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: None,
        // TODO: reflector should be able to provide minimum viable flags
        usage: BufferUsages::UNIFORM
            | BufferUsages::MAP_READ
            | BufferUsages::COPY_DST
            | BufferUsages::STORAGE,
        // TODO: be able to provide the data for this through a serde json like object
        contents: &data,
    });

    let bind_group = refl
        .create_bind_group(&device, 0, |req| {
            if let BindSlot::StorageBuffer { loc: (0, 0), slot } = req {
                slot.borrow_mut().replace(buffer.as_entire_buffer_binding());
            }
        })
        .unwrap();

    let mut encoder = device.create_command_encoder(&Default::default());

    {
        let mut cpass = encoder.begin_compute_pass(&Default::default());
        cpass.set_pipeline(&compute_pipeline);
        // TODO make forgetting to do this unrepresentable state
        cpass.set_push_constants(0, &add.to_le_bytes());
        // TODO automate this if possible and make the dynamic offset less fallible
        cpass.set_bind_group(0, &bind_group, &[]);
        cpass.dispatch_workgroups(length / wg_size[0], wg_size[1], wg_size[2]);
    }

    queue.submit([encoder.finish()]);

    let result_floats: Vec<_> = device.buffer_view(&buffer, |slice| {
        slice
            .unwrap()
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect()
    });

    for (i, &value) in result_floats.iter().enumerate() {
        assert_eq!(value, (i as f32 + add), "Mismatch at index {}", i);
    }

    Ok(())
}

#[test]
fn addition_next() -> Result<(), kinnara::Error> {
    let (device, queue) = set_up_wgpu();
    let source = compute_stage(BASIC_EXEC);

    let refl = ComputeReflector::new_compute(source)?;

    let length = 1048576;
    let add = 5.0f32;
    let data: Vec<_> = (0..length).flat_map(|i| (i as f32).to_le_bytes()).collect();

    let buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: None,
        // TODO: reflector should be able to provide minimum viable flags
        usage: BufferUsages::UNIFORM
            | BufferUsages::MAP_READ
            | BufferUsages::COPY_DST
            | BufferUsages::STORAGE,
        // TODO: be able to provide the data for this through a serde json like object
        contents: &data,
    });

    let railed = UnboundComputePipeline::new(&device, "main", Default::default(), refl)?;
    let wg_size = railed.work_group_size().unwrap();

    let bound_pipline = railed.bind(&device, |slot| match slot {
        BindSlot::StorageBuffer { loc: (0, 0), slot } => {
            slot.borrow_mut().replace(buffer.as_entire_buffer_binding());
        }
        _ => {}
    })?;

    let push_slice = add.to_le_bytes();
    let mut encoder = device.create_command_encoder(&Default::default());

    {
        let mut cpass = bound_pipline.create_pass(&mut encoder, |req| match req {
            //TODO: TYPE SAFETY HERE
            PassSlot::PushConstantRange { buffer, .. } => {
                buffer.borrow_mut().replace(&push_slice);
            }
            _ => {}
        })?;
        cpass.dispatch_workgroups(length / wg_size[0], wg_size[1], wg_size[2]);
    }

    queue.submit([encoder.finish()]);

    let result_floats: Vec<_> = device.buffer_view(&buffer, |slice| {
        slice
            .unwrap()
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect()
    });

    for (i, &value) in result_floats.iter().enumerate() {
        assert_eq!(value, (i as f32 + add), "Mismatch at index {}", i);
    }

    Ok(())
}

fn compute_stage(src: &str) -> wgpu::ShaderSource {
    wgpu::ShaderSource::Glsl {
        shader: src.into(),
        stage: wgpu::naga::ShaderStage::Compute,
        defines: Default::default(),
    }
}

fn set_up_wgpu() -> (wgpu::Device, wgpu::Queue) {
    let instance = if cfg!(windows) {
        let desc = wgpu::InstanceDescriptor {
            backends: wgpu::Backends::DX12,
            ..Default::default()
        };

        wgpu::Instance::new(desc)
    } else {
        wgpu::Instance::default()
    };

    let adapter = pollster::block_on(async {
        instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .expect("Failed to find an appropriate adapter")
    });
    let mut required_limits = wgpu::Limits::default().using_resolution(adapter.limits());
    required_limits.max_push_constant_size = 128;

    let (d, q) = pollster::block_on(async {
        adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::PUSH_CONSTANTS
                        | wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
                        | wgpu::Features::MAPPABLE_PRIMARY_BUFFERS
                        | wgpu::Features::CLEAR_TEXTURE,
                    required_limits,
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await
            .expect("Failed to create device")
    });

    d.on_uncaptured_error(Box::new(|e| match e {
        wgpu::Error::Internal {
            source,
            description,
        } => {
            panic!("internal error: {source}, {description}");
        }
        wgpu::Error::OutOfMemory { .. } => {
            panic!("Out Of GPU Memory! bailing");
        }
        wgpu::Error::Validation {
            description,
            source,
        } => {
            panic!("{description} : {source}");
        }
    }));
    (d, q)
}
