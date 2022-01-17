use ash::{vk, Device};
use std::default::Default;

use crate::vulkan_base::*;
use crate::vulkan_helpers::*;
use crate::SdfLevel;

pub struct SdfTexture {
    pub image: VkImage,
    pub upload_buffer: VkBuffer,
    pub sampler: vk::Sampler,
    pub view: vk::ImageView,
    pub descriptor: vk::DescriptorImageInfo,
}

impl SdfTexture {
    pub fn new(
        device: &Device,
        allocator: &vk_mem::Allocator,
        sdf_levels: &[SdfLevel],
        sdf_total_voxels: usize,
    ) -> SdfTexture {
        let alloc_info_cpu = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::CpuOnly,
            flags: vk_mem::AllocationCreateFlags::MAPPED,
            ..Default::default()
        };

        let alloc_info_gpu = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::GpuOnly,
            ..Default::default()
        };

        let image_buffer_info = vk::BufferCreateInfo {
            size: (std::mem::size_of::<u16>() * sdf_total_voxels) as u64,
            usage: vk::BufferUsageFlags::TRANSFER_SRC,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let upload_buffer = VkBuffer::new(allocator, &image_buffer_info, &alloc_info_cpu);

        for level in sdf_levels {
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

        let image = VkImage::new(allocator, &texture_create_info, &alloc_info_gpu);

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

        let sampler = unsafe { device.create_sampler(&sampler_info, None).unwrap() };

        let view_info = vk::ImageViewCreateInfo {
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
        let view = unsafe { device.create_image_view(&view_info, None) }.unwrap();

        let descriptor = vk::DescriptorImageInfo {
            image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            image_view: view,
            sampler,
        };

        SdfTexture {
            image,
            upload_buffer,
            sampler,
            view,
            descriptor,
        }
    }

    pub fn gpu_setup(
        &self,
        device: &Device,
        command_buffer: &vk::CommandBuffer,
        sdf_levels: &[SdfLevel],
    ) {
        // Setup distance field texture
        let texture_barrier = vk::ImageMemoryBarrier {
            dst_access_mask: vk::AccessFlags::TRANSFER_WRITE,
            new_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            image: self.image.image,
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
                    .buffer_offset(std::mem::size_of::<u16>() as u64 * sdf_levels[i].offset as u64)
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
            image: self.image.image,
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
                *command_buffer,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[texture_barrier],
            );

            device.cmd_copy_buffer_to_image(
                *command_buffer,
                self.upload_buffer.buffer,
                self.image.image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &image_copys[..],
            );

            device.cmd_pipeline_barrier(
                *command_buffer,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[texture_barrier_end],
            );
        };
    }

    pub fn destroy(&self, device: &Device, allocator: &vk_mem::Allocator) {
        unsafe {
            device.destroy_image_view(self.view, None);
            self.image.destroy(allocator);
            self.upload_buffer.destroy(allocator);
            device.destroy_sampler(self.sampler, None);
        }
    }
}
