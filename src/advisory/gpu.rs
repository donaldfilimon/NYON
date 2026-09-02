use std::{
    collections::VecDeque,
    num::NonZeroU64,
    sync::{Arc, Mutex},
};

use super::{
    AdvisoryMetadata, AdvisoryRequest, PACKED_FEATURE_COUNT, PACKED_WEIGHT_COUNT, SCORE_COUNT,
    packed_weights,
};

pub const ADVISORY_WGSL: &str = include_str!("../../assets/shaders/advisory.wgsl");
pub const FEATURES_BUFFER_SIZE: u64 = (PACKED_FEATURE_COUNT * size_of::<f32>()) as u64;
pub const WEIGHTS_BUFFER_SIZE: u64 = (PACKED_WEIGHT_COUNT * size_of::<f32>()) as u64;
pub const SCORES_BUFFER_SIZE: u64 = (SCORE_COUNT * size_of::<f32>()) as u64;

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum GpuAdvisoryError {
    #[error("GPU advisory is already processing a request")]
    Busy,
    #[error("advisory shader validation failed: {0}")]
    Shader(String),
    #[error("GPU score readback failed: {0}")]
    Map(String),
    #[error("GPU device polling failed: {0}")]
    Poll(String),
    #[error("GPU device failed: {0}")]
    Device(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct GpuCompletion {
    pub metadata: AdvisoryMetadata,
    pub result: Result<[f32; SCORE_COUNT], GpuAdvisoryError>,
}

struct MapSignal {
    metadata: AdvisoryMetadata,
    result: Result<(), GpuAdvisoryError>,
}

pub struct GpuAdvisory {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    bind_group: wgpu::BindGroup,
    features: wgpu::Buffer,
    scores: wgpu::Buffer,
    readback: wgpu::Buffer,
    completions: Arc<Mutex<VecDeque<MapSignal>>>,
    in_flight: Option<AdvisoryMetadata>,
}

impl GpuAdvisory {
    pub fn adapter_supported(adapter: &wgpu::Adapter) -> bool {
        adapter
            .get_downlevel_capabilities()
            .flags
            .contains(wgpu::DownlevelFlags::COMPUTE_SHADERS)
    }

    pub fn new(
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<Option<Self>, GpuAdvisoryError> {
        if !Self::adapter_supported(adapter) {
            return Ok(None);
        }
        crate::engine::shader::validate_wgsl("assets/shaders/advisory.wgsl", ADVISORY_WGSL)
            .map_err(|error| GpuAdvisoryError::Shader(error.to_string()))?;

        let features = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("advisory features"),
            size: FEATURES_BUFFER_SIZE,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let weights = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("advisory weights"),
            size: WEIGHTS_BUFFER_SIZE,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let scores = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("advisory scores"),
            size: SCORES_BUFFER_SIZE,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("advisory score readback"),
            size: SCORES_BUFFER_SIZE,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&weights, 0, bytemuck::cast_slice(&packed_weights()));

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("advisory bind group layout"),
            entries: &[
                storage_layout_entry(0, true, FEATURES_BUFFER_SIZE),
                storage_layout_entry(1, true, WEIGHTS_BUFFER_SIZE),
                storage_layout_entry(2, false, SCORES_BUFFER_SIZE),
            ],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("advisory bind group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: features.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: weights.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: scores.as_entire_binding(),
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("advisory pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("assets/shaders/advisory.wgsl"),
            source: wgpu::ShaderSource::Wgsl(ADVISORY_WGSL.into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("advisory compute pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("score_world"),
            compilation_options: Default::default(),
            cache: None,
        });

        Ok(Some(Self {
            device: device.clone(),
            queue: queue.clone(),
            pipeline,
            bind_group,
            features,
            scores,
            readback,
            completions: Arc::new(Mutex::new(VecDeque::new())),
            in_flight: None,
        }))
    }

    pub fn submit(&mut self, request: &AdvisoryRequest) -> Result<(), GpuAdvisoryError> {
        if self.in_flight.is_some() {
            return Err(GpuAdvisoryError::Busy);
        }
        self.queue
            .write_buffer(&self.features, 0, bytemuck::cast_slice(&request.features));
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("advisory command encoder"),
            });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("advisory compute pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }
        encoder.copy_buffer_to_buffer(&self.scores, 0, &self.readback, 0, SCORES_BUFFER_SIZE);

        let metadata = request.metadata;
        let completions = Arc::clone(&self.completions);
        encoder.map_buffer_on_submit(
            &self.readback,
            wgpu::MapMode::Read,
            0..SCORES_BUFFER_SIZE,
            move |result| {
                let completion_result =
                    result.map_err(|error| GpuAdvisoryError::Map(error.to_string()));
                if let Ok(mut queue) = completions.lock() {
                    queue.push_back(MapSignal {
                        metadata,
                        result: completion_result,
                    });
                }
            },
        );
        self.queue.submit([encoder.finish()]);
        self.in_flight = Some(metadata);
        Ok(())
    }

    pub fn poll(&mut self) -> Result<Option<GpuCompletion>, GpuAdvisoryError> {
        #[cfg(not(target_arch = "wasm32"))]
        if let Err(error) = self.device.poll(wgpu::PollType::Poll) {
            let Some(metadata) = self.in_flight.take() else {
                return Err(GpuAdvisoryError::Poll(error.to_string()));
            };
            return Ok(Some(GpuCompletion {
                metadata,
                result: Err(GpuAdvisoryError::Poll(error.to_string())),
            }));
        }
        let signal = match self.completions.lock() {
            Ok(mut queue) => queue.pop_front(),
            Err(_) => {
                let Some(metadata) = self.in_flight.take() else {
                    return Err(GpuAdvisoryError::Map(
                        "completion queue lock was poisoned".into(),
                    ));
                };
                return Ok(Some(GpuCompletion {
                    metadata,
                    result: Err(GpuAdvisoryError::Map(
                        "completion queue lock was poisoned".into(),
                    )),
                }));
            }
        };
        let Some(signal) = signal else {
            return Ok(None);
        };
        self.in_flight = None;
        let result = signal.result.and_then(|()| {
            let view = self
                .readback
                .slice(..)
                .get_mapped_range()
                .map_err(|error| GpuAdvisoryError::Map(error.to_string()))?;
            let mut values = [0.0; SCORE_COUNT];
            for (index, value) in values.iter_mut().enumerate() {
                let start = index * size_of::<f32>();
                *value = f32::from_le_bytes(
                    view[start..start + size_of::<f32>()]
                        .try_into()
                        .expect("four-byte score"),
                );
            }
            drop(view);
            self.readback.unmap();
            Ok(values)
        });
        Ok(Some(GpuCompletion {
            metadata: signal.metadata,
            result,
        }))
    }

    pub fn has_in_flight_request(&self) -> bool {
        self.in_flight.is_some()
    }
}

fn storage_layout_entry(binding: u32, read_only: bool, size: u64) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: NonZeroU64::new(size),
        },
        count: None,
    }
}
