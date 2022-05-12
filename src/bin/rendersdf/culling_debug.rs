const SIMPLE_FRAGMENT_SHADER: bool = false;
const CUBE_BACKFACE_OPTIMIZATION: bool = true;

use std::default::Default;
use std::ffi::CString;
use std::io::Cursor;
use std::mem;

use ash::util::*;
use ash::{vk, Device};

use gpu_allocator::vulkan::*;
use gpu_allocator::MemoryLocation;

use crate::vulkan_helpers::*;

#[derive(Clone, Copy)]
pub struct CullingDebugUniforms {
    pub depth_pyramid_dimension: u32, // pow2 y dimension of mip 0 (texture x is 1.5x wider)
}

pub struct CullingDebug {
    pub pipeline_layout: vk::PipelineLayout,
    pub uniform_buffer: VkBuffer,
    pub uniform_buffer_gpu: VkBuffer,
    pub desc_set_layout: vk::DescriptorSetLayout,
    pub graphic_pipeline: vk::Pipeline,
    pub descriptor_sets: Vec<vk::DescriptorSet>,
    pub vertex_shader_module: vk::ShaderModule,
    pub fragment_shader_module: vk::ShaderModule,
}

impl CullingDebug {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        device: &Device,
        allocator: &mut Allocator,
        descriptor_pool: &vk::DescriptorPool,
        render_pass: &vk::RenderPass,
        view_scissor: &VkViewScissor,
        depth_pyramid_debug_descriptor: &vk::DescriptorImageInfo,
    ) -> CullingDebug {
        let uniform_buffer_info = vk::BufferCreateInfo {
            size: std::mem::size_of::<CullingDebugUniforms>() as u64,
            usage: vk::BufferUsageFlags::TRANSFER_SRC,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let uniform_buffer = VkBuffer::new(
            device,
            allocator,
            &uniform_buffer_info,
            MemoryLocation::CpuToGpu,
        );

        let uniform_buffer_gpu_info = vk::BufferCreateInfo {
            size: std::mem::size_of::<CullingDebugUniforms>() as u64,
            usage: vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::UNIFORM_BUFFER,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let uniform_buffer_gpu = VkBuffer::new(
            device,
            allocator,
            &uniform_buffer_gpu_info,
            MemoryLocation::GpuOnly,
        );

        let desc_layout_bindings = [
            vk::DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
                ..Default::default()
            },
            vk::DescriptorSetLayoutBinding {
                binding: 1,
                descriptor_type: vk::DescriptorType::STORAGE_IMAGE,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
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
            range: mem::size_of::<CullingDebugUniforms>() as u64,
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
                descriptor_type: vk::DescriptorType::STORAGE_IMAGE,
                p_image_info: depth_pyramid_debug_descriptor,
                ..Default::default()
            },
        ];
        unsafe { device.update_descriptor_sets(&write_desc_sets, &[]) };

        let layout_create_info =
            vk::PipelineLayoutCreateInfo::builder().set_layouts(desc_set_layouts);

        let pipeline_layout =
            unsafe { device.create_pipeline_layout(&layout_create_info, None) }.unwrap();

        let mut vertex_spv_file =
            Cursor::new(&include_bytes!("../../../shader/full_screen_triangle_vert.spv")[..]);
        let mut frag_spv_file =
            Cursor::new(&include_bytes!("../../../shader/culling_debug_frag.spv")[..]);

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
            blend_enable: 1,
            src_color_blend_factor: vk::BlendFactor::ONE,
            dst_color_blend_factor: vk::BlendFactor::ONE,
            color_blend_op: vk::BlendOp::ADD,
            src_alpha_blend_factor: vk::BlendFactor::ONE,
            dst_alpha_blend_factor: vk::BlendFactor::ONE,
            alpha_blend_op: vk::BlendOp::ADD,
            color_write_mask: vk::ColorComponentFlags::RGBA,
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

        CullingDebug {
            pipeline_layout,
            uniform_buffer,
            uniform_buffer_gpu,
            desc_set_layout,
            graphic_pipeline,
            descriptor_sets,
            vertex_shader_module,
            fragment_shader_module,
        }
    }

    pub fn update(&self, uniforms: &CullingDebugUniforms) {
        self.uniform_buffer.copy_from_slice(&[*uniforms], 0);
    }

    pub fn gpu_draw(&self, device: &Device, command_buffer: &vk::CommandBuffer) {
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

    pub fn gpu_draw_main_render_pass(&self, device: &Device, command_buffer: &vk::CommandBuffer) {
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

            device.cmd_draw(*command_buffer, 3, 1, 0, 0);
        }
    }

    pub fn destroy(&mut self, device: &Device, allocator: &mut Allocator) {
        unsafe {
            device.destroy_pipeline(self.graphic_pipeline, None);
            device.destroy_pipeline_layout(self.pipeline_layout, None);
            device.destroy_shader_module(self.vertex_shader_module, None);
            device.destroy_shader_module(self.fragment_shader_module, None);
            self.uniform_buffer.destroy(device, allocator);
            self.uniform_buffer_gpu.destroy(device, allocator);
            device.destroy_descriptor_set_layout(self.desc_set_layout, None);
        }
    }
}
