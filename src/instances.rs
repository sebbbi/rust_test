pub const NUM_INSTANCES: usize = 1024 * 1024;

use rand::Rng;
use std::default::Default;

use ash::{vk, Device};

use crate::minivector::*;
use crate::vulkan_helpers::*;

#[derive(Clone, Copy)]
pub struct InstanceData {
    pub position: Vec4,
}

pub struct Instances {
    pub instances_buffer: VkBuffer,
    pub instances_buffer_descriptor: vk::DescriptorBufferInfo,
}

impl Instances {
    pub fn new(_device: &Device, allocator: &vk_mem::Allocator) -> Instances {
        let alloc_info_cpu_to_gpu = vk_mem::AllocationCreateInfo {
            usage: vk_mem::MemoryUsage::CpuToGpu,
            flags: vk_mem::AllocationCreateFlags::MAPPED,
            ..Default::default()
        };

        let instances_buffer_info = vk::BufferCreateInfo {
            size: (std::mem::size_of::<InstanceData>() * NUM_INSTANCES) as u64,
            usage: vk::BufferUsageFlags::STORAGE_BUFFER,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let instances_buffer =
            VkBuffer::new(allocator, &instances_buffer_info, &alloc_info_cpu_to_gpu);

        let instances_buffer_descriptor = vk::DescriptorBufferInfo {
            buffer: instances_buffer.buffer,
            offset: 0,
            range: (std::mem::size_of::<InstanceData>() * NUM_INSTANCES) as u64,
        };

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

        Instances {
            instances_buffer,
            instances_buffer_descriptor,
        }
    }

    pub fn destroy(&self, _device: &Device, allocator: &vk_mem::Allocator) {
        self.instances_buffer.destroy(&allocator);
    }
}
