use erupt::vk;
use std::sync::{Arc, RwLock};
use vk_mem_erupt as vma;

#[derive(Clone)]
pub struct AllocatedBuffer<T: Sized> {
    pub buffer: vk::Buffer,
    pub size: vk::DeviceSize,

    pub allocator: Arc<RwLock<vma::Allocator>>,
    pub allocation: vma::Allocation,
    pub allocation_info: vma::AllocationInfo,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Sized> AllocatedBuffer<T> {
    pub fn with_data(
        allocator: Arc<RwLock<vma::Allocator>>,
        buffer_info: &vk::BufferCreateInfoBuilder,
        allocation_info: vma::AllocationCreateInfo,
        data: &[T],
    ) -> vma::Result<Self> {
        let data_size = std::mem::size_of::<T>() * data.len();
        let buffer_info = buffer_info.size(data_size as vk::DeviceSize);
        let (buffer, allocation, allocation_info) = allocator
            .read()
            .unwrap()
            .create_buffer(&buffer_info, &allocation_info)?;
        allocator
            .read()
            .unwrap()
            .flush_allocation(&allocation, 0, data_size);
        let mut buf = Self {
            buffer,
            size: buffer_info.size,
            allocator,
            allocation,
            allocation_info,
            _phantom: Default::default(),
        };
        buf.set_data(data);
        Ok(buf)
    }

    pub fn set_data(&mut self, data: &[T]) {
        let data_size = std::mem::size_of::<T>() * data.len();
        assert!(data_size <= self.allocation_info.get_size());
        let buffer_addr = self
            .allocator
            .read()
            .unwrap()
            .map_memory(&self.allocation)
            .expect("Cannot map allocated memory");
        assert_ne!(buffer_addr, std::ptr::null_mut());
        unsafe {
            let bytes = std::slice::from_raw_parts(data.as_ptr() as *const u8, data_size);
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer_addr, data_size);
        }
        self.allocator
            .read()
            .unwrap()
            .unmap_memory(&self.allocation);
    }

    pub fn destroy(&self) {
        self.allocator
            .read()
            .unwrap()
            .destroy_buffer(self.buffer, &self.allocation);
    }
}