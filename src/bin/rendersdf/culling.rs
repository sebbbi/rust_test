use std::default::Default;
use std::ffi::CString;
use std::io::Cursor;
use std::mem;

use ash::util::*;
use ash::{vk, Device};

use crate::minivector::*;
use crate::vulkan_helpers::*;

use gpu_allocator::vulkan::*;
use gpu_allocator::MemoryLocation;

#[derive(Clone, Copy)]
pub struct VisibilityData {
    pub index: u32,
}

#[derive(Clone, Copy)]
pub struct DrawIndexedIndirectArguments {
    pub index_count: u32,
    pub instance_count: u32,
    pub first_index: u32,
    pub vertex_offset: i32,
    pub first_instance: u32,
}

#[derive(Clone, Copy)]
pub struct CullingUniforms {
    pub world_to_screen: Mat4x4,
    pub depth_pyramid_dimension: u32, // pow2 y dimension of mip 0 (texture x is 1.5x wider)
}

pub struct Culling {
    pub pipeline_layout: vk::PipelineLayout,
    pub uniform_buffer: VkBuffer,
    pub uniform_buffer_gpu: VkBuffer,
    pub visibility_buffer: VkBuffer,
    pub visibility_arguments: VkBuffer,
    pub visibility_buffer_descriptor: vk::DescriptorBufferInfo,
    pub desc_set_layout: vk::DescriptorSetLayout,
    pub descriptor_sets: Vec<vk::DescriptorSet>,
    pub compute_pipeline: vk::Pipeline,
    pub compute_shader_module: vk::ShaderModule,
}

impl Culling {
    pub fn new(
        device: &Device,
        allocator: &mut Allocator,
        descriptor_pool: &vk::DescriptorPool,
        depth_pyramid_descriptor: &vk::DescriptorImageInfo,
        depth_pyramid_debug_descriptor: &vk::DescriptorImageInfo,
        instances_buffer_descriptor: &vk::DescriptorBufferInfo,
        num_instances: usize,
    ) -> Culling {
        let visibility_buffer_info = vk::BufferCreateInfo {
            size: (std::mem::size_of::<VisibilityData>() * num_instances) as u64,
            usage: vk::BufferUsageFlags::STORAGE_BUFFER,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let visibility_buffer = VkBuffer::new(
            device,
            allocator,
            &visibility_buffer_info,
            MemoryLocation::CpuToGpu,
        );

        let visibility_buffer_descriptor = vk::DescriptorBufferInfo {
            buffer: visibility_buffer.buffer,
            offset: 0,
            range: (std::mem::size_of::<VisibilityData>() * num_instances) as u64,
        };

        let visibility_arguments_info = vk::BufferCreateInfo {
            size: std::mem::size_of::<DrawIndexedIndirectArguments>() as u64,
            usage: vk::BufferUsageFlags::STORAGE_BUFFER
                | vk::BufferUsageFlags::TRANSFER_DST
                | vk::BufferUsageFlags::INDIRECT_BUFFER,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let visibility_arguments = VkBuffer::new(
            device,
            allocator,
            &visibility_arguments_info,
            MemoryLocation::GpuOnly,
        );

        let visibility_arguments_descriptor = vk::DescriptorBufferInfo {
            buffer: visibility_arguments.buffer,
            offset: 0,
            range: std::mem::size_of::<DrawIndexedIndirectArguments>() as u64,
        };

        let uniform_buffer_info = vk::BufferCreateInfo {
            size: std::mem::size_of::<CullingUniforms>() as u64,
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
            size: std::mem::size_of::<CullingUniforms>() as u64,
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
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::COMPUTE,
                ..Default::default()
            },
            vk::DescriptorSetLayoutBinding {
                binding: 3,
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::COMPUTE,
                ..Default::default()
            },
            vk::DescriptorSetLayoutBinding {
                binding: 4,
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::COMPUTE,
                ..Default::default()
            },
            vk::DescriptorSetLayoutBinding {
                binding: 5,
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
            range: mem::size_of::<CullingUniforms>() as u64,
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
                p_image_info: depth_pyramid_descriptor,
                ..Default::default()
            },
            vk::WriteDescriptorSet {
                dst_set: descriptor_sets[0],
                dst_binding: 2,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                p_buffer_info: instances_buffer_descriptor,
                ..Default::default()
            },
            vk::WriteDescriptorSet {
                dst_set: descriptor_sets[0],
                dst_binding: 3,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                p_buffer_info: &visibility_buffer_descriptor,
                ..Default::default()
            },
            vk::WriteDescriptorSet {
                dst_set: descriptor_sets[0],
                dst_binding: 4,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
                p_buffer_info: &visibility_arguments_descriptor,
                ..Default::default()
            },
            vk::WriteDescriptorSet {
                dst_set: descriptor_sets[0],
                dst_binding: 5,
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

        let compute_shader_module = {
            let mut comp_spv_file = Cursor::new(&include_bytes!("../../../shader/culling.spv"));
            let comp_code =
                read_spv(&mut comp_spv_file).expect("Failed to read compute shader spv file");
            let comp_shader_info = vk::ShaderModuleCreateInfo::builder().code(&comp_code);

            unsafe { device.create_shader_module(&comp_shader_info, None) }
                .expect("Fragment shader module error")
        };

        let shader_entry_name = CString::new("main").unwrap();

        let compute_pipeline_info = {
            let shader_stage_create_info = vk::PipelineShaderStageCreateInfo {
                module: compute_shader_module,
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
                &[compute_pipeline_info],
                None,
            )
        }
        .unwrap();

        let compute_pipeline = compute_pipelines[0];

        Culling { 
            pipeline_layout,
            uniform_buffer,
            uniform_buffer_gpu,
            visibility_buffer,
            visibility_arguments,
            visibility_buffer_descriptor,
            desc_set_layout,
            descriptor_sets,
            compute_pipeline,
            compute_shader_module,
        }
    }

    pub fn gpu_setup(&self, device: &Device, command_buffer: &vk::CommandBuffer) {
        unsafe {
            // Lazy way to init the argument buffer :)
            let arguments = [0, 1, 0, 0, 0];
            for (a, i) in arguments.iter().enumerate() {
                device.cmd_fill_buffer(
                    *command_buffer,
                    self.visibility_arguments.buffer,
                    (i * std::mem::size_of::<u32>()) as u64,
                    std::mem::size_of::<u32>() as u64,
                    a as u32,
                );
            }
        }
    }

    pub fn update(&self, uniforms: &CullingUniforms) {
        self.uniform_buffer.copy_from_slice(&[*uniforms], 0);
    }

    pub fn gpu_draw(
        &self,
        device: &Device,
        command_buffer: &vk::CommandBuffer,
        pyramid_image: &vk::Image,
        pyramid_debug_image: &vk::Image,
        num_instances: u32,
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
       
        let arguments_barrier_clear = vk::BufferMemoryBarrier {
            //src_access_mask: vk::AccessFlags::INDIRECT_COMMAND_READ,
            dst_access_mask: vk::AccessFlags::TRANSFER_WRITE,
            buffer: self.visibility_arguments.buffer,
            offset: 0,
            size: std::mem::size_of::<DrawIndexedIndirectArguments>() as u64,
            ..Default::default()
        };        

        let arguments_barrier_compute = vk::BufferMemoryBarrier {
            src_access_mask: vk::AccessFlags::TRANSFER_WRITE,
            dst_access_mask: vk::AccessFlags::SHADER_READ | vk::AccessFlags::SHADER_WRITE,
            buffer: self.visibility_arguments.buffer,
            offset: 0,
            size: std::mem::size_of::<DrawIndexedIndirectArguments>() as u64,
            ..Default::default()
        };

        let arguments_barrier_indirect = vk::BufferMemoryBarrier {
            src_access_mask: vk::AccessFlags::SHADER_READ | vk::AccessFlags::SHADER_WRITE,
            dst_access_mask: vk::AccessFlags::INDIRECT_COMMAND_READ,
            buffer: self.visibility_arguments.buffer,
            offset: 0,
            size: std::mem::size_of::<DrawIndexedIndirectArguments>() as u64,
            ..Default::default()
        };

        let barrier_pyramid_read = vk::ImageMemoryBarrier {
            src_access_mask: vk::AccessFlags::SHADER_READ | vk::AccessFlags::SHADER_WRITE,
            dst_access_mask: vk::AccessFlags::SHADER_READ,
            old_layout: vk::ImageLayout::GENERAL,
            new_layout: vk::ImageLayout::GENERAL,
            image: *pyramid_image,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                level_count: 1,
                layer_count: 1,
                ..Default::default()
            },
            ..Default::default()
        };

        let barrier_pyramid_rw = vk::ImageMemoryBarrier {
            src_access_mask: vk::AccessFlags::SHADER_READ,
            dst_access_mask: vk::AccessFlags::SHADER_READ | vk::AccessFlags::SHADER_WRITE,
            old_layout: vk::ImageLayout::GENERAL,
            new_layout: vk::ImageLayout::GENERAL,
            image: *pyramid_image,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                level_count: 1,
                layer_count: 1,
                ..Default::default()
            },
            ..Default::default()
        };

        let clear_color = vk::ClearColorValue {
            uint32: [0, 0, 0, 0],
        };

        let image_subresource_range = vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
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

            device.cmd_clear_color_image(
                *command_buffer,
                *pyramid_debug_image,
                vk::ImageLayout::GENERAL,
                &clear_color,
                &[image_subresource_range],
            );

            // Indirect arguments to GPU fill (clear)
            device.cmd_pipeline_barrier(
                *command_buffer,
                vk::PipelineStageFlags::DRAW_INDIRECT,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &[],
                &[arguments_barrier_clear],
                &[],
            );

            // Clear the visible index count (remaining of the buffer stays)
            device.cmd_fill_buffer(
                *command_buffer,
                self.visibility_arguments.buffer,
                0,
                std::mem::size_of::<u32>() as u64,
                0,
            );

            // Pyramid to GPU read
            device.cmd_pipeline_barrier(
                *command_buffer,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier_pyramid_read],
            );

            // Indirect arguments to GPU write
            device.cmd_pipeline_barrier(
                *command_buffer,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[arguments_barrier_compute],
                &[],
            );

            // Culling compute shader
            device.cmd_bind_pipeline(
                *command_buffer,
                vk::PipelineBindPoint::COMPUTE,
                self.compute_pipeline,
            );

            device.cmd_bind_descriptor_sets(
                *command_buffer,
                vk::PipelineBindPoint::COMPUTE,
                self.pipeline_layout,
                0,
                &self.descriptor_sets[..],
                &[],
            );

            let group_dim = 64;
            let dim = num_instances / group_dim;
            device.cmd_dispatch(*command_buffer, dim, 1, 1);

            // Pyramid back to GPU read & write
            device.cmd_pipeline_barrier(
                *command_buffer,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier_pyramid_rw],
            );

            // Indirect arguments to indirect draw
            device.cmd_pipeline_barrier(
                *command_buffer,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::PipelineStageFlags::DRAW_INDIRECT,
                vk::DependencyFlags::empty(),
                &[],
                &[arguments_barrier_indirect],
                &[],
            );
        }
    }

    pub fn destroy(&mut self, device: &Device, allocator: &mut Allocator) {
        unsafe {
            self.visibility_arguments.destroy(device, allocator);
            self.visibility_buffer.destroy(device, allocator);
            self.uniform_buffer.destroy(device, allocator);
            self.uniform_buffer_gpu.destroy(device, allocator);
            device.destroy_pipeline_layout(self.pipeline_layout, None);
            device.destroy_descriptor_set_layout(self.desc_set_layout, None);
            device.destroy_pipeline(self.compute_pipeline, None);
            device.destroy_shader_module(self.compute_shader_module, None);
        }
    }
}
