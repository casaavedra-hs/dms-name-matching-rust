#![cfg(feature = "gpu")]

use anyhow::{anyhow, Context, Result};
use wgpu::util::DeviceExt;

const MAX_CHARS: usize = 64;
const STRIDE: usize = 130;

#[derive(Debug)]
pub struct GpuLevenshtein {
    instance: wgpu::Instance,
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    adapter_name: String,
    backend_name: String,
}

impl GpuLevenshtein {
    pub fn new() -> Result<Self> {
        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        })).context("No compatible GPU adapter was found")?;

        let info = adapter.get_info();
        let adapter_name = info.name.clone();
        let backend_name = format!("{:?}", info.backend);

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("DMS Name Matching GPU Device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        })).context("Unable to create GPU compute device")?;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Levenshtein Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/levenshtein.wgsl").into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Levenshtein Pipeline"),
            layout: None,
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        Ok(Self { instance, device, queue, pipeline, adapter_name, backend_name })
    }

    pub fn adapter_name(&self) -> &str { &self.adapter_name }
    pub fn backend_name(&self) -> &str { &self.backend_name }

    pub fn normalized_similarity_batch(&mut self, pairs: &[(String, String)]) -> Result<Vec<f32>> {
        if pairs.is_empty() { return Ok(Vec::new()); }

        let mut packed = vec![0u32; pairs.len() * STRIDE];
        for (idx, (a, b)) in pairs.iter().enumerate() {
            let base = idx * STRIDE;
            let aa: Vec<u32> = a.to_uppercase().chars().take(MAX_CHARS).map(|c| c as u32).collect();
            let bb: Vec<u32> = b.to_uppercase().chars().take(MAX_CHARS).map(|c| c as u32).collect();
            packed[base] = aa.len() as u32;
            packed[base + 1] = bb.len() as u32;
            for (i, c) in aa.into_iter().enumerate() { packed[base + 2 + i] = c; }
            for (i, c) in bb.into_iter().enumerate() { packed[base + 66 + i] = c; }
        }

        let input = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("DMS GPU Input"),
            contents: bytemuck::cast_slice(&packed),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let output_bytes = (pairs.len() * std::mem::size_of::<u32>()) as u64;
        let output = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("DMS GPU Output"),
            size: output_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("DMS GPU Readback"),
            size: output_bytes,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let layout = self.pipeline.get_bind_group_layout(0);
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("DMS GPU Bind Group"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: input.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: output.as_entire_binding() },
            ],
        });

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("DMS GPU Encoder") });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("DMS GPU Levenshtein Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(((pairs.len() as u32) + 63) / 64, 1, 1);
        }
        encoder.copy_buffer_to_buffer(&output, 0, &readback, 0, output_bytes);
        let submission = self.queue.submit(Some(encoder.finish()));

        let slice = readback.slice(..);
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        slice.map_async(wgpu::MapMode::Read, move |r| { let _ = tx.send(r); });
        self.device.poll(wgpu::PollType::Wait { submission_index: Some(submission), timeout: None })
            .map_err(|e| anyhow!("GPU poll failed: {e:?}"))?;
        rx.recv().map_err(|_| anyhow!("GPU readback callback was dropped"))??;

        let mapped = slice.get_mapped_range();
        let raw: &[u32] = bytemuck::cast_slice(&mapped);
        let result = raw.iter().map(|v| (*v as f32) / 100.0).collect::<Vec<_>>();
        drop(mapped);
        readback.unmap();
        let _ = &self.instance;
        Ok(result)
    }
}
