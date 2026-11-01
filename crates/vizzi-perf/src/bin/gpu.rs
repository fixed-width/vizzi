//! Native offscreen rendering calibration; it does not measure browser interaction.

use serde_json::json;
use std::{
    error::Error,
    time::{Duration, Instant},
};
use vizzi_perf::{
    scene::{Scene, vertex_bytes},
    timing,
};
use wgpu::util::DeviceExt;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args: Vec<_> = std::env::args().skip(1).collect();
    let allow_software = args.last().is_some_and(|arg| arg == "--allow-software");
    if allow_software {
        args.pop();
    }
    let nodes: u32 = args
        .first()
        .map(String::as_str)
        .unwrap_or("10000")
        .parse()?;
    let degree: u32 = args.get(1).map(String::as_str).unwrap_or("4").parse()?;
    let frames: usize = args.get(2).map(String::as_str).unwrap_or("30").parse()?;
    if args.len() > 3 || frames == 0 {
        return Err("use vizzi-gpu [NODES DEGREE FRAMES] [--allow-software]".into());
    }
    let prepared = Instant::now();
    let scene = Scene::grid(nodes, degree)?;
    let node_data = vertex_bytes(&scene.nodes);
    let edge_data = vertex_bytes(&scene.edges);
    let preparation_us = prepared.elapsed().as_nanos() as f64 / 1000.0;

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
    let info = adapter.get_info();
    let software = info.device_type == wgpu::DeviceType::Cpu;
    let adapter_report = json!({
        "name": info.name, "device_type": format!("{:?}", info.device_type),
        "backend": format!("{:?}", info.backend), "driver": info.driver, "driver_info": info.driver_info,
    });
    if software && !allow_software {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "experiment":"native_offscreen_calibration", "status":"software_adapter_requires_opt_in",
                "adapter":adapter_report, "hardware_baseline":false
            }))?
        );
        return Ok(());
    }

    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
    let uploaded = Instant::now();
    let node_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("nodes"),
        contents: &node_data,
        usage: wgpu::BufferUsages::VERTEX,
    });
    let edge_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("edges"),
        contents: &edge_data,
        usage: wgpu::BufferUsages::VERTEX,
    });
    let camera = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("camera"),
        contents: &vertex_bytes(&[[1.0, 0.0, 0.0, 0.0]]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
    let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &bind_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: camera.as_entire_binding(),
        }],
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(&bind_layout)],
        immediate_size: 0,
    });
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("calibration"),
        source: wgpu::ShaderSource::Wgsl(include_str!("calibration.wgsl").into()),
    });
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let node_pipeline = pipeline(&device, &layout, &shader, format, true);
    let edge_pipeline = pipeline(&device, &layout, &shader, format, false);
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("offscreen"),
        size: wgpu::Extent3d {
            width: 1920,
            height: 1080,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    device.poll(wgpu::PollType::Wait {
        submission_index: None,
        timeout: Some(Duration::from_secs(10)),
    })?;
    let setup_us = uploaded.elapsed().as_nanos() as f64 / 1000.0;
    let edge_vertices = u32::try_from(scene.edges.len())?;
    let mut samples = Vec::with_capacity(frames);
    for frame in 0..frames + 5 {
        let start = Instant::now();
        queue.write_buffer(
            &camera,
            0,
            &vertex_bytes(&[[1.0, (frame as f32 * 0.01).sin() * 0.01, 0.0, 0.0]]),
        );
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.02,
                            g: 0.025,
                            b: 0.03,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_bind_group(0, &bind_group, &[]);
            if edge_vertices > 0 {
                pass.set_pipeline(&edge_pipeline);
                pass.set_vertex_buffer(0, edge_buffer.slice(..));
                pass.draw(0..edge_vertices, 0..1);
            }
            pass.set_pipeline(&node_pipeline);
            pass.set_vertex_buffer(0, node_buffer.slice(..));
            pass.draw(0..6, 0..nodes);
        }
        let submission = queue.submit([encoder.finish()]);
        device.poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(Duration::from_secs(10)),
        })?;
        if frame >= 5 {
            samples.push(start.elapsed());
        }
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "experiment":"native_offscreen_calibration", "status":"measured",
            "adapter":adapter_report, "hardware_baseline":!software,
            "nodes": nodes, "edges": scene.edges.len()/2, "labels":0,
            "width":1920, "height":1080, "vertex_buffer_bytes":node_data.len()+edge_data.len(),
            "scene_preparation_us":preparation_us, "upload_and_pipeline_setup_us":setup_us,
            "synchronous_frame":timing::summarize(&samples),
            "conditions": {
                "build":if cfg!(debug_assertions) { "debug" } else { "release" },
                "warmup_frames":5, "presentation":"offscreen; no browser, compositor, or vsync",
                "scope":"unlabelled nodes and straight edges; no picking, layout updates, or multiple views",
                "timing":"wall clock for uniform update, command encoding, submission, and completion wait; not GPU timestamps"
            }
        }))?
    );
    Ok(())
}

fn pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
    nodes: bool,
) -> wgpu::RenderPipeline {
    let attributes = [wgpu::VertexAttribute {
        format: if nodes {
            wgpu::VertexFormat::Float32x4
        } else {
            wgpu::VertexFormat::Float32x2
        },
        offset: 0,
        shader_location: 0,
    }];
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some(if nodes { "node_vs" } else { "edge_vs" }),
            compilation_options: Default::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: if nodes { 16 } else { 8 },
                step_mode: if nodes {
                    wgpu::VertexStepMode::Instance
                } else {
                    wgpu::VertexStepMode::Vertex
                },
                attributes: &attributes,
            })],
        },
        primitive: wgpu::PrimitiveState {
            topology: if nodes {
                wgpu::PrimitiveTopology::TriangleList
            } else {
                wgpu::PrimitiveTopology::LineList
            },
            ..Default::default()
        },
        depth_stencil: None,
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fragment"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}
