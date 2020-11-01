extern crate ash;
extern crate vk_mem;

use ash::vk;
use vk_mem::{Allocator, AllocationCreateInfo};
use std::default::Default;

pub struct VkBuffer {
	pub buffer: vk::Buffer,
	pub allocation: vk_mem::Allocation,
	pub info: vk_mem::AllocationInfo,
}

impl VkBuffer {
	fn new(allocator: &vk_mem::Allocator, buffer_info: &vk::BufferCreateInfo, allocation_info: &vk_mem::AllocationCreateInfo) -> VkBuffer {
		let (buffer, allocation, info) = allocator.create_buffer(buffer_info, allocation_info).expect("Buffer creation failed");
		VkBuffer {
			buffer,	
			allocation,	
			info
		}
	}

	fn destroy(&self, allocator: &Allocator) {
		allocator.destroy_buffer(self.buffer, &self.allocation).expect("Buffer destroy failed");
	}
}

pub struct VkImage {
	pub image: vk::Image,
	pub allocation: vk_mem::Allocation,
	pub info: vk_mem::AllocationInfo,
}

impl VkImage {
	fn new(allocator: &vk_mem::Allocator, image_info: &vk::ImageCreateInfo, allocation_info: &vk_mem::AllocationCreateInfo) -> VkImage {
		let (image, allocation, info) = allocator.create_image(image_info, allocation_info).expect("Image creation failed");
		VkImage {
			image,	
			allocation,	
			info
		}
	}

	fn destroy(&self, allocator: &Allocator) {
		allocator.destroy_image(self.image, &self.allocation).expect("Image destroy failed");
	}
}



