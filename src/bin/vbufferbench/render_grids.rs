enum GridTechnique {
    Color,
    PrimId,
    NonIndexed,
}

const GRID_TECHNIQUE: GridTechnique = GridTechnique::Color;

use std::default::Default;
use std::ffi::CString;
use std::io::Cursor;
use std::mem;

use ash::util::*;
use ash::{vk, Device};

use crate::minivector::*;
use crate::vulkan_base::*;
use crate::vulkan_helpers::*;

#[derive(Clone, Copy)]
pub struct GridUniforms {
    pub world_to_screen: Mat4x4,
    pub color: Vec4,
    pub center_to_edge: Vec4,
}

pub struct RenderGrids {
    pub pipeline_layout: vk::PipelineLayout,
    pub index_buffer: VkBuffer,
    pub index_buffer_gpu: VkBuffer,
    pub uniform_buffer: VkBuffer,
    pub uniform_buffer_gpu: VkBuffer,
    pub desc_set_layout: vk::DescriptorSetLayout,
    pub graphic_pipeline: vk::Pipeline,
    pub descriptor_sets: Vec<vk::DescriptorSet>,
    pub vertex_shader_module: vk::ShaderModule,
    pub fragment_shader_module: vk::ShaderModule,
}

impl RenderGrids {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        device: &Device,
        allocator: &vk_mem::Allocator,
        descriptor_pool: &vk::DescriptorPool,
        render_pass: &vk::RenderPass,
        view_scissor: &VkViewScissor,
        instances_buffer_descriptor: &vk::DescriptorBufferInfo,
        num_instances: usize,
    ) -> RenderGrids {
        let alloc_info_cpu = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::CpuOnly,
            flags: vk_mem::AllocationCreateFlags::MAPPED,
            ..Default::default()
        };

        let alloc_info_gpu = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::GpuOnly,
            ..Default::default()
        };

        const GRID_DIM: usize = 7;
        const NUM_GRID_INDICES: usize = GRID_DIM * GRID_DIM * 2 * 3;
        const NUM_GRID_VERTICES: usize = (GRID_DIM + 1) * (GRID_DIM + 1);

        let mut grid_indices: [u32; NUM_GRID_INDICES] = [0; NUM_GRID_INDICES];
        for y in 0..GRID_DIM {
            for x in 0..GRID_DIM {
                let grid = x + y * GRID_DIM;
                let vertex = (x + y * (GRID_DIM + 1)) as u32;

                // Upper left triangle
                grid_indices[grid * 6 + 0] = 0 + vertex;
                grid_indices[grid * 6 + 1] = 1 + vertex;
                grid_indices[grid * 6 + 2] = (GRID_DIM + 1) as u32 + vertex;

                // Lower right triangle
                grid_indices[grid * 6 + 3] = (GRID_DIM + 1) as u32 + vertex;
                grid_indices[grid * 6 + 4] = 1 + vertex;
                grid_indices[grid * 6 + 5] = 1 + (GRID_DIM + 1) as u32 + vertex;
            }
        }

        let num_indices = num_instances * NUM_GRID_INDICES;

        let index_buffer_data: Vec<u32> = (0..num_indices)
            .map(|i| {
                let grid = i / NUM_GRID_INDICES;
                let grid_local = i % NUM_GRID_INDICES;
                grid_indices[grid_local] + grid as u32 * NUM_GRID_VERTICES as u32
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

        let uniform_buffer_info = vk::BufferCreateInfo {
            size: std::mem::size_of::<GridUniforms>() as u64,
            usage: vk::BufferUsageFlags::TRANSFER_SRC,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let uniform_buffer = VkBuffer::new(allocator, &uniform_buffer_info, &alloc_info_cpu);

        let uniform_buffer_gpu_info = vk::BufferCreateInfo {
            size: std::mem::size_of::<GridUniforms>() as u64,
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
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::FRAGMENT | vk::ShaderStageFlags::VERTEX,
                ..Default::default()
            },
            vk::DescriptorSetLayoutBinding {
                binding: 3,
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
            range: mem::size_of::<GridUniforms>() as u64,
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
                p_buffer_info: instances_buffer_descriptor,
                ..Default::default()
            },
        ];
        unsafe { device.update_descriptor_sets(&write_desc_sets, &[]) };

        let layout_create_info =
            vk::PipelineLayoutCreateInfo::builder().set_layouts(desc_set_layouts);

        let pipeline_layout =
            unsafe { device.create_pipeline_layout(&layout_create_info, None) }.unwrap();


        let mut vertex_spv_file = Cursor::new(match GRID_TECHNIQUE {
            GridTechnique::Color => &include_bytes!("../../../shader/vbuffer_vert.spv")[..],
            GridTechnique::PrimId => &include_bytes!("../../../shader/vbuffer_vert.spv")[..],
            GridTechnique::NonIndexed => &include_bytes!("../../../shader/vbuffer_nonindexed_vert.spv")[..],
        });

        let mut frag_spv_file = Cursor::new(match GRID_TECHNIQUE {
            GridTechnique::Color => &include_bytes!("../../../shader/vbuffer_color_frag.spv")[..],
            GridTechnique::PrimId => &include_bytes!("../../../shader/vbuffer_primid_frag.spv")[..],
            GridTechnique::NonIndexed => &include_bytes!("../../../shader/vbuffer_nonindexed_frag.spv")[..],
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

        RenderGrids {
            pipeline_layout,
            index_buffer,
            index_buffer_gpu,
            uniform_buffer,
            uniform_buffer_gpu,
            desc_set_layout,
            graphic_pipeline,
            descriptor_sets,
            vertex_shader_module,
            fragment_shader_module,
        }
    }

    pub fn update(&self, uniforms: &GridUniforms) {
        self.uniform_buffer.copy_from_slice(&[*uniforms], 0);
    }

    pub fn gpu_setup(&self, device: &Device, command_buffer: &vk::CommandBuffer) {
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

    pub fn gpu_draw_main_render_pass(
        &self,
        device: &Device,
        command_buffer: &vk::CommandBuffer,
    ) {
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

            match GRID_TECHNIQUE {
                GridTechnique::NonIndexed =>                 
                    device.cmd_draw(
                            *command_buffer,
                            self.index_buffer_gpu.size as u32 / std::mem::size_of::<u32>() as u32,
                            1,
                            0,
                            0,
                        ),
                _ => device.cmd_draw_indexed(
                    *command_buffer,
                    self.index_buffer_gpu.size as u32 / std::mem::size_of::<u32>() as u32,
                    1,
                    0,
                    0,
                    0,
                )
            }
        }
    }

    pub fn destroy(&self, device: &Device, allocator: &vk_mem::Allocator) {
        unsafe {
            device.destroy_pipeline(self.graphic_pipeline, None);
            device.destroy_pipeline_layout(self.pipeline_layout, None);
            device.destroy_shader_module(self.vertex_shader_module, None);
            device.destroy_shader_module(self.fragment_shader_module, None);
            self.index_buffer.destroy(allocator);
            self.index_buffer_gpu.destroy(allocator);
            self.uniform_buffer.destroy(allocator);
            self.uniform_buffer_gpu.destroy(allocator);
            device.destroy_descriptor_set_layout(self.desc_set_layout, None);
        }
    }
}
