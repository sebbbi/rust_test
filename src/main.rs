#![allow(dead_code)]

const SDF_LEVELS: u32 = 6;

extern crate winit;

mod minivector;
mod render_cubes;
mod sdf;
mod sdf_texture;
mod serialization;
mod sparse_sdf;
mod vulkan_base;
mod vulkan_helpers;

use std::time::Instant;

use ash::vk;

use winit::{
    event::{ElementState, Event, MouseButton, MouseScrollDelta, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::desktop::EventLoopExtDesktop,
    window::WindowBuilder,
};

use minivector::*;
use render_cubes::*;
use sdf::*;
use sdf_texture::*;
use vulkan_base::*;
use vulkan_helpers::*;

#[derive(Clone, Copy)]
pub struct Vertex {
    pub pos: [f32; 4],
    pub uv: [f32; 2],
}

pub struct SdfLevel {
    pub sdf: Sdf,
    pub offset: u32,
}

fn main() {
    // Distance field
    let sdf = load_sdf_zlib("data/ganymede-and-jupiter.sdf").expect("SDF loading failed");

    /*
    let sdf = orient_sdf(
        &sdf,
        AxisFlip::PositiveX,
        AxisFlip::PositiveZ,
        AxisFlip::PositiveY,
    );

    store_sdf_zlib("data/ganymede-and-jupiter2.sdf", &sdf);
    */

    let mut sdf_levels = Vec::new();
    let mut sdf_total_voxels = sdf.header.dim.0 * sdf.header.dim.1 * sdf.header.dim.2;
    sdf_levels.push(SdfLevel { sdf, offset: 0 });
    for _ in 1..SDF_LEVELS {
        let sdf = downsample_2x_sdf(&sdf_levels.last().unwrap().sdf);
        let offset = sdf_total_voxels;
        sdf_total_voxels += sdf.header.dim.0 * sdf.header.dim.1 * sdf.header.dim.2;
        sdf_levels.push(SdfLevel { sdf, offset });
    }

    let dx = sdf_levels[0].sdf.header.dx;
    let dim = sdf_levels[0].sdf.header.dim;

    let diagonal = Vec3 {
        x: dx * dim.0 as f32,
        y: dx * dim.1 as f32,
        z: dx * dim.2 as f32,
    };

    let center_to_edge = diagonal * 0.5;
    let diagonal_length = diagonal.length();
    let volume_scale = Vec3::from_scalar(diagonal_length) / diagonal;

    let texels = Vec3 {
        x: dim.0 as f32,
        y: dim.1 as f32,
        z: dim.2 as f32,
    };
    let texel_scale = Vec3::from_scalar(1.0) / texels;

    /*
    let tile_size = 8;
    let stride_y = dim.0;
    let stride_z = dim.0 * dim.1;
    let level_zero = (65536 / 2) as u16;
    let mut total_tile_count = 0;
    let mut edge_tile_count = 0;

    for z in 0..(dim.2/tile_size) {
        for y in (0..dim.1/tile_size) {
            for x in (0..dim.0/tile_size) {
                let tile_offset = tile_size * (z * stride_z + y * stride_y + x);
                let mut has_inside = false;
                let mut has_outside = false;
                for iz in 0..tile_size {
                    for iy in 0..tile_size {
                        for ix in 0..tile_size {
                            let voxel_offset = iz * stride_z + iy * stride_y + ix;
                            let d = sdf.voxels[tile_offset as usize + voxel_offset as usize];
                            if d < level_zero { has_inside = true; };
                            if d > level_zero { has_outside = true; };
                        }
                    }
                }
                if has_inside && has_outside {
                    edge_tile_count += 1;
                }
                total_tile_count += 1;
            }
        }
    }

    println!("Tile size = {}x{}x{}, Total tiles = {}, Edge tiles = {} ({}%)", tile_size, tile_size, tile_size, total_tile_count, edge_tile_count, edge_tile_count as f32 * 100.0 / total_tile_count as f32);
    */

    // Window
    let window_width = 1280;
    let window_height = 720;

    let mut events_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Vulkan Test")
        .with_inner_size(winit::dpi::LogicalSize::new(
            f64::from(window_width),
            f64::from(window_height),
        ))
        .build(&events_loop)
        .unwrap();

    // Vulkan base initialization
    let base = VulkanBase::new(&window, window_width, window_height);

    // Render passes
    let render_pass_attachments = [
        vk::AttachmentDescription {
            format: base.surface_format.format,
            samples: vk::SampleCountFlags::TYPE_1,
            load_op: vk::AttachmentLoadOp::CLEAR,
            store_op: vk::AttachmentStoreOp::STORE,
            final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
            ..Default::default()
        },
        vk::AttachmentDescription {
            format: vk::Format::D32_SFLOAT,
            samples: vk::SampleCountFlags::TYPE_1,
            load_op: vk::AttachmentLoadOp::CLEAR,
            initial_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
            final_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
            ..Default::default()
        },
    ];
    let color_attachment_refs = [vk::AttachmentReference {
        attachment: 0,
        layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    }];
    let depth_attachment_ref = vk::AttachmentReference {
        attachment: 1,
        layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
    };
    let dependencies = [vk::SubpassDependency {
        src_subpass: vk::SUBPASS_EXTERNAL,
        src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_READ
            | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
        dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        ..Default::default()
    }];

    let subpasses = [vk::SubpassDescription::builder()
        .color_attachments(&color_attachment_refs)
        .depth_stencil_attachment(&depth_attachment_ref)
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .build()];

    let render_pass_create_info = vk::RenderPassCreateInfo::builder()
        .attachments(&render_pass_attachments)
        .subpasses(&subpasses)
        .dependencies(&dependencies);

    let render_pass = unsafe {
        base.device
            .create_render_pass(&render_pass_create_info, None)
    }
    .unwrap();

    let framebuffers: Vec<vk::Framebuffer> = base
        .present_image_views
        .iter()
        .map(|&present_image_view| {
            let framebuffer_attachments = [present_image_view, base.depth_image_view];
            let frame_buffer_create_info = vk::FramebufferCreateInfo::builder()
                .render_pass(render_pass)
                .attachments(&framebuffer_attachments)
                .width(base.surface_resolution.width)
                .height(base.surface_resolution.height)
                .layers(1);

            unsafe {
                base.device
                    .create_framebuffer(&frame_buffer_create_info, None)
            }
            .unwrap()
        })
        .collect();

    let view_scissor = {
        let viewport = vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: base.surface_resolution.width as f32,
            height: base.surface_resolution.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        };
        let scissor = vk::Rect2D {
            extent: base.surface_resolution,
            ..Default::default()
        };
        VkViewScissor { viewport, scissor }
    };

    // Descriptor pool
    let descriptor_sizes = [
        vk::DescriptorPoolSize {
            ty: vk::DescriptorType::UNIFORM_BUFFER,
            descriptor_count: 1,
        },
        vk::DescriptorPoolSize {
            ty: vk::DescriptorType::STORAGE_BUFFER,
            descriptor_count: 1,
        },
        vk::DescriptorPoolSize {
            ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            descriptor_count: 1,
        },
    ];
    let descriptor_pool_info = vk::DescriptorPoolCreateInfo::builder()
        .pool_sizes(&descriptor_sizes)
        .max_sets(1);

    let descriptor_pool = unsafe {
        base.device
            .create_descriptor_pool(&descriptor_pool_info, None)
    }
    .unwrap();

    // SDF volume texture
    let sdf_texture = SdfTexture::new(
        &base.device,
        &base.allocator,
        &sdf_levels,
        sdf_total_voxels as usize,
    );

    // Cube renderer
    let render_cubes = RenderCubes::new(
        &base.device,
        &base.allocator,
        &descriptor_pool,
        &render_pass,
        &view_scissor,
        &sdf_texture.descriptor,
    );

    // Submit initialization command buffer before rendering starts
    base.record_submit_commandbuffer(
        0,
        base.present_queue,
        &[],
        &[],
        &[],
        |device, command_buffer| {
            // Setup cube renderer
            render_cubes.setup(device, &command_buffer);

            // Setup SDF volume texture
            sdf_texture.setup(device, &command_buffer, &sdf_levels);
        },
    );

    // Camera
    struct Camera {
        position: Vec3,
        direction: Vec3,
    };

    let mut camera = Camera {
        position: Vec3 {
            x: 0.0,
            y: 25.0,
            z: 50.0,
        },
        direction: Vec3 {
            x: 0.0,
            y: -0.5,
            z: -1.0,
        },
    };

    // Inputs
    #[derive(Clone, Copy)]
    struct Inputs {
        is_left_clicked: bool,
        cursor_position: (i32, i32),
        wheel_delta: f32,
        keyboard_forward: i32,
        keyboard_side: i32,
    };

    impl Default for Inputs {
        fn default() -> Inputs {
            Inputs {
                is_left_clicked: false,
                cursor_position: (0, 0),
                wheel_delta: 0.0,
                keyboard_forward: 0,
                keyboard_side: 0,
            }
        }
    };

    // Window event loop
    println!("Start window event loop");

    let mut inputs_prev: Inputs = Default::default();
    let mut inputs: Inputs = Default::default();

    let mut time_start = Instant::now();
    let mut frame = 0u32;
    let mut active_command_buffer = 0;

    events_loop.run_return(|event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::NewEvents(_) => {
                inputs.wheel_delta = 0.0;
            }

            Event::MainEventsCleared => {
                let cursor_delta = (
                    inputs.cursor_position.0 - inputs_prev.cursor_position.0,
                    inputs.cursor_position.1 - inputs_prev.cursor_position.1,
                );

                inputs_prev = inputs;

                // Update camera based in inputs
                let view_rot = view(
                    Vec3 {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    camera.direction,
                    Vec3 {
                        x: 0.0,
                        y: 1.0,
                        z: 0.0,
                    },
                );

                let forward_speed = inputs.wheel_delta * 5.0 + inputs.keyboard_forward as f32 * 1.5;
                camera.position = camera.position + camera.direction * forward_speed;

                let side_speed = inputs.keyboard_side as f32 * 1.5;
                let side_vec = Vec3 {
                    x: view_rot.r0.x,
                    y: view_rot.r1.x,
                    z: view_rot.r2.x,
                };
                camera.position = camera.position + side_vec * side_speed;

                if inputs.is_left_clicked {
                    let rot = rot_y_axis(cursor_delta.0 as f32 * 0.0015)
                        * rot_x_axis(cursor_delta.1 as f32 * 0.0015);

                    let rot = rot * inverse(view_rot);

                    camera.direction = Vec3 {
                        x: 0.0,
                        y: 0.0,
                        z: 1.0,
                    } * rot;

                    camera.direction = camera.direction.normalize();
                }

                // Render
                let (present_index, _) = unsafe {
                    base.swapchain_loader.acquire_next_image(
                        base.swapchain,
                        std::u64::MAX,
                        base.present_complete_semaphore,
                        vk::Fence::null(),
                    )
                }
                .unwrap();

                // Update uniform buffer
                let color = Vec4 {
                    x: 1.0,
                    y: 0.1,
                    z: 0.0,
                    w: 0.0,
                };

                let world_to_screen = view(
                    camera.position,
                    camera.direction,
                    Vec3 {
                        x: 0.0,
                        y: 1.0,
                        z: 0.0,
                    },
                ) * projection(
                    std::f32::consts::PI / 2.0,
                    window_width as f32 / window_height as f32,
                    1.0,
                    10000000.0,
                );

                let uniforms = CubeUniforms {
                    world_to_screen,
                    color,
                    camera_position: camera.position.to_4d(),
                    volume_scale: volume_scale.to_4d(),
                    center_to_edge: center_to_edge.to_4d(),
                    texel_scale: texel_scale.to_4d(),
                };

                render_cubes.update(&uniforms);

                // Setup render passs
                let clear_values = [
                    vk::ClearValue {
                        color: vk::ClearColorValue {
                            float32: [0.0, 0.0, 0.0, 0.0],
                        },
                    },
                    vk::ClearValue {
                        depth_stencil: vk::ClearDepthStencilValue {
                            depth: 0.0,
                            stencil: 0,
                        },
                    },
                ];

                let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
                    .render_pass(render_pass)
                    .framebuffer(framebuffers[present_index as usize])
                    .render_area(vk::Rect2D {
                        offset: vk::Offset2D { x: 0, y: 0 },
                        extent: base.surface_resolution,
                    })
                    .clear_values(&clear_values);

                // Submit main command buffer
                active_command_buffer = base.record_submit_commandbuffer(
                    active_command_buffer,
                    base.present_queue,
                    &[vk::PipelineStageFlags::BOTTOM_OF_PIPE],
                    &[base.present_complete_semaphore],
                    &[base.rendering_complete_semaphore],
                    |device, command_buffer| {
                        // Setup
                        render_cubes.draw_setup(device, &command_buffer);

                        // Render pass
                        unsafe {
                            device.cmd_begin_render_pass(
                                command_buffer,
                                &render_pass_begin_info,
                                vk::SubpassContents::INLINE,
                            );
                            device.cmd_set_viewport(command_buffer, 0, &[view_scissor.viewport]);
                            device.cmd_set_scissor(command_buffer, 0, &[view_scissor.scissor]);
                        }

                        render_cubes.draw_render_pass(device, &command_buffer);

                        unsafe {
                            device.cmd_end_render_pass(command_buffer);
                        }
                    },
                );

                // Present frame
                let present_info = vk::PresentInfoKHR {
                    wait_semaphore_count: 1,
                    p_wait_semaphores: &base.rendering_complete_semaphore,
                    swapchain_count: 1,
                    p_swapchains: &base.swapchain,
                    p_image_indices: &present_index,
                    ..Default::default()
                };

                unsafe {
                    base.swapchain_loader
                        .queue_present(base.present_queue, &present_info)
                }
                .unwrap();

                // Output performance info every 60 frames
                frame += 1;
                if (frame % 60) == 0 {
                    let time_now = Instant::now();
                    let interval = (time_now - time_start).as_millis();
                    println!("Average frame time: {} ms", interval as f32 / 60.0f32);

                    time_start = time_now;
                }
            }

            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,

                // TODO: Handle swapchain resize
                WindowEvent::Resized { .. } => {}

                // Keyboard
                WindowEvent::KeyboardInput { input, .. } => {
                    let pressed = input.state == ElementState::Pressed;

                    if input.virtual_keycode == Some(VirtualKeyCode::W) {
                        inputs.keyboard_forward = if pressed { 1 } else { 0 };
                    }

                    if input.virtual_keycode == Some(VirtualKeyCode::S) {
                        inputs.keyboard_forward = if pressed { -1 } else { 0 };
                    }

                    if input.virtual_keycode == Some(VirtualKeyCode::D) {
                        inputs.keyboard_side = if pressed { 1 } else { 0 };
                    }

                    if input.virtual_keycode == Some(VirtualKeyCode::A) {
                        inputs.keyboard_side = if pressed { -1 } else { 0 };
                    }
                }

                // Mouse
                WindowEvent::MouseInput {
                    button: MouseButton::Left,
                    state,
                    ..
                } => {
                    inputs.is_left_clicked = state == ElementState::Pressed;
                }
                WindowEvent::CursorMoved { position, .. } => {
                    let position: (i32, i32) = position.into();
                    inputs.cursor_position = position;
                }
                WindowEvent::MouseWheel {
                    delta: MouseScrollDelta::LineDelta(_, v_lines),
                    ..
                } => {
                    inputs.wheel_delta += v_lines;
                }
                _ => (),
            },

            Event::LoopDestroyed => unsafe { base.device.device_wait_idle() }.unwrap(),
            _ => (),
        }
    });

    println!("End window event loop");

    unsafe { base.device.device_wait_idle() }.unwrap();

    // Cleanup
    render_cubes.destroy(&base.device, &base.allocator);
    sdf_texture.destroy(&base.device, &base.allocator);
    unsafe {
        base.device.destroy_descriptor_pool(descriptor_pool, None);
        for framebuffer in framebuffers {
            base.device.destroy_framebuffer(framebuffer, None);
        }
        base.device.destroy_render_pass(render_pass, None);
    }
}
