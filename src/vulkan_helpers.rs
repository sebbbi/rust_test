extern crate ash;
extern crate vk_mem;

pub use ash::version::{DeviceV1_0, EntryV1_0, InstanceV1_0};
use ash::{vk, Device};
use std::ptr;
use std::slice::from_raw_parts_mut;
use vk_mem::{Allocator, MemoryUsage};

pub struct VkBuffer {
    pub buffer: vk::Buffer,
    pub allocation: vk_mem::Allocation,
    pub info: vk_mem::AllocationInfo,
    pub size: u64,
    pub mapped_ptr: *mut u8,
}

impl VkBuffer {
    pub fn new(
        allocator: &vk_mem::Allocator,
        buffer_info: &vk::BufferCreateInfo,
        allocation_info: &vk_mem::AllocationCreateInfo,
    ) -> VkBuffer {
        let size = buffer_info.size;

        let (buffer, allocation, info) = allocator
            .create_buffer(buffer_info, allocation_info)
            .expect("Buffer creation failed");

        let mapped_ptr = if allocation_info.usage == MemoryUsage::GpuOnly {
            ptr::null_mut()
        } else {
            info.get_mapped_data()
        };

        VkBuffer {
            buffer,
            allocation,
            info,
            size,
            mapped_ptr,
        }
    }

    pub fn destroy(&self, allocator: &Allocator) {
        allocator
            .destroy_buffer(self.buffer, &self.allocation)
            .expect("Buffer destroy failed");
    }

    pub fn copy_from_slice<T>(&self, slice: &[T], offset: usize)
    where
        T: Copy,
    {
        assert!(std::mem::size_of_val(slice) + offset <= self.info.get_size());
        assert!(!self.mapped_ptr.is_null());

        unsafe {
            let mem_ptr = self.mapped_ptr.add(offset);
            let mapped_slice = from_raw_parts_mut(mem_ptr as *mut T, slice.len());
            mapped_slice.copy_from_slice(slice);
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

pub struct VkViewScissor {
    pub viewport: vk::Viewport,
    pub scissor: vk::Rect2D,
}

pub struct VkUploadImageWithViewSampler {
    pub image: VkImage,
    pub upload_buffer: VkBuffer,
    pub sampler: vk::Sampler,
    pub view: vk::ImageView,
    pub descriptor: vk::DescriptorImageInfo,
}

impl VkUploadImageWithViewSampler {
    pub fn destroy(&self, device: &Device, allocator: &vk_mem::Allocator) {
        unsafe {
            device.destroy_image_view(self.view, None);
            self.image.destroy(allocator);
            self.upload_buffer.destroy(allocator);
            device.destroy_sampler(self.sampler, None);
        }
    }
}
