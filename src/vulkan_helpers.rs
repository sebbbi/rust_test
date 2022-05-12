extern crate ash;
extern crate gpu_allocator;

use ash::vk;
pub use ash::{Device, Instance};
use gpu_allocator::vulkan::*;
use gpu_allocator::MemoryLocation;
use std::ptr;
use std::slice::{from_raw_parts, from_raw_parts_mut};

pub struct VkBuffer {
    pub buffer: vk::Buffer,
    pub allocation: Option<Allocation>,
    pub size: u64,
    pub mapped_ptr: *mut u8,
}

impl VkBuffer {
    pub fn new(
        device: &Device,
        allocator: &mut Allocator,
        buffer_info: &vk::BufferCreateInfo,
        location: MemoryLocation,
    ) -> VkBuffer {
        let size = buffer_info.size;

        let buffer = unsafe { device.create_buffer(buffer_info, None) }.unwrap();
        let requirements = unsafe { device.get_buffer_memory_requirements(buffer) };

        let allocation = allocator
            .allocate(&AllocationCreateDesc {
                name: "buffer",
                requirements,
                location,
                linear: true,
            })
            .unwrap();

        unsafe {
            device
                .bind_buffer_memory(buffer, allocation.memory(), allocation.offset())
                .unwrap()
        };

        let mapped_ptr = if location == MemoryLocation::GpuOnly {
            ptr::null_mut()
        } else {
            allocation.mapped_ptr().unwrap().as_ptr() as *mut u8
        };

        VkBuffer {
            buffer,
            allocation: Some(allocation),
            size,
            mapped_ptr,
        }
    }

    pub fn destroy(&mut self, device: &Device, allocator: &mut Allocator) {
        allocator.free(self.allocation.take().unwrap()).unwrap();
        unsafe { device.destroy_buffer(self.buffer, None) };
    }

    pub fn copy_from_slice<T>(&self, slice: &[T], offset: usize)
    where
        T: Copy,
    {
        //assert!(std::mem::size_of_val(slice) + offset <= self.info.get_size());
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
    pub allocation: Option<Allocation>,
}

impl VkImage {
    pub fn new(
        device: &Device,
        allocator: &mut Allocator,
        image_info: &vk::ImageCreateInfo,
        location: MemoryLocation,
    ) -> VkImage {
        let image = unsafe { device.create_image(image_info, None) }.unwrap();
        let requirements = unsafe { device.get_image_memory_requirements(image) };

        let mut allocation = allocator.allocate(&AllocationCreateDesc {
            name: "image",
            requirements,
            location,
            linear: false,
        });

        // Workaround for gpu_allocator DEVICE_LOCAL memory running out on iGPUs
        // TODO: Remove once gpu_allocator handles iGPUs properly!
        if allocation.is_err() {
            allocation = allocator.allocate(&AllocationCreateDesc {
                name: "image",
                requirements,
                location: MemoryLocation::CpuToGpu,
                linear: false,
            });
        };

        let allocation = allocation.unwrap();

        unsafe {
            device
                .bind_image_memory(image, allocation.memory(), allocation.offset())
                .unwrap()
        };

        VkImage {
            image,
            allocation: Some(allocation),
        }
    }

    pub fn destroy(&mut self, device: &Device, allocator: &mut Allocator) {
        allocator.free(self.allocation.take().unwrap()).unwrap();
        unsafe { device.destroy_image(self.image, None) };
    }
}

pub struct VkViewScissor {
    pub viewport: vk::Viewport,
    pub scissor: vk::Rect2D,
}

pub fn raw_bytes<T>(data: &[T]) -> &[u8]
where
    T: Copy,
{
    unsafe {
        from_raw_parts(
            data.as_ptr() as *const u8,
            data.len() * std::mem::size_of::<T>(),
        )
    }
}
