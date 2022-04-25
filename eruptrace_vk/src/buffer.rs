use std::sync::{Arc, RwLock};

use erupt::vk;
use vk_mem_erupt as vma;

#[derive(Clone)]
pub struct AllocatedBuffer<T: Sized> {
    pub buffer: vk::Buffer,
    pub size:   vk::DeviceSize,

    pub allocator:       Arc<RwLock<vma::Allocator>>,
    pub allocation:      vma::Allocation,
    pub allocation_info: vma::AllocationInfo,
    _phantom:            std::marker::PhantomData<T>,
}

impl<T: Sized> AllocatedBuffer<T> {
    pub fn new(
        allocator: Arc<RwLock<vma::Allocator>>,
        buffer_info: &vk::BufferCreateInfoBuilder,
        usage: vma::MemoryUsage,
    ) -> Self {
        let allocation_info = vma::AllocationCreateInfo {
            usage,
            flags: vma::AllocationCreateFlags::DEDICATED_MEMORY | vma::AllocationCreateFlags::MAPPED,
            ..Default::default()
        };
        let (buffer, allocation, allocation_info) =
            allocator.read().unwrap().create_buffer(buffer_info, &allocation_info).expect("Cannot create buffer");
        allocator.read().unwrap().flush_allocation(&allocation, 0, buffer_info.size as usize);
        Self { buffer, size: buffer_info.size, allocator, allocation, allocation_info, _phantom: Default::default() }
    }

    pub fn with_data(
        allocator: Arc<RwLock<vma::Allocator>>,
        buffer_info: &vk::BufferCreateInfoBuilder,
        usage: vma::MemoryUsage,
        data: &[T],
    ) -> Self {
        let data_size = std::mem::size_of::<T>() * data.len();
        let buffer_info = buffer_info.size(data_size as vk::DeviceSize);
        let buf = Self::new(allocator, &buffer_info, usage);
        buf.set_data(data);
        buf
    }

    pub fn memory_ptr(&self) -> *mut u8 {
        self.allocator.read().unwrap().map_memory(&self.allocation).expect("Cannot map allocated memory")
    }

    pub fn set_data(&self, data: &[T]) {
        self.set_data_at(0, data);
    }

    pub fn set_data_at(&self, start: usize, data: &[T]) {
        let data_size = std::mem::size_of::<T>() * data.len();
        assert!(start + data_size <= self.allocation_info.get_size());
        let buffer_addr = self.memory_ptr();
        assert_ne!(buffer_addr, std::ptr::null_mut());
        unsafe {
            let bytes = std::slice::from_raw_parts(data.as_ptr() as *const u8, data_size);
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), (buffer_addr as usize + start) as *mut u8, data_size);
        }
        self.allocator.read().unwrap().unmap_memory(&self.allocation);
    }

    pub fn destroy(&self) {
        self.allocator.read().unwrap().destroy_buffer(self.buffer, &self.allocation);
    }
}
