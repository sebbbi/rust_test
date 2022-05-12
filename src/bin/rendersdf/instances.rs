pub const NUM_INSTANCES: usize = 1024 * 1024;
pub const CLOUD_RADIUS: f32 = 8000.0;

use rand::Rng;
use rand::SeedableRng;
use std::default::Default;

use ash::{vk, Device};

use gpu_allocator::vulkan::*;
use gpu_allocator::MemoryLocation;

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
    pub fn new(device: &Device, allocator: &mut Allocator, instance_radius: f32) -> Instances {
        let instances_buffer_info = vk::BufferCreateInfo {
            size: (std::mem::size_of::<InstanceData>() * NUM_INSTANCES) as u64,
            usage: vk::BufferUsageFlags::STORAGE_BUFFER,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let instances_buffer = VkBuffer::new(
            device,
            allocator,
            &instances_buffer_info,
            MemoryLocation::CpuToGpu,
        );

        let instances_buffer_descriptor = vk::DescriptorBufferInfo {
            buffer: instances_buffer.buffer,
            offset: 0,
            range: (std::mem::size_of::<InstanceData>() * NUM_INSTANCES) as u64,
        };

        // Random cloud of SDF box instances
        //let mut rng = rand::thread_rng();
        let mut rng = rand::rngs::StdRng::from_seed([
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10,
            11, 12, 13, 14, 15,
        ]);
        let instances_buffer_data: Vec<InstanceData> = (0..NUM_INSTANCES)
            .map(|_i| InstanceData {
                position: Vec4 {
                    x: rng.gen_range(-CLOUD_RADIUS, CLOUD_RADIUS),
                    y: rng.gen_range(-CLOUD_RADIUS, CLOUD_RADIUS),
                    z: rng.gen_range(-CLOUD_RADIUS, CLOUD_RADIUS),
                    w: instance_radius,
                },
            })
            .collect();

        instances_buffer.copy_from_slice(&instances_buffer_data[..], 0);

        Instances {
            instances_buffer,
            instances_buffer_descriptor,
        }
    }

    pub fn destroy(&mut self, device: &Device, allocator: &mut Allocator) {
        self.instances_buffer.destroy(device, allocator);
    }
}
