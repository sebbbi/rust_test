extern crate winit;

use std::default::Default;
use std::ffi::CString;
use std::io::Cursor;
use std::mem::{self, align_of};
use std::os::raw::c_void;
use std::time::Instant;

use ash::util::*;
use ash::vk;

use winit::{
    event::{ElementState, Event, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::desktop::EventLoopExtDesktop,
    window::WindowBuilder,
};

mod sdf;
use sdf::*;

mod vulkan_base;
use vulkan_base::*;

mod minivector;
use minivector::*;

#[derive(Clone, Debug, Copy)]
struct Vertex {
    pos: [f32; 4],
    uv: [f32; 2],
}

fn main() {
    unsafe {
        let sdf = load_sdf("data/ganymede-and-jupiter.sdf").expect("SDF loading failed");

        let dx = sdf.header.dx;
        let dim = sdf.header.dim;

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
        let tile_size = 16;
        let dim = sdf.header.dim;
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

        let base = VulkanBase::new(&window, window_width, window_height);

        let renderpass_attachments = [
            vk::AttachmentDescription {
                format: base.surface_format.format,
                samples: vk::SampleCountFlags::TYPE_1,
                load_op: vk::AttachmentLoadOp::CLEAR,
                store_op: vk::AttachmentStoreOp::STORE,
                final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
                ..Default::default()
            },
            vk::AttachmentDescription {
                format: vk::Format::D16_UNORM,
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

        let renderpass_create_info = vk::RenderPassCreateInfo::builder()
            .attachments(&renderpass_attachments)
            .subpasses(&subpasses)
            .dependencies(&dependencies);

        let renderpass = base
            .device
            .create_render_pass(&renderpass_create_info, None)
            .unwrap();

        let framebuffers: Vec<vk::Framebuffer> = base
            .present_image_views
            .iter()
            .map(|&present_image_view| {
                let framebuffer_attachments = [present_image_view, base.depth_image_view];
                let frame_buffer_create_info = vk::FramebufferCreateInfo::builder()
                    .render_pass(renderpass)
                    .attachments(&framebuffer_attachments)
                    .width(base.surface_resolution.width)
                    .height(base.surface_resolution.height)
                    .layers(1);

                base.device
                    .create_framebuffer(&frame_buffer_create_info, None)
                    .unwrap()
            })
            .collect();
        let index_buffer_data = [
            0u32, 2, 1, 2, 3, 1, 2, 6, 3, 6, 7, 3, 7, 1, 3, 7, 5, 1, 5, 4, 1, 1, 4, 0, 0, 4, 6, 0,
            6, 2, 6, 5, 7, 6, 4, 5,
        ];
        let index_buffer_info = vk::BufferCreateInfo {
            size: std::mem::size_of_val(&index_buffer_data) as u64,
            usage: vk::BufferUsageFlags::INDEX_BUFFER,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };
        let index_buffer = base.device.create_buffer(&index_buffer_info, None).unwrap();
        let index_buffer_memory_req = base.device.get_buffer_memory_requirements(index_buffer);
        let index_buffer_memory_index = find_memorytype_index(
            &index_buffer_memory_req,
            &base.device_memory_properties,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )
        .expect("Unable to find suitable memorytype for the index buffer.");
        let index_allocate_info = vk::MemoryAllocateInfo {
            allocation_size: index_buffer_memory_req.size,
            memory_type_index: index_buffer_memory_index,
            ..Default::default()
        };
        let index_buffer_memory = base
            .device
            .allocate_memory(&index_allocate_info, None)
            .unwrap();
        let index_ptr: *mut c_void = base
            .device
            .map_memory(
                index_buffer_memory,
                0,
                index_buffer_memory_req.size,
                vk::MemoryMapFlags::empty(),
            )
            .unwrap();
        let mut index_slice = Align::new(
            index_ptr,
            align_of::<u32>() as u64,
            index_buffer_memory_req.size,
        );
        index_slice.copy_from_slice(&index_buffer_data);
        base.device.unmap_memory(index_buffer_memory);
        base.device
            .bind_buffer_memory(index_buffer, index_buffer_memory, 0)
            .unwrap();

        const num_instances: usize = 1024;

        #[derive(Clone, Copy)]
        struct InstanceData {
            position: Vec4,
        }

        #[derive(Clone, Copy)]
        struct InstanceDatas {
            instances: [InstanceData; num_instances],
        }

        let instances_buffer_info = vk::BufferCreateInfo {
            size: std::mem::size_of::<InstanceDatas>() as u64,
            usage: vk::BufferUsageFlags::STORAGE_BUFFER,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };
        let instances_buffer = base
            .device
            .create_buffer(&instances_buffer_info, None)
            .unwrap();
        let instances_buffer_memory_req =
            base.device.get_buffer_memory_requirements(instances_buffer);
        let instances_buffer_memory_index = find_memorytype_index(
            &instances_buffer_memory_req,
            &base.device_memory_properties,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )
        .expect("Unable to find suitable memorytype for the instances buffer.");

        let instances_buffer_allocate_info = vk::MemoryAllocateInfo {
            allocation_size: instances_buffer_memory_req.size,
            memory_type_index: instances_buffer_memory_index,
            ..Default::default()
        };
        let instances_buffer_memory = base
            .device
            .allocate_memory(&instances_buffer_allocate_info, None)
            .unwrap();

        base.device
            .bind_buffer_memory(instances_buffer, instances_buffer_memory, 0)
            .unwrap();

        #[derive(Clone, Debug, Copy)]
        struct Uniforms {
            model_to_world: Mat4x4,
            world_to_model: Mat4x4,
            model_to_screen: Mat4x4,
            color: Vec4,
            camera_position: Vec4,
            volume_scale: Vec4,
            center_to_edge: Vec4,
            texel_scale: Vec4,
        }

        let uniform_buffer_info = vk::BufferCreateInfo {
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: vk::BufferUsageFlags::UNIFORM_BUFFER,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };
        let uniform_buffer = base
            .device
            .create_buffer(&uniform_buffer_info, None)
            .unwrap();
        let uniform_buffer_memory_req = base.device.get_buffer_memory_requirements(uniform_buffer);
        let uniform_buffer_memory_index = find_memorytype_index(
            &uniform_buffer_memory_req,
            &base.device_memory_properties,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )
        .expect("Unable to find suitable memorytype for the uniform buffer.");

        let uniform_buffer_allocate_info = vk::MemoryAllocateInfo {
            allocation_size: uniform_buffer_memory_req.size,
            memory_type_index: uniform_buffer_memory_index,
            ..Default::default()
        };
        let uniform_buffer_memory = base
            .device
            .allocate_memory(&uniform_buffer_allocate_info, None)
            .unwrap();

        base.device
            .bind_buffer_memory(uniform_buffer, uniform_buffer_memory, 0)
            .unwrap();

        let image_dimensions = sdf.header.dim;
        let image_data = sdf.voxels;
        let image_buffer_info = vk::BufferCreateInfo {
            size: (std::mem::size_of::<u16>() * image_data.len()) as u64,
            usage: vk::BufferUsageFlags::TRANSFER_SRC,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };
        let image_buffer = base.device.create_buffer(&image_buffer_info, None).unwrap();
        let image_buffer_memory_req = base.device.get_buffer_memory_requirements(image_buffer);
        let image_buffer_memory_index = find_memorytype_index(
            &image_buffer_memory_req,
            &base.device_memory_properties,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )
        .expect("Unable to find suitable memorytype for the image buffer.");

        let image_buffer_allocate_info = vk::MemoryAllocateInfo {
            allocation_size: image_buffer_memory_req.size,
            memory_type_index: image_buffer_memory_index,
            ..Default::default()
        };
        let image_buffer_memory = base
            .device
            .allocate_memory(&image_buffer_allocate_info, None)
            .unwrap();
        let image_ptr = base
            .device
            .map_memory(
                image_buffer_memory,
                0,
                image_buffer_memory_req.size,
                vk::MemoryMapFlags::empty(),
            )
            .unwrap();
        let mut image_slice = Align::new(
            image_ptr,
            std::mem::align_of::<u8>() as u64,
            image_buffer_memory_req.size,
        );
        image_slice.copy_from_slice(&image_data);
        base.device.unmap_memory(image_buffer_memory);
        base.device
            .bind_buffer_memory(image_buffer, image_buffer_memory, 0)
            .unwrap();

        let texture_create_info = vk::ImageCreateInfo {
            image_type: vk::ImageType::TYPE_3D,
            format: vk::Format::R16_UNORM,
            extent: vk::Extent3D {
                width: image_dimensions.0,
                height: image_dimensions.1,
                depth: image_dimensions.2,
            },
            mip_levels: 1,
            array_layers: 1,
            samples: vk::SampleCountFlags::TYPE_1,
            tiling: vk::ImageTiling::OPTIMAL,
            usage: vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };
        let texture_image = base
            .device
            .create_image(&texture_create_info, None)
            .unwrap();
        let texture_memory_req = base.device.get_image_memory_requirements(texture_image);
        let texture_memory_index = find_memorytype_index(
            &texture_memory_req,
            &base.device_memory_properties,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )
        .expect("Unable to find suitable memory index for depth image.");

        let texture_allocate_info = vk::MemoryAllocateInfo {
            allocation_size: texture_memory_req.size,
            memory_type_index: texture_memory_index,
            ..Default::default()
        };
        let texture_memory = base
            .device
            .allocate_memory(&texture_allocate_info, None)
            .unwrap();
        base.device
            .bind_image_memory(texture_image, texture_memory, 0)
            .expect("Unable to bind depth image memory");

        base.record_submit_commandbuffer(
            0,
            base.present_queue,
            &[],
            &[],
            &[],
            |device, texture_command_buffer| {
                let texture_barrier = vk::ImageMemoryBarrier {
                    dst_access_mask: vk::AccessFlags::TRANSFER_WRITE,
                    new_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    image: texture_image,
                    subresource_range: vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        level_count: 1,
                        layer_count: 1,
                        ..Default::default()
                    },
                    ..Default::default()
                };
                device.cmd_pipeline_barrier(
                    texture_command_buffer,
                    vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[texture_barrier],
                );
                let buffer_copy_regions = vk::BufferImageCopy::builder()
                    .image_subresource(
                        vk::ImageSubresourceLayers::builder()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .layer_count(1)
                            .build(),
                    )
                    .image_extent(vk::Extent3D {
                        width: image_dimensions.0,
                        height: image_dimensions.1,
                        depth: image_dimensions.2,
                    });

                device.cmd_copy_buffer_to_image(
                    texture_command_buffer,
                    image_buffer,
                    texture_image,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    &[buffer_copy_regions.build()],
                );
                let texture_barrier_end = vk::ImageMemoryBarrier {
                    src_access_mask: vk::AccessFlags::TRANSFER_WRITE,
                    dst_access_mask: vk::AccessFlags::SHADER_READ,
                    old_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                    image: texture_image,
                    subresource_range: vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        level_count: 1,
                        layer_count: 1,
                        ..Default::default()
                    },
                    ..Default::default()
                };
                device.cmd_pipeline_barrier(
                    texture_command_buffer,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::PipelineStageFlags::FRAGMENT_SHADER,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[texture_barrier_end],
                );
            },
        );

        let sampler_info = vk::SamplerCreateInfo {
            mag_filter: vk::Filter::LINEAR,
            min_filter: vk::Filter::LINEAR,
            mipmap_mode: vk::SamplerMipmapMode::LINEAR,
            address_mode_u: vk::SamplerAddressMode::MIRRORED_REPEAT,
            address_mode_v: vk::SamplerAddressMode::MIRRORED_REPEAT,
            address_mode_w: vk::SamplerAddressMode::MIRRORED_REPEAT,
            max_anisotropy: 1.0,
            border_color: vk::BorderColor::FLOAT_OPAQUE_WHITE,
            compare_op: vk::CompareOp::NEVER,
            ..Default::default()
        };

        let sampler = base.device.create_sampler(&sampler_info, None).unwrap();

        let tex_image_view_info = vk::ImageViewCreateInfo {
            view_type: vk::ImageViewType::TYPE_3D,
            format: texture_create_info.format,
            components: vk::ComponentMapping {
                r: vk::ComponentSwizzle::R,
                g: vk::ComponentSwizzle::G,
                b: vk::ComponentSwizzle::B,
                a: vk::ComponentSwizzle::A,
            },
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                level_count: 1,
                layer_count: 1,
                ..Default::default()
            },
            image: texture_image,
            ..Default::default()
        };
        let tex_image_view = base
            .device
            .create_image_view(&tex_image_view_info, None)
            .unwrap();
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

        let descriptor_pool = base
            .device
            .create_descriptor_pool(&descriptor_pool_info, None)
            .unwrap();
        let desc_layout_bindings = [
            vk::DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::FRAGMENT | vk::ShaderStageFlags::VERTEX,
                ..Default::default()
            },
            vk::DescriptorSetLayoutBinding {
                binding: 1,
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::FRAGMENT | vk::ShaderStageFlags::VERTEX,
                ..Default::default()
            },
            vk::DescriptorSetLayoutBinding {
                binding: 2,
                descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::FRAGMENT | vk::ShaderStageFlags::VERTEX,
                ..Default::default()
            },
        ];
        let descriptor_info =
            vk::DescriptorSetLayoutCreateInfo::builder().bindings(&desc_layout_bindings);

        let desc_set_layouts = [base
            .device
            .create_descriptor_set_layout(&descriptor_info, None)
            .unwrap()];

        let desc_alloc_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&desc_set_layouts);
        let descriptor_sets = base
            .device
            .allocate_descriptor_sets(&desc_alloc_info)
            .unwrap();

        let uniform_buffer_descriptor = vk::DescriptorBufferInfo {
            buffer: uniform_buffer,
            offset: 0,
            range: mem::size_of::<Uniforms>() as u64,
        };

        let storage_buffer_descriptor = vk::DescriptorBufferInfo {
            buffer: instances_buffer,
            offset: 0,
            range: mem::size_of::<InstanceDatas>() as u64,
        };

        let tex_descriptor = vk::DescriptorImageInfo {
            image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            image_view: tex_image_view,
            sampler,
        };

        let write_desc_sets = [
            vk::WriteDescriptorSet {
                dst_set: descriptor_sets[0],
                dst_binding: 0,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                p_buffer_info: &uniform_buffer_descriptor,
                ..Default::default()
            },
            vk::WriteDescriptorSet {
                dst_set: descriptor_sets[0],
                dst_binding: 1,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                p_buffer_info: &storage_buffer_descriptor,
                ..Default::default()
            },
            vk::WriteDescriptorSet {
                dst_set: descriptor_sets[0],
                dst_binding: 2,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                p_image_info: &tex_descriptor,
                ..Default::default()
            },
        ];
        base.device.update_descriptor_sets(&write_desc_sets, &[]);

        let mut vertex_spv_file = Cursor::new(&include_bytes!("../shader/main_vert.spv")[..]);
        let mut frag_spv_file = Cursor::new(&include_bytes!("../shader/main_frag.spv")[..]);

        let vertex_code =
            read_spv(&mut vertex_spv_file).expect("Failed to read vertex shader spv file");
        let vertex_shader_info = vk::ShaderModuleCreateInfo::builder().code(&vertex_code);

        let frag_code =
            read_spv(&mut frag_spv_file).expect("Failed to read fragment shader spv file");
        let frag_shader_info = vk::ShaderModuleCreateInfo::builder().code(&frag_code);

        let vertex_shader_module = base
            .device
            .create_shader_module(&vertex_shader_info, None)
            .expect("Vertex shader module error");

        let fragment_shader_module = base
            .device
            .create_shader_module(&frag_shader_info, None)
            .expect("Fragment shader module error");

        let layout_create_info =
            vk::PipelineLayoutCreateInfo::builder().set_layouts(&desc_set_layouts);

        let pipeline_layout = base
            .device
            .create_pipeline_layout(&layout_create_info, None)
            .unwrap();

        let shader_entry_name = CString::new("main").unwrap();
        let shader_stage_create_infos = [
            vk::PipelineShaderStageCreateInfo {
                module: vertex_shader_module,
                p_name: shader_entry_name.as_ptr(),
                stage: vk::ShaderStageFlags::VERTEX,
                ..Default::default()
            },
            vk::PipelineShaderStageCreateInfo {
                module: fragment_shader_module,
                p_name: shader_entry_name.as_ptr(),
                stage: vk::ShaderStageFlags::FRAGMENT,
                ..Default::default()
            },
        ];
        /*
        let vertex_input_binding_descriptions = [vk::VertexInputBindingDescription {
            binding: 0,
            stride: mem::size_of::<Vertex>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }];
        let vertex_input_attribute_descriptions = [
            vk::VertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: offset_of!(Vertex, pos) as u32,
            },
            vk::VertexInputAttributeDescription {
                location: 1,
                binding: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: offset_of!(Vertex, uv) as u32,
            },
        ];
        let vertex_input_state_info = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_attribute_descriptions(&vertex_input_attribute_descriptions)
            .vertex_binding_descriptions(&vertex_input_binding_descriptions);
            */

        let vertex_input_state_info = vk::PipelineVertexInputStateCreateInfo::default();

        let vertex_input_assembly_state_info = vk::PipelineInputAssemblyStateCreateInfo {
            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            ..Default::default()
        };
        let viewports = [vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: base.surface_resolution.width as f32,
            height: base.surface_resolution.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        }];
        let scissors = [vk::Rect2D {
            extent: base.surface_resolution,
            ..Default::default()
        }];
        let viewport_state_info = vk::PipelineViewportStateCreateInfo::builder()
            .scissors(&scissors)
            .viewports(&viewports);

        let rasterization_info = vk::PipelineRasterizationStateCreateInfo {
            cull_mode: vk::CullModeFlags::BACK,
            front_face: vk::FrontFace::COUNTER_CLOCKWISE,
            line_width: 1.0,
            polygon_mode: vk::PolygonMode::FILL,
            ..Default::default()
        };

        let multisample_state_info = vk::PipelineMultisampleStateCreateInfo::builder()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        let noop_stencil_state = vk::StencilOpState {
            fail_op: vk::StencilOp::KEEP,
            pass_op: vk::StencilOp::KEEP,
            depth_fail_op: vk::StencilOp::KEEP,
            compare_op: vk::CompareOp::ALWAYS,
            ..Default::default()
        };
        let depth_state_info = vk::PipelineDepthStencilStateCreateInfo {
            depth_test_enable: 1,
            depth_write_enable: 1,
            depth_compare_op: vk::CompareOp::LESS_OR_EQUAL,
            front: noop_stencil_state,
            back: noop_stencil_state,
            max_depth_bounds: 1.0,
            ..Default::default()
        };

        let color_blend_attachment_states = [vk::PipelineColorBlendAttachmentState {
            blend_enable: 0,
            src_color_blend_factor: vk::BlendFactor::SRC_COLOR,
            dst_color_blend_factor: vk::BlendFactor::ONE_MINUS_DST_COLOR,
            color_blend_op: vk::BlendOp::ADD,
            src_alpha_blend_factor: vk::BlendFactor::ZERO,
            dst_alpha_blend_factor: vk::BlendFactor::ZERO,
            alpha_blend_op: vk::BlendOp::ADD,
            color_write_mask: vk::ColorComponentFlags::all(),
        }];
        let color_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op(vk::LogicOp::CLEAR)
            .attachments(&color_blend_attachment_states);

        let dynamic_state = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state_info =
            vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&dynamic_state);

        let graphic_pipeline_infos = vk::GraphicsPipelineCreateInfo::builder()
            .stages(&shader_stage_create_infos)
            .vertex_input_state(&vertex_input_state_info)
            .input_assembly_state(&vertex_input_assembly_state_info)
            .viewport_state(&viewport_state_info)
            .rasterization_state(&rasterization_info)
            .multisample_state(&multisample_state_info)
            .depth_stencil_state(&depth_state_info)
            .color_blend_state(&color_blend_state)
            .dynamic_state(&dynamic_state_info)
            .layout(pipeline_layout)
            .render_pass(renderpass);

        let graphics_pipelines = base
            .device
            .create_graphics_pipelines(
                vk::PipelineCache::null(),
                &[graphic_pipeline_infos.build()],
                None,
            )
            .unwrap();

        let graphic_pipeline = graphics_pipelines[0];

        {
            struct Application {
                is_left_clicked: bool,
                cursor_position: (i32, i32),
                cursor_delta: Option<(i32, i32)>,
                wheel_delta: Option<f32>,
            };

            let mut app = Application {
                is_left_clicked: false,
                cursor_position: (0, 0),
                cursor_delta: None,
                wheel_delta: None,
            };

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

            /*
            let instances_buffer_data2 = (0..num_instances)
                .map(|i| i)
                .collect();
               */
            
            let instances_buffer_data : Vec<InstanceData> = (0..num_instances)
                .map(|i| InstanceData {
                    position: Vec4 {
                        x: 0.0,
                        y: 5.0 * i as f32,
                        z: 0.0,
                        w: 1.0,
                    },
                })
                .collect();

            let instances_ptr = base
                .device
                .map_memory(
                    instances_buffer_memory,
                    0,
                    instances_buffer_memory_req.size,
                    vk::MemoryMapFlags::empty(),
                )
                .unwrap();
            let mut instances_aligned_slice = Align::new(
                instances_ptr,
                align_of::<Vec4>() as u64,
                instances_buffer_memory_req.size,
            );
            instances_aligned_slice.copy_from_slice(&[&instances_buffer_data]);
            base.device.unmap_memory(instances_buffer_memory);

            let mut time_start = Instant::now();
            let mut frame = 0u32;
            let mut active_command_buffer = 0;

            // Used to accumutate input events from the start to the end of a frame
            let mut is_left_clicked = None;
            let mut cursor_position = None;
            let mut last_position = app.cursor_position;
            let mut wheel_delta = None;
            let mut dirty_swapchain = false;

            println!("Start event loop");

            events_loop.run_return(|event, _, control_flow| {
                *control_flow = ControlFlow::Poll;

                match event {
                    Event::NewEvents(_) => {
                        // reset input states on new frame
                        {
                            is_left_clicked = None;
                            cursor_position = None;
                            last_position = app.cursor_position;
                            wheel_delta = None;
                        }
                    }
                    Event::MainEventsCleared => {
                        // update input state after accumulating event
                        {
                            if let Some(is_left_clicked) = is_left_clicked {
                                app.is_left_clicked = is_left_clicked;
                            }
                            if let Some(position) = cursor_position {
                                app.cursor_position = position;
                                app.cursor_delta = Some((
                                    position.0 - last_position.0,
                                    position.1 - last_position.1,
                                ));
                            } else {
                                app.cursor_delta = None;
                            }
                            app.wheel_delta = wheel_delta;
                        }

                        // render
                        {
                            // TODO spawchain resize
                            /*
                            if dirty_swapchain {
                                let size = window.inner_size();
                                if size.width > 0 && size.height > 0 {
                                    app.recreate_swapchain();
                                } else {
                                    return;
                                }
                            }
                            dirty_swapchain = app.draw_frame();*/

                            let (present_index, _) = base
                                .swapchain_loader
                                .acquire_next_image(
                                    base.swapchain,
                                    std::u64::MAX,
                                    base.present_complete_semaphore,
                                    vk::Fence::null(),
                                )
                                .unwrap();

                            if let Some(delta) = app.wheel_delta {
                                camera.position = camera.position + camera.direction * delta * 5.0;
                            }
                            if app.is_left_clicked {
                                if let Some(delta) = app.cursor_delta {
                                    let rot = rot_x_axis(delta.1 as f32 * -0.001)
                                        * rot_y_axis(delta.0 as f32 * 0.001);
                                    camera.direction = camera.direction * rot;
                                    camera.direction = camera.direction.normalize();
                                }
                            }

                            let color = Vec4 {
                                x: 1.0,
                                y: 0.1,
                                z: 0.0,
                                w: 0.0,
                            };

                            let model_to_world = rot_x_axis(-std::f32::consts::PI / 2.0) // Model from Z-up to Y-up
                                * rot_y_axis(3.201)
//                                * rot_y_axis(frame as f32 * 0.001)
                                * translate(Vec3 {
                                    x: 0.0, // - 2.0 * (frame as f32 * 0.01).cos(),
                                    y: 0.0, // + 3.0 * (frame as f32 * 0.01).sin(),
                                    z: 0.0,
                                });

                            let model_to_view = model_to_world
                                * view(
                                    camera.position,
                                    camera.direction,
                                    Vec3 {
                                        x: 0.0,
                                        y: 1.0,
                                        z: 0.0,
                                    },
                                );

                            let model_to_screen = model_to_view
                                * projection(
                                    std::f32::consts::PI / 2.0,
                                    window_width as f32 / window_height as f32,
                                    0.1,
                                    1000.0,
                                );

                            let world_to_model = inverse(model_to_world);

                            let uniform_buffer_data = Uniforms {
                                model_to_world,
                                world_to_model,
                                model_to_screen,
                                color,
                                camera_position: camera.position.to_4d(),
                                volume_scale: volume_scale.to_4d(),
                                center_to_edge: center_to_edge.to_4d(),
                                texel_scale: texel_scale.to_4d(),
                            };

                            let uniform_ptr = base
                                .device
                                .map_memory(
                                    uniform_buffer_memory,
                                    0,
                                    uniform_buffer_memory_req.size,
                                    vk::MemoryMapFlags::empty(),
                                )
                                .unwrap();
                            let mut uniform_aligned_slice = Align::new(
                                uniform_ptr,
                                align_of::<Vec4>() as u64,
                                uniform_buffer_memory_req.size,
                            );
                            uniform_aligned_slice.copy_from_slice(&[uniform_buffer_data]);
                            base.device.unmap_memory(uniform_buffer_memory);

                            let clear_values = [
                                vk::ClearValue {
                                    color: vk::ClearColorValue {
                                        float32: [0.0, 0.0, 0.0, 0.0],
                                    },
                                },
                                vk::ClearValue {
                                    depth_stencil: vk::ClearDepthStencilValue {
                                        depth: 1.0,
                                        stencil: 0,
                                    },
                                },
                            ];

                            let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
                                .render_pass(renderpass)
                                .framebuffer(framebuffers[present_index as usize])
                                .render_area(vk::Rect2D {
                                    offset: vk::Offset2D { x: 0, y: 0 },
                                    extent: base.surface_resolution,
                                })
                                .clear_values(&clear_values);

                            active_command_buffer = base.record_submit_commandbuffer(
                                active_command_buffer,
                                base.present_queue,
                                &[vk::PipelineStageFlags::BOTTOM_OF_PIPE],
                                &[base.present_complete_semaphore],
                                &[base.rendering_complete_semaphore],
                                |device, draw_command_buffer| {
                                    device.cmd_begin_render_pass(
                                        draw_command_buffer,
                                        &render_pass_begin_info,
                                        vk::SubpassContents::INLINE,
                                    );
                                    device.cmd_bind_descriptor_sets(
                                        draw_command_buffer,
                                        vk::PipelineBindPoint::GRAPHICS,
                                        pipeline_layout,
                                        0,
                                        &descriptor_sets[..],
                                        &[],
                                    );
                                    device.cmd_bind_pipeline(
                                        draw_command_buffer,
                                        vk::PipelineBindPoint::GRAPHICS,
                                        graphic_pipeline,
                                    );
                                    device.cmd_set_viewport(draw_command_buffer, 0, &viewports);
                                    device.cmd_set_scissor(draw_command_buffer, 0, &scissors);
                                    /*device.cmd_bind_vertex_buffers(
                                        draw_command_buffer,
                                        0,
                                        &[vertex_input_buffer],
                                        &[0],
                                    );*/
                                    device.cmd_bind_index_buffer(
                                        draw_command_buffer,
                                        index_buffer,
                                        0,
                                        vk::IndexType::UINT32,
                                    );
                                    device.cmd_draw_indexed(
                                        draw_command_buffer,
                                        index_buffer_data.len() as u32,
                                        1,
                                        0,
                                        0,
                                        1,
                                    );
                                    // Or draw without the index buffer
                                    // device.cmd_draw(draw_command_buffer, 3, 1, 0, 0);
                                    device.cmd_end_render_pass(draw_command_buffer);
                                },
                            );

                            //let mut present_info_err = mem::zeroed();
                            let present_info = vk::PresentInfoKHR {
                                wait_semaphore_count: 1,
                                p_wait_semaphores: &base.rendering_complete_semaphore,
                                swapchain_count: 1,
                                p_swapchains: &base.swapchain,
                                p_image_indices: &present_index,
                                ..Default::default()
                            };

                            base.swapchain_loader
                                .queue_present(base.present_queue, &present_info)
                                .unwrap();

                            frame += 1;
                            if (frame % 60) == 0 {
                                let time_now = Instant::now();
                                let interval = (time_now - time_start).as_millis();
                                println!("Avg frame time: {}", interval as f32 / 60.0f32);

                                time_start = time_now;
                            }
                        }
                    }
                    Event::WindowEvent { event, .. } => match event {
                        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                        WindowEvent::Resized { .. } => dirty_swapchain = true,
                        // Accumulate input events
                        WindowEvent::MouseInput {
                            button: MouseButton::Left,
                            state,
                            ..
                        } => {
                            if state == ElementState::Pressed {
                                is_left_clicked = Some(true);
                            } else {
                                is_left_clicked = Some(false);
                            }
                        }
                        WindowEvent::CursorMoved { position, .. } => {
                            let position: (i32, i32) = position.into();
                            cursor_position = Some(position);
                        }
                        WindowEvent::MouseWheel {
                            delta: MouseScrollDelta::LineDelta(_, v_lines),
                            ..
                        } => {
                            wheel_delta = Some(v_lines);
                        }
                        _ => (),
                    },
                    Event::LoopDestroyed => base.device.device_wait_idle().unwrap(),
                    _ => (),
                }
            });
        }

        println!("End event loop");

        base.device.device_wait_idle().unwrap();

        for pipeline in graphics_pipelines {
            base.device.destroy_pipeline(pipeline, None);
        }
        base.device.destroy_pipeline_layout(pipeline_layout, None);
        base.device
            .destroy_shader_module(vertex_shader_module, None);
        base.device
            .destroy_shader_module(fragment_shader_module, None);
        base.device.free_memory(image_buffer_memory, None);
        base.device.destroy_buffer(image_buffer, None);
        base.device.free_memory(texture_memory, None);
        base.device.destroy_image_view(tex_image_view, None);
        base.device.destroy_image(texture_image, None);
        base.device.free_memory(index_buffer_memory, None);
        base.device.destroy_buffer(index_buffer, None);
        base.device.free_memory(uniform_buffer_memory, None);
        base.device.destroy_buffer(uniform_buffer, None);
        base.device.free_memory(instances_buffer_memory, None);
        base.device.destroy_buffer(instances_buffer, None);
        for &descriptor_set_layout in desc_set_layouts.iter() {
            base.device
                .destroy_descriptor_set_layout(descriptor_set_layout, None);
        }
        base.device.destroy_descriptor_pool(descriptor_pool, None);
        base.device.destroy_sampler(sampler, None);
        for framebuffer in framebuffers {
            base.device.destroy_framebuffer(framebuffer, None);
        }
        base.device.destroy_render_pass(renderpass, None);
    }
}
