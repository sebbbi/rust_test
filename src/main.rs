#![allow(dead_code)]

const SDF_LEVELS: u32 = 6;
const SIMPLE_FRAGMENT_SHADER: bool = false;
const CUBE_BACKFACE_OPTIMIZATION: bool = true;
const NUM_INSTANCES: usize = 1024 * 1024;

extern crate winit;

mod minivector;
mod sdf;
mod serialization;
mod sparse_sdf;
mod vulkan_base;
mod vulkan_helpers;

use rand::Rng;
use std::default::Default;
use std::ffi::CString;
use std::io::Cursor;
use std::mem;
use std::time::Instant;

use ash::util::*;
use ash::{vk, Device};

use winit::{
    event::{ElementState, Event, MouseButton, MouseScrollDelta, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::desktop::EventLoopExtDesktop,
    window::WindowBuilder,
};

use minivector::*;
use sdf::*;
use vulkan_base::*;
use vulkan_helpers::*;

#[derive(Clone, Copy)]
struct Vertex {
    pos: [f32; 4],
    uv: [f32; 2],
}

#[derive(Clone, Copy)]
struct Uniforms {
    world_to_screen: Mat4x4,
    color: Vec4,
    camera_position: Vec4,
    volume_scale: Vec4,
    center_to_edge: Vec4,
    texel_scale: Vec4,
}

struct RenderCubes {
    pub pipeline_layout: vk::PipelineLayout,
    pub index_buffer: VkBuffer,
    pub index_buffer_gpu: VkBuffer,
    pub instances_buffer: VkBuffer,
    pub uniform_buffer: VkBuffer,
    pub uniform_buffer_gpu: VkBuffer,
    pub desc_set_layout: vk::DescriptorSetLayout,
    pub graphic_pipeline: vk::Pipeline,
    pub descriptor_sets: Vec<vk::DescriptorSet>,
    pub vertex_shader_module: vk::ShaderModule,
    pub fragment_shader_module: vk::ShaderModule,
}

impl RenderCubes {
    pub fn new(
        device: &Device,
        allocator: &vk_mem::Allocator,
        descriptor_pool: &vk::DescriptorPool,
        render_pass: &vk::RenderPass,
        view_scissor: &VkViewScissor,
        sdf_texture: &VkUploadImageWithViewSampler,
    ) -> RenderCubes {
        let alloc_info_cpu = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::CpuOnly,
            flags: vk_mem::AllocationCreateFlags::MAPPED,
            ..Default::default()
        };

        let alloc_info_gpu = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::GpuOnly,
            ..Default::default()
        };

        let alloc_info_cpu_to_gpu = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::CpuToGpu,
            flags: vk_mem::AllocationCreateFlags::MAPPED,
            ..Default::default()
        };

        const NUM_CUBE_INDICES: usize = if CUBE_BACKFACE_OPTIMIZATION {
            3 * 3 * 2
        } else {
            3 * 6 * 2
        };
        const NUM_CUBE_VERTICES: usize = 8;

        #[rustfmt::skip]
        let cube_indices = [
            0u32, 2, 1, 2, 3, 1,
            5, 4, 1, 1, 4, 0,
            0, 4, 6, 0, 6, 2,
            6, 5, 7, 6, 4, 5,
            2, 6, 3, 6, 7, 3,
            7, 1, 3, 7, 5, 1,
        ];

        let num_indices = NUM_INSTANCES * NUM_CUBE_INDICES;

        let index_buffer_data: Vec<u32> = (0..num_indices)
            .map(|i| {
                let cube = i / NUM_CUBE_INDICES;
                let cube_local = i % NUM_CUBE_INDICES;
                cube_indices[cube_local] + cube as u32 * NUM_CUBE_VERTICES as u32
            })
            .collect();

        let index_buffer_info = vk::BufferCreateInfo {
            size: std::mem::size_of_val(&index_buffer_data[..]) as u64,
            usage: vk::BufferUsageFlags::TRANSFER_SRC,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let index_buffer = VkBuffer::new(allocator, &index_buffer_info, &alloc_info_cpu);
        index_buffer.copy_from_slice(&index_buffer_data[..], 0);

        let index_buffer_gpu_info = vk::BufferCreateInfo {
            size: std::mem::size_of_val(&index_buffer_data[..]) as u64,
            usage: vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let index_buffer_gpu = VkBuffer::new(allocator, &index_buffer_gpu_info, &alloc_info_gpu);

        #[derive(Clone, Copy)]
        struct InstanceData {
            position: Vec4,
        }

        #[derive(Clone, Copy)]
        struct InstanceDatas {
            instances: [InstanceData; NUM_INSTANCES],
        }

        let instances_buffer_info = vk::BufferCreateInfo {
            size: std::mem::size_of::<InstanceDatas>() as u64,
            usage: vk::BufferUsageFlags::STORAGE_BUFFER,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let instances_buffer =
            VkBuffer::new(allocator, &instances_buffer_info, &alloc_info_cpu_to_gpu);

        let uniform_buffer_info = vk::BufferCreateInfo {
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: vk::BufferUsageFlags::TRANSFER_SRC,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let uniform_buffer = VkBuffer::new(allocator, &uniform_buffer_info, &alloc_info_cpu);

        let uniform_buffer_gpu_info = vk::BufferCreateInfo {
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::UNIFORM_BUFFER,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let uniform_buffer_gpu =
            VkBuffer::new(allocator, &uniform_buffer_gpu_info, &alloc_info_gpu);

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

        let desc_set_layout =
            unsafe { device.create_descriptor_set_layout(&descriptor_info, None) }.unwrap();

        let desc_set_layouts = &[desc_set_layout];

        let descriptor_sets = {
            let desc_alloc_info = vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(*descriptor_pool)
                .set_layouts(desc_set_layouts);

            unsafe { device.allocate_descriptor_sets(&desc_alloc_info) }.unwrap()
        };

        let uniform_buffer_descriptor = vk::DescriptorBufferInfo {
            buffer: uniform_buffer_gpu.buffer,
            offset: 0,
            range: mem::size_of::<Uniforms>() as u64,
        };

        let storage_buffer_descriptor = vk::DescriptorBufferInfo {
            buffer: instances_buffer.buffer,
            offset: 0,
            range: mem::size_of::<InstanceDatas>() as u64,
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
                p_image_info: &sdf_texture.descriptor,
                ..Default::default()
            },
        ];
        unsafe { device.update_descriptor_sets(&write_desc_sets, &[]) };

        let layout_create_info =
            vk::PipelineLayoutCreateInfo::builder().set_layouts(desc_set_layouts);

        let pipeline_layout =
            unsafe { device.create_pipeline_layout(&layout_create_info, None) }.unwrap();

        let mut vertex_spv_file = Cursor::new(if CUBE_BACKFACE_OPTIMIZATION {
            &include_bytes!("../shader/main_frontface_vert.spv")[..]
        } else {
            &include_bytes!("../shader/main_vert.spv")[..]
        });

        let mut frag_spv_file = Cursor::new(if SIMPLE_FRAGMENT_SHADER {
            &include_bytes!("../shader/simple_frag.spv")[..]
        } else {
            &include_bytes!("../shader/main_frag.spv")[..]
        });

        let vertex_code =
            read_spv(&mut vertex_spv_file).expect("Failed to read vertex shader spv file");
        let vertex_shader_info = vk::ShaderModuleCreateInfo::builder().code(&vertex_code);

        let frag_code =
            read_spv(&mut frag_spv_file).expect("Failed to read fragment shader spv file");
        let frag_shader_info = vk::ShaderModuleCreateInfo::builder().code(&frag_code);

        let vertex_shader_module =
            unsafe { device.create_shader_module(&vertex_shader_info, None) }
                .expect("Vertex shader module error");

        let fragment_shader_module =
            unsafe { device.create_shader_module(&frag_shader_info, None) }
                .expect("Fragment shader module error");

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

        let vertex_input_state_info = vk::PipelineVertexInputStateCreateInfo::default();

        let vertex_input_assembly_state_info = vk::PipelineInputAssemblyStateCreateInfo {
            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            ..Default::default()
        };

        let scissors = &[view_scissor.scissor];
        let viewports = &[view_scissor.viewport];
        let viewport_state_info = vk::PipelineViewportStateCreateInfo::builder()
            .scissors(scissors)
            .viewports(viewports);

        let rasterization_info = vk::PipelineRasterizationStateCreateInfo {
            cull_mode: vk::CullModeFlags::NONE,
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
            depth_compare_op: vk::CompareOp::GREATER_OR_EQUAL,
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
            .render_pass(*render_pass);

        let graphics_pipelines = unsafe {
            device.create_graphics_pipelines(
                vk::PipelineCache::null(),
                &[graphic_pipeline_infos.build()],
                None,
            )
        }
        .unwrap();

        let graphic_pipeline = graphics_pipelines[0];

        // Random cloud of SDF box instances
        let mut rng = rand::thread_rng();
        let instances_buffer_data: Vec<InstanceData> = (0..NUM_INSTANCES)
            .map(|_i| InstanceData {
                position: Vec4 {
                    x: rng.gen_range(-8000.0, 8000.0),
                    y: rng.gen_range(-8000.0, 8000.0),
                    z: rng.gen_range(-8000.0, 8000.0),
                    w: 1.0,
                },
            })
            .collect();

        instances_buffer.copy_from_slice(&instances_buffer_data[..], 0);

        RenderCubes {
            pipeline_layout,
            index_buffer,
            index_buffer_gpu,
            instances_buffer,
            uniform_buffer,
            uniform_buffer_gpu,
            desc_set_layout,
            graphic_pipeline,
            descriptor_sets,
            vertex_shader_module,
            fragment_shader_module,
        }
    }

    pub fn update(&self, uniforms: &Uniforms) {
        self.uniform_buffer.copy_from_slice(&[*uniforms], 0);
    }

    pub fn setup(&self, device: &Device, command_buffer: &vk::CommandBuffer) {
        let buffer_copy_regions = vk::BufferCopy {
            src_offset: 0,
            dst_offset: 0,
            size: self.index_buffer.size,
        };

        let buffer_barrier = vk::BufferMemoryBarrier {
            dst_access_mask: vk::AccessFlags::TRANSFER_WRITE,
            buffer: self.index_buffer_gpu.buffer,
            offset: 0,
            size: buffer_copy_regions.size,
            ..Default::default()
        };

        let buffer_barrier_end = vk::BufferMemoryBarrier {
            src_access_mask: vk::AccessFlags::TRANSFER_WRITE,
            dst_access_mask: vk::AccessFlags::INDEX_READ,
            buffer: self.index_buffer_gpu.buffer,
            offset: 0,
            size: buffer_copy_regions.size,
            ..Default::default()
        };

        unsafe {
            device.cmd_pipeline_barrier(
                *command_buffer,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &[],
                &[buffer_barrier],
                &[],
            );

            device.cmd_copy_buffer(
                *command_buffer,
                self.index_buffer.buffer,
                self.index_buffer_gpu.buffer,
                &[buffer_copy_regions],
            );

            device.cmd_pipeline_barrier(
                *command_buffer,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::VERTEX_INPUT,
                vk::DependencyFlags::empty(),
                &[],
                &[buffer_barrier_end],
                &[],
            );
        };
    }

    fn draw_setup(&self, device: &Device, command_buffer: &vk::CommandBuffer) {
        let buffer_copy_regions = vk::BufferCopy {
            src_offset: 0,
            dst_offset: 0,
            size: self.uniform_buffer.size,
        };

        let buffer_barrier = vk::BufferMemoryBarrier {
            dst_access_mask: vk::AccessFlags::TRANSFER_WRITE,
            buffer: self.uniform_buffer_gpu.buffer,
            offset: 0,
            size: buffer_copy_regions.size,
            ..Default::default()
        };

        let buffer_barrier_end = vk::BufferMemoryBarrier {
            src_access_mask: vk::AccessFlags::TRANSFER_WRITE,
            dst_access_mask: vk::AccessFlags::INDEX_READ,
            buffer: self.uniform_buffer_gpu.buffer,
            offset: 0,
            size: buffer_copy_regions.size,
            ..Default::default()
        };

        unsafe {
            device.cmd_pipeline_barrier(
                *command_buffer,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &[],
                &[buffer_barrier],
                &[],
            );

            device.cmd_copy_buffer(
                *command_buffer,
                self.uniform_buffer.buffer,
                self.uniform_buffer_gpu.buffer,
                &[buffer_copy_regions],
            );

            device.cmd_pipeline_barrier(
                *command_buffer,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::VERTEX_INPUT,
                vk::DependencyFlags::empty(),
                &[],
                &[buffer_barrier_end],
                &[],
            );
        }
    }

    fn draw_render_pass(&self, device: &Device, command_buffer: &vk::CommandBuffer) {
        unsafe {
            device.cmd_bind_descriptor_sets(
                *command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                &self.descriptor_sets[..],
                &[],
            );
            device.cmd_bind_pipeline(
                *command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.graphic_pipeline,
            );
            device.cmd_bind_index_buffer(
                *command_buffer,
                self.index_buffer_gpu.buffer,
                0,
                vk::IndexType::UINT32,
            );
            device.cmd_draw_indexed(
                *command_buffer,
                self.index_buffer_gpu.size as u32 / std::mem::size_of::<u32>() as u32,
                1,
                0,
                0,
                1,
            );
        }
    }

    fn destroy(&self, device: &Device, allocator: &vk_mem::Allocator) {
        unsafe {
            device.destroy_pipeline(self.graphic_pipeline, None);
            device.destroy_pipeline_layout(self.pipeline_layout, None);
            device.destroy_shader_module(self.vertex_shader_module, None);
            device.destroy_shader_module(self.fragment_shader_module, None);
            self.index_buffer.destroy(&allocator);
            self.index_buffer_gpu.destroy(&allocator);
            self.uniform_buffer.destroy(&allocator);
            self.uniform_buffer_gpu.destroy(&allocator);
            self.instances_buffer.destroy(&allocator);
            device.destroy_descriptor_set_layout(self.desc_set_layout, None);
        }
    }
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

    struct SdfLevel {
        pub sdf: Sdf,
        pub offset: u32,
    }

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

    let alloc_info_cpu = vk_mem::AllocationCreateInfo {
        usage: vk_mem::MemoryUsage::CpuOnly,
        flags: vk_mem::AllocationCreateFlags::MAPPED,
        ..Default::default()
    };

    let alloc_info_gpu = vk_mem::AllocationCreateInfo {
        usage: vk_mem::MemoryUsage::GpuOnly,
        ..Default::default()
    };

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

    // Volume texture for distance field
    let sdf_texture = {
        let image_buffer_info = vk::BufferCreateInfo {
            size: (std::mem::size_of::<u16>() * sdf_total_voxels as usize) as u64,
            usage: vk::BufferUsageFlags::TRANSFER_SRC,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let upload_buffer = VkBuffer::new(&base.allocator, &image_buffer_info, &alloc_info_cpu);

        for level in &sdf_levels {
            upload_buffer.copy_from_slice(
                &level.sdf.voxels[..],
                level.offset as usize * std::mem::size_of::<u16>(),
            );
        }

        let image_dimensions = sdf_levels[0].sdf.header.dim;

        let texture_create_info = vk::ImageCreateInfo {
            image_type: vk::ImageType::TYPE_3D,
            format: vk::Format::R16_UNORM,
            extent: vk::Extent3D {
                width: image_dimensions.0,
                height: image_dimensions.1,
                depth: image_dimensions.2,
            },
            mip_levels: sdf_levels.len() as u32,
            array_layers: 1,
            samples: vk::SampleCountFlags::TYPE_1,
            tiling: vk::ImageTiling::OPTIMAL,
            usage: vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let image = VkImage::new(&base.allocator, &texture_create_info, &alloc_info_gpu);

        let sampler_info = vk::SamplerCreateInfo {
            mag_filter: vk::Filter::LINEAR,
            min_filter: vk::Filter::LINEAR,
            //mipmap_mode: vk::SamplerMipmapMode::LINEAR,    // LINEAR mipmap sampling is 1/2 rate
            mipmap_mode: vk::SamplerMipmapMode::NEAREST,
            address_mode_u: vk::SamplerAddressMode::REPEAT,
            address_mode_v: vk::SamplerAddressMode::REPEAT,
            address_mode_w: vk::SamplerAddressMode::REPEAT,
            max_anisotropy: 1.0,
            border_color: vk::BorderColor::FLOAT_OPAQUE_WHITE,
            compare_op: vk::CompareOp::NEVER,
            min_lod: 0.0,
            max_lod: sdf_levels.len() as f32,
            ..Default::default()
        };

        let sampler = unsafe { base.device.create_sampler(&sampler_info, None).unwrap() };

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
                level_count: sdf_levels.len() as u32,
                layer_count: 1,
                ..Default::default()
            },
            image: image.image,
            ..Default::default()
        };
        let tex_image_view =
            unsafe { base.device.create_image_view(&tex_image_view_info, None) }.unwrap();

        let tex_descriptor = vk::DescriptorImageInfo {
            image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            image_view: tex_image_view,
            sampler,
        };

        VkUploadImageWithViewSampler {
            image,
            upload_buffer,
            sampler,
            view: tex_image_view,
            descriptor: tex_descriptor,
        }
    };

    // Cube renderer
    let render_cubes = RenderCubes::new(
        &base.device,
        &base.allocator,
        &descriptor_pool,
        &render_pass,
        &view_scissor,
        &sdf_texture,
    );

    // Submit initialization command buffer before rendering starts
    base.record_submit_commandbuffer(
        0,
        base.present_queue,
        &[],
        &[],
        &[],
        |device, command_buffer| {
            // Setup cubes
            render_cubes.setup(device, &command_buffer);

            // Setup distance field texture
            let texture_barrier = vk::ImageMemoryBarrier {
                dst_access_mask: vk::AccessFlags::TRANSFER_WRITE,
                new_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                image: sdf_texture.image.image,
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    level_count: sdf_levels.len() as u32,
                    layer_count: 1,
                    ..Default::default()
                },
                ..Default::default()
            };

            let image_copys: Vec<vk::BufferImageCopy> = (0..sdf_levels.len())
                .map(|i| {
                    let buffer_image_copy_regions = vk::BufferImageCopy::builder()
                        .buffer_offset(
                            std::mem::size_of::<u16>() as u64 * sdf_levels[i].offset as u64,
                        )
                        .image_subresource(
                            vk::ImageSubresourceLayers::builder()
                                .aspect_mask(vk::ImageAspectFlags::COLOR)
                                .mip_level(i as u32)
                                .layer_count(1)
                                .build(),
                        )
                        .image_extent(vk::Extent3D {
                            width: sdf_levels[i].sdf.header.dim.0,
                            height: sdf_levels[i].sdf.header.dim.1,
                            depth: sdf_levels[i].sdf.header.dim.2,
                        });
                    buffer_image_copy_regions.build()
                })
                .collect();

            let texture_barrier_end = vk::ImageMemoryBarrier {
                src_access_mask: vk::AccessFlags::TRANSFER_WRITE,
                dst_access_mask: vk::AccessFlags::SHADER_READ,
                old_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                image: sdf_texture.image.image,
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    level_count: sdf_levels.len() as u32,
                    layer_count: 1,
                    ..Default::default()
                },
                ..Default::default()
            };

            unsafe {
                device.cmd_pipeline_barrier(
                    command_buffer,
                    vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[texture_barrier],
                );

                device.cmd_copy_buffer_to_image(
                    command_buffer,
                    sdf_texture.upload_buffer.buffer,
                    sdf_texture.image.image,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    &image_copys[..],
                );

                device.cmd_pipeline_barrier(
                    command_buffer,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::PipelineStageFlags::FRAGMENT_SHADER,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[texture_barrier_end],
                );
            };
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

                let uniforms = Uniforms {
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
