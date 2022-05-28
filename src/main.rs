// Copyright (c) 2016 The vulkano developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or https://opensource.org/licenses/MIT>,
// at your option. All files in the project carrying such
// notice may not be copied, modified, or distributed except
// according to those terms.

use bytemuck::{Pod, Zeroable};
use cgmath::{Matrix3, Matrix4, Point3, Rad, Vector3};
use std::io::Cursor;
use std::{sync::Arc, time::Instant};
use vulkano::image::{ImageDimensions, ImmutableImage, MipmapsCount};
use vulkano::impl_vertex;
use vulkano::sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo};
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer, CpuBufferPool, TypedBufferAccess},
    command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, SubpassContents},
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    device::{
        physical::{PhysicalDevice, PhysicalDeviceType},
        Device, DeviceCreateInfo, DeviceExtensions, QueueCreateInfo,
    },
    format::Format,
    image::{view::ImageView, AttachmentImage, ImageAccess, ImageUsage, SwapchainImage},
    instance::{Instance, InstanceCreateInfo},
    pipeline::{
        graphics::{
            depth_stencil::DepthStencilState,
            input_assembly::InputAssemblyState,
            vertex_input::BuffersDefinition,
            viewport::{Viewport, ViewportState},
        },
        GraphicsPipeline, Pipeline, PipelineBindPoint,
    },
    render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass},
    shader::ShaderModule,
    swapchain::{
        acquire_next_image, AcquireError, Swapchain, SwapchainCreateInfo, SwapchainCreationError,
    },
    sync::{self, FlushError, GpuFuture},
};
use vulkano_win::VkSurfaceBuild;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
pub struct Vertex {
    position: [f32; 3],
}

impl_vertex!(Vertex, position);

const SIZE: f32 = 10.0;

// x to the right
// y down
// z inwards

fn repeat_element<T: Clone>(it: impl Iterator<Item = T>, cnt: usize) -> impl Iterator<Item = T> {
    it.flat_map(move |n| std::iter::repeat(n).take(cnt))
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
pub struct Normal {
    normal: [f32; 3],
}

impl_vertex!(Normal, normal);

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
pub struct TexCoord {
    tex_coord: [f32; 2],
}

impl_vertex!(TexCoord, tex_coord);

fn main() {
    // TODO to render a cube we only need the three visible faces

    // every vertex is duplicated three times for the three normal directions
    let VERTICES: Vec<Vertex> = repeat_element(
        [
            Vertex {
                position: [-SIZE, -SIZE, -SIZE],
            },
            Vertex {
                position: [SIZE, -SIZE, -SIZE],
            },
            Vertex {
                position: [SIZE, SIZE, -SIZE],
            },
            Vertex {
                position: [-SIZE, SIZE, -SIZE],
            },
            Vertex {
                position: [-SIZE, -SIZE, SIZE],
            },
            Vertex {
                position: [SIZE, -SIZE, SIZE],
            },
            Vertex {
                position: [SIZE, SIZE, SIZE],
            },
            Vertex {
                position: [-SIZE, SIZE, SIZE],
            },
        ]
        .into_iter(),
        3,
    )
    .collect();

    const N_TOP: Normal = Normal {
        normal: [0.0, -SIZE, 0.0],
    };
    const N_BOTTOM: Normal = Normal {
        normal: [0.0, SIZE, 0.0],
    };
    const N_LEFT: Normal = Normal {
        normal: [-SIZE, 0.0, 0.0],
    };
    const N_RIGHT: Normal = Normal {
        normal: [SIZE, 0.0, 0.0],
    };
    const N_FRONT: Normal = Normal {
        normal: [0.0, 0.0, -SIZE],
    };
    const N_BACK: Normal = Normal {
        normal: [0.0, 0.0, SIZE],
    };

    let NORMALS: Vec<Normal> = vec![
        N_LEFT, N_TOP, N_FRONT, N_RIGHT, N_TOP, N_FRONT, N_RIGHT, N_BOTTOM, N_FRONT, N_LEFT,
        N_BOTTOM, N_FRONT, // repeat with N_BACK
        N_LEFT, N_TOP, N_BACK, N_RIGHT, N_TOP, N_BACK, N_RIGHT, N_BOTTOM, N_BACK, N_LEFT, N_BOTTOM,
        N_BACK,
    ];

    let TEXTURE_COORDINATES: Vec<TexCoord> = repeat_element(
        [
            TexCoord {
                tex_coord: [0.0, 0.0],
            },
            TexCoord {
                tex_coord: [1.0, 0.0],
            },
            TexCoord {
                tex_coord: [1.0, 1.0],
            },
            TexCoord {
                tex_coord: [1.0, 1.0],
            },
            TexCoord {
                tex_coord: [0.0, 1.0],
            },
            TexCoord {
                tex_coord: [0.0, 0.0],
            },
        ]
        .into_iter(),
        6,
    )
    .collect();

    let INDICES: Vec<u16> = vec![
        0 * 3 + 2,
        1 * 3 + 2,
        2 * 3 + 2,
        2 * 3 + 2,
        3 * 3 + 2,
        0 * 3 + 2, // front
        4 * 3 + 2,
        5 * 3 + 2,
        6 * 3 + 2,
        6 * 3 + 2,
        7 * 3 + 2,
        4 * 3 + 2, // back
        0 * 3,
        3 * 3,
        7 * 3,
        0 * 3,
        4 * 3,
        7 * 3, // left
        1 * 3,
        2 * 3,
        5 * 3,
        2 * 3,
        5 * 3,
        6 * 3, // right
        0 * 3 + 1,
        1 * 3 + 1,
        4 * 3 + 1,
        1 * 3 + 1,
        4 * 3 + 1,
        5 * 3 + 1, // top
        2 * 3 + 1,
        6 * 3 + 1,
        7 * 3 + 1,
        2 * 3 + 1,
        3 * 3 + 1,
        7 * 3 + 1, // bottom
    ];

    // The start of this example is exactly the same as `triangle`. You should read the
    // `triangle` example if you haven't done so yet.

    let required_extensions = vulkano_win::required_extensions();
    let instance = Instance::new(InstanceCreateInfo {
        enabled_extensions: required_extensions,
        ..Default::default()
    })
    .unwrap();

    let event_loop = EventLoop::new();
    let surface = WindowBuilder::new()
        .build_vk_surface(&event_loop, instance.clone())
        .unwrap();

    let device_extensions = DeviceExtensions {
        khr_swapchain: true,
        ..DeviceExtensions::none()
    };
    let (physical_device, queue_family) = PhysicalDevice::enumerate(&instance)
        .filter(|&p| p.supported_extensions().is_superset_of(&device_extensions))
        .filter_map(|p| {
            p.queue_families()
                .find(|&q| q.supports_graphics() && q.supports_surface(&surface).unwrap_or(false))
                .map(|q| (p, q))
        })
        .min_by_key(|(p, _)| match p.properties().device_type {
            PhysicalDeviceType::DiscreteGpu => 0,
            PhysicalDeviceType::IntegratedGpu => 1,
            PhysicalDeviceType::VirtualGpu => 2,
            PhysicalDeviceType::Cpu => 3,
            PhysicalDeviceType::Other => 4,
        })
        .unwrap();

    println!(
        "Using device: {} (type: {:?})",
        physical_device.properties().device_name,
        physical_device.properties().device_type,
    );

    let (device, mut queues) = Device::new(
        physical_device,
        DeviceCreateInfo {
            enabled_extensions: physical_device
                .required_extensions()
                .union(&device_extensions),
            queue_create_infos: vec![QueueCreateInfo::family(queue_family)],
            ..Default::default()
        },
    )
    .unwrap();

    let queue = queues.next().unwrap();

    let (mut swapchain, images) = {
        let surface_capabilities = physical_device
            .surface_capabilities(&surface, Default::default())
            .unwrap();
        let image_format = Some(
            physical_device
                .surface_formats(&surface, Default::default())
                .unwrap()[0]
                .0,
        );

        Swapchain::new(
            device.clone(),
            surface.clone(),
            SwapchainCreateInfo {
                min_image_count: surface_capabilities.min_image_count,
                image_format,
                image_extent: surface.window().inner_size().into(),
                image_usage: ImageUsage::color_attachment(),
                composite_alpha: surface_capabilities
                    .supported_composite_alpha
                    .iter()
                    .next()
                    .unwrap(),
                ..Default::default()
            },
        )
        .unwrap()
    };

    let vertex_buffer =
        CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, VERTICES)
            .unwrap();
    let normals_buffer =
        CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, NORMALS).unwrap();
    let texture_coordinate_buffer = CpuAccessibleBuffer::from_iter(
        device.clone(),
        BufferUsage::all(),
        false,
        TEXTURE_COORDINATES,
    )
    .unwrap();

    let index_buffer =
        CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, INDICES).unwrap();

    let uniform_buffer = CpuBufferPool::<vs::ty::Data>::new(device.clone(), BufferUsage::all());

    let vs = vs::load(device.clone()).unwrap();
    let fs = fs::load(device.clone()).unwrap();

    let render_pass = vulkano::single_pass_renderpass!(device.clone(),
        attachments: {
            color: {
                load: Clear,
                store: Store,
                format: swapchain.image_format(),
                samples: 1,
            },
            depth: {
                load: Clear,
                store: DontCare,
                format: Format::D16_UNORM,
                samples: 1,
            }
        },
        pass: {
            color: [color],
            depth_stencil: {depth}
        }
    )
    .unwrap();

    let (texture, tex_future) = {
        let png_bytes = include_bytes!("grass_block_side.png").to_vec();
        let cursor = Cursor::new(png_bytes);
        let decoder = png::Decoder::new(cursor);
        let mut reader = decoder.read_info().unwrap();
        let info = reader.info();
        let dimensions = ImageDimensions::Dim2d {
            width: info.width,
            height: info.height,
            array_layers: 1,
        };
        let mut image_data = Vec::new();
        image_data.resize((info.width * info.height * 4) as usize, 0);
        let output = reader.next_frame(&mut image_data).unwrap();

        println!("{:?}", output);

        let (image, future) = ImmutableImage::from_iter(
            image_data,
            dimensions,
            MipmapsCount::One,
            Format::R8G8B8A8_SRGB,
            queue.clone(),
        )
        .unwrap();
        (ImageView::new_default(image).unwrap(), future)
    };

    let sampler = Sampler::new(
        device.clone(),
        SamplerCreateInfo {
            mag_filter: Filter::Nearest,
            min_filter: Filter::Nearest,
            address_mode: [SamplerAddressMode::Repeat; 3],
            ..Default::default()
        },
    )
    .unwrap();

    let (mut pipeline, mut framebuffers) =
        window_size_dependent_setup(device.clone(), &vs, &fs, &images, render_pass.clone());
    let mut recreate_swapchain = false;

    let mut previous_frame_end = Some(tex_future.boxed());
    let rotation_start = Instant::now();

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => {
                recreate_swapchain = true;
            }
            Event::RedrawEventsCleared => {
                previous_frame_end.as_mut().unwrap().cleanup_finished();

                if recreate_swapchain {
                    let (new_swapchain, new_images) =
                        match swapchain.recreate(SwapchainCreateInfo {
                            image_extent: surface.window().inner_size().into(),
                            ..swapchain.create_info()
                        }) {
                            Ok(r) => r,
                            Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => return,
                            Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
                        };

                    swapchain = new_swapchain;
                    let (new_pipeline, new_framebuffers) = window_size_dependent_setup(
                        device.clone(),
                        &vs,
                        &fs,
                        &new_images,
                        render_pass.clone(),
                    );
                    pipeline = new_pipeline;
                    framebuffers = new_framebuffers;
                    recreate_swapchain = false;
                }

                let uniform_buffer_subbuffer = {
                    let elapsed = rotation_start.elapsed();
                    let rotation =
                        elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 / 1_000_000_000.0;
                    let rotation = Matrix3::from_angle_y(Rad(rotation as f32));

                    // note: this teapot was meant for OpenGL where the origin is at the lower left
                    //       instead the origin is at the upper left in Vulkan, so we reverse the Y axis
                    let aspect_ratio =
                        swapchain.image_extent()[0] as f32 / swapchain.image_extent()[1] as f32;
                    let proj = cgmath::perspective(
                        Rad(std::f32::consts::FRAC_PI_2),
                        aspect_ratio,
                        0.01,
                        100.0,
                    );
                    let view = Matrix4::look_at_rh(
                        Point3::new(0.3, 0.3, 1.0),
                        Point3::new(0.0, 0.0, 0.0),
                        Vector3::new(0.0, -1.0, 0.0),
                    );
                    let scale = Matrix4::from_scale(0.01);

                    let uniform_data = vs::ty::Data {
                        world: Matrix4::from(rotation).into(),
                        view: (view * scale).into(),
                        proj: proj.into(),
                    };

                    uniform_buffer.next(uniform_data).unwrap()
                };

                let layout = pipeline.layout().set_layouts().get(0).unwrap();
                let set = PersistentDescriptorSet::new(
                    layout.clone(),
                    [WriteDescriptorSet::buffer(0, uniform_buffer_subbuffer)],
                )
                .unwrap();

                let layout2 = pipeline.layout().set_layouts().get(1).unwrap();
                let set2 = PersistentDescriptorSet::new(
                    layout2.clone(),
                    [WriteDescriptorSet::image_view_sampler(
                        0,
                        texture.clone(),
                        sampler.clone(),
                    )],
                )
                .unwrap();

                let (image_num, suboptimal, acquire_future) =
                    match acquire_next_image(swapchain.clone(), None) {
                        Ok(r) => r,
                        Err(AcquireError::OutOfDate) => {
                            recreate_swapchain = true;
                            return;
                        }
                        Err(e) => panic!("Failed to acquire next image: {:?}", e),
                    };

                if suboptimal {
                    recreate_swapchain = true;
                }

                let mut builder = AutoCommandBufferBuilder::primary(
                    device.clone(),
                    queue.family(),
                    CommandBufferUsage::OneTimeSubmit,
                )
                .unwrap();
                builder
                    .begin_render_pass(
                        framebuffers[image_num].clone(),
                        SubpassContents::Inline,
                        vec![[0.0, 0.0, 1.0, 1.0].into(), 1f32.into()],
                    )
                    .unwrap()
                    .bind_pipeline_graphics(pipeline.clone())
                    .bind_descriptor_sets(
                        PipelineBindPoint::Graphics,
                        pipeline.layout().clone(),
                        0,
                        set.clone(),
                    )
                    .bind_descriptor_sets(
                        PipelineBindPoint::Graphics,
                        pipeline.layout().clone(),
                        1,
                        set2.clone(),
                    )
                    .bind_vertex_buffers(
                        0,
                        (
                            vertex_buffer.clone(),
                            normals_buffer.clone(),
                            texture_coordinate_buffer.clone(),
                        ),
                    )
                    .bind_index_buffer(index_buffer.clone())
                    .draw_indexed(index_buffer.len() as u32, 1, 0, 0, 0)
                    .unwrap()
                    .end_render_pass()
                    .unwrap();
                let command_buffer = builder.build().unwrap();

                let future = previous_frame_end
                    .take()
                    .unwrap()
                    .join(acquire_future)
                    .then_execute(queue.clone(), command_buffer)
                    .unwrap()
                    .then_swapchain_present(queue.clone(), swapchain.clone(), image_num)
                    .then_signal_fence_and_flush();

                match future {
                    Ok(future) => {
                        previous_frame_end = Some(future.boxed());
                    }
                    Err(FlushError::OutOfDate) => {
                        recreate_swapchain = true;
                        previous_frame_end = Some(sync::now(device.clone()).boxed());
                    }
                    Err(e) => {
                        println!("Failed to flush future: {:?}", e);
                        previous_frame_end = Some(sync::now(device.clone()).boxed());
                    }
                }
            }
            _ => (),
        }
    });
}

/// This method is called once during initialization, then again whenever the window is resized
fn window_size_dependent_setup(
    device: Arc<Device>,
    vs: &ShaderModule,
    fs: &ShaderModule,
    images: &[Arc<SwapchainImage<Window>>],
    render_pass: Arc<RenderPass>,
) -> (Arc<GraphicsPipeline>, Vec<Arc<Framebuffer>>) {
    let dimensions = images[0].dimensions().width_height();

    let depth_buffer = ImageView::new_default(
        AttachmentImage::transient(device.clone(), dimensions, Format::D16_UNORM).unwrap(),
    )
    .unwrap();

    let framebuffers = images
        .iter()
        .map(|image| {
            let view = ImageView::new_default(image.clone()).unwrap();
            Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo {
                    attachments: vec![view, depth_buffer.clone()],
                    ..Default::default()
                },
            )
            .unwrap()
        })
        .collect::<Vec<_>>();

    // In the triangle example we use a dynamic viewport, as its a simple example.
    // However in the teapot example, we recreate the pipelines with a hardcoded viewport instead.
    // This allows the driver to optimize things, at the cost of slower window resizes.
    // https://computergraphics.stackexchange.com/questions/5742/vulkan-best-way-of-updating-pipeline-viewport
    let pipeline = GraphicsPipeline::start()
        .vertex_input_state(
            BuffersDefinition::new()
                .vertex::<Vertex>()
                .vertex::<Normal>()
                .vertex::<TexCoord>(),
        )
        .vertex_shader(vs.entry_point("main").unwrap(), ())
        .input_assembly_state(InputAssemblyState::new())
        .viewport_state(ViewportState::viewport_fixed_scissor_irrelevant([
            Viewport {
                origin: [0.0, 0.0],
                dimensions: [dimensions[0] as f32, dimensions[1] as f32],
                depth_range: 0.0..1.0,
            },
        ]))
        .fragment_shader(fs.entry_point("main").unwrap(), ())
        .depth_stencil_state(DepthStencilState::simple_depth_test())
        .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
        .build(device.clone())
        .unwrap();

    (pipeline, framebuffers)
}

mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "src/vert.glsl",
        types_meta: {
            use bytemuck::{Pod, Zeroable};

            #[derive(Clone, Copy, Zeroable, Pod)]
        },
    }
}

mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "src/frag.glsl"
    }
}
