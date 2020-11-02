extern crate ash;
extern crate vk_mem;

use ash::util::*;
pub use ash::version::{DeviceV1_0, EntryV1_0, InstanceV1_0};
use ash::{vk, Device};
use std::default::Default;
use std::mem::{self, align_of};
use std::os::raw::c_void;
use vk_mem::{AllocationCreateInfo, Allocator};

pub struct VkBuffer {
    pub buffer: vk::Buffer,
    pub allocation: vk_mem::Allocation,
    pub info: vk_mem::AllocationInfo,
}

impl VkBuffer {
    pub fn new(
        allocator: &vk_mem::Allocator,
        buffer_info: &vk::BufferCreateInfo,
        allocation_info: &vk_mem::AllocationCreateInfo,
    ) -> VkBuffer {
        let (buffer, allocation, info) = allocator
            .create_buffer(buffer_info, allocation_info)
            .expect("Buffer creation failed");

        println!("Buffer::new size={} offset={}", info.get_size(), info.get_offset());

        VkBuffer {
            buffer,
            allocation,
            info,
        }
    }

    pub fn destroy(&self, allocator: &Allocator) {
        allocator
            .destroy_buffer(self.buffer, &self.allocation)
            .expect("Buffer destroy failed");
    }

    pub fn copy_from_slice<T>(&self, device: &Device, slice: &[T])
    where
        T: Copy,
    {
        unsafe {
            let index_ptr: *mut c_void = device
                .map_memory(
                    self.info.get_device_memory(),
                    self.info.get_offset() as u64,
                    self.info.get_size() as u64,
                    vk::MemoryMapFlags::empty(),
                )
                .unwrap();
            let mut index_slice = Align::new(
                index_ptr,
                align_of::<u32>() as u64,
                self.info.get_size() as u64,
            );
            index_slice.copy_from_slice(slice);
            device.unmap_memory(self.info.get_device_memory());
        }
    }
}

pub struct VkImage {
    pub image: vk::Image,
    pub allocation: vk_mem::Allocation,
    pub info: vk_mem::AllocationInfo,
}

impl VkImage {
    pub fn new(
        allocator: &vk_mem::Allocator,
        image_info: &vk::ImageCreateInfo,
        allocation_info: &vk_mem::AllocationCreateInfo,
    ) -> VkImage {
        let (image, allocation, info) = allocator
            .create_image(image_info, allocation_info)
            .expect("Image creation failed");
        VkImage {
            image,
            allocation,
            info,
        }
    }

    pub fn destroy(&self, allocator: &Allocator) {
        allocator
            .destroy_image(self.image, &self.allocation)
            .expect("Image destroy failed");
    }
}
