use std::default::Default;
use std::ffi::CString;
use std::io::Cursor;
use std::mem;

use ash::util::*;
use ash::{vk, Device};

use crate::vulkan_base::*;
use crate::vulkan_helpers::*;

#[derive(Clone, Copy)]
pub struct CullingPushConstants {
    pub mip: u32,
}

#[derive(Clone, Copy)]
pub struct CullingUniforms {
    pub depth_buffer_dimensions: (u32, u32),
    pub depth_pyramid_dimension: u32, // pow2 y dimension of mip 0 (texture x is 1.5x wider)
}

pub struct Culling {
    pub pipeline_layout: vk::PipelineLayout,
    pub uniform_buffer: VkBuffer,
    pub uniform_buffer_gpu: VkBuffer,
    pub desc_set_layout: vk::DescriptorSetLayout,
    pub descriptor_sets: Vec<vk::DescriptorSet>,
    pub compute_pipeline: vk::Pipeline,
    pub compute_shader_module: vk::ShaderModule,
}

impl Culling {
    pub fn new(
        device: &Device,
        allocator: &vk_mem::Allocator,
        descriptor_pool: &vk::DescriptorPool,
        depth_pyramid_view: &vk::ImageView,
        depth_pyramid_dimension: u32,
    ) -> Culling {
        let alloc_info_cpu = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::CpuOnly,
            flags: vk_mem::AllocationCreateFlags::MAPPED,
            ..Default::default()
        };

        let alloc_info_gpu = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::GpuOnly,
            ..Default::default()
        };

        let uniform_buffer_info = vk::BufferCreateInfo {
            size: std::mem::size_of::<DepthPyramidUniforms>() as u64,
            usage: vk::BufferUsageFlags::TRANSFER_SRC,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let uniform_buffer = VkBuffer::new(allocator, &uniform_buffer_info, &alloc_info_cpu);

        let uniform_buffer_gpu_info = vk::BufferCreateInfo {
            size: std::mem::size_of::<DepthPyramidUniforms>() as u64,
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
                stage_flags: vk::ShaderStageFlags::COMPUTE,
                ..Default::default()
            },
            vk::DescriptorSetLayoutBinding {
                binding: 1,
                descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::COMPUTE,
                ..Default::default()
            },
            vk::DescriptorSetLayoutBinding {
                binding: 2,
                descriptor_type: vk::DescriptorType::STORAGE_IMAGE,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::COMPUTE,
                ..Default::default()
            },
            vk::DescriptorSetLayoutBinding {
                binding: 3,
                descriptor_type: vk::DescriptorType::STORAGE_IMAGE,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::COMPUTE,
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
            range: mem::size_of::<DepthPyramidUniforms>() as u64,
        };

        let depth_pyramid_descriptor = vk::DescriptorImageInfo {
            image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            image_view: *depth_view,
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
                descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                p_image_info: &depth_image_descriptor,
                ..Default::default()
            },
            vk::WriteDescriptorSet {
                dst_set: descriptor_sets[0],
                dst_binding: 2,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::STORAGE_IMAGE,
                p_image_info: &descriptor,
                ..Default::default()
            },
            vk::WriteDescriptorSet {
                dst_set: descriptor_sets[0],
                dst_binding: 3,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::STORAGE_IMAGE,
                p_image_info: &descriptor,
                ..Default::default()
            },
        ];
        unsafe { device.update_descriptor_sets(&write_desc_sets, &[]) };

        let push_constants = [vk::PushConstantRange {
            stage_flags: vk::ShaderStageFlags::COMPUTE,
            offset: 0,
            size: std::mem::size_of::<DepthPyramidUniforms>() as u32,
        }];

        let layout_create_info = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(desc_set_layouts)
            .push_constant_ranges(&push_constants);

        let pipeline_layout =
            unsafe { device.create_pipeline_layout(&layout_create_info, None) }.unwrap();

        let compute_shader_module = {
            let mut comp_spv_file =
                Cursor::new(&include_bytes!("../shader/depth_pyramid_first_mip.spv"));
            let comp_code =
                read_spv(&mut comp_spv_file).expect("Failed to read compute shader spv file");
            let comp_shader_info = vk::ShaderModuleCreateInfo::builder().code(&comp_code);

            unsafe { device.create_shader_module(&comp_shader_info, None) }
                .expect("Fragment shader module error")
        };

        let shader_entry_name = CString::new("main").unwrap();

        let compute_pipeline_info = {
            let shader_stage_create_info = vk::PipelineShaderStageCreateInfo {
                module: compute_shader_module_pass_1,
                p_name: shader_entry_name.as_ptr(),
                stage: vk::ShaderStageFlags::COMPUTE,
                ..Default::default()
            };
            vk::ComputePipelineCreateInfo {
                stage: shader_stage_create_info,
                layout: pipeline_layout,
                ..Default::default()
            }
        };

        let compute_pipelines = unsafe {
            device.create_compute_pipelines(
                vk::PipelineCache::null(),
                &[
                    compute_pipeline_info,
                ],
                None,
            )
        }
        .unwrap();

        let compute_pipeline = compute_pipelines[0];

        Culling {
            pipeline_layout,
            uniform_buffer,
            uniform_buffer_gpu,
            desc_set_layout,
            descriptor_sets,
            image,
            sampler,
            view,
            descriptor,
            compute_pipeline,
            compute_shader_module_pass_1,
            compute_shader_module_downsample,
        }
    }

    pub fn gpu_setup(&self, device: &Device, command_buffer: &vk::CommandBuffer) {

    }

    pub fn update(&self, uniforms: &CullingUniforms) {
        self.uniform_buffer.copy_from_slice(&[*uniforms], 0);
    }

    pub fn gpu_draw(
        &self,
        device: &Device,
        command_buffer: &vk::CommandBuffer,
        depth_image: &vk::Image,
        pyramid_mip0_dimension: u32,
        num_mips: u32,
    ) {
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

        let barrier_pyramid_pass = vk::ImageMemoryBarrier {
            src_access_mask: vk::AccessFlags::SHADER_READ | vk::AccessFlags::SHADER_WRITE,
            dst_access_mask: vk::AccessFlags::SHADER_READ | vk::AccessFlags::SHADER_WRITE,
            old_layout: vk::ImageLayout::GENERAL,
            new_layout: vk::ImageLayout::GENERAL,
            image: self.image.image,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                level_count: 1,
                layer_count: 1,
                ..Default::default()
            },
            ..Default::default()
        };

        unsafe {
            // Update uniform buffer
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

            // First pass downsample compute shader
            device.cmd_bind_pipeline(
                *command_buffer,
                vk::PipelineBindPoint::COMPUTE,
                self.compute_pipeline_pass_1,
            );

            device.cmd_bind_descriptor_sets(
                *command_buffer,
                vk::PipelineBindPoint::COMPUTE,
                self.pipeline_layout,
                0,
                &self.descriptor_sets[..],
                &[],
            );

            let group_dim = (8, 8);
            let dim = (
                pyramid_mip0_dimension / group_dim.0,
                pyramid_mip0_dimension / group_dim.1,
            );
            device.cmd_dispatch(*command_buffer, dim.0, dim.1, 1);

            // Barrier between pyramid passes to avoid RaW hazards
            device.cmd_pipeline_barrier(
                *command_buffer,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier_pyramid_pass],
            );
        }
    }

    pub fn destroy(&self, device: &Device, allocator: &vk_mem::Allocator) {
        unsafe {
            self.uniform_buffer.destroy(&allocator);
            self.uniform_buffer_gpu.destroy(&allocator);
            device.destroy_pipeline_layout(self.pipeline_layout, None);
            device.destroy_descriptor_set_layout(self.desc_set_layout, None);
            device.destroy_pipeline(self.compute_pipeline, None);
            device.destroy_shader_module(self.compute_shader_module, None);
        }
    }
}