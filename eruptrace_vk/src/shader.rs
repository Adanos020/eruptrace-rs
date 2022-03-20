use erupt::{vk, DeviceLoader};

pub fn make_shader_module(device: &DeviceLoader, spv_bytes: &[u8]) -> vk::ShaderModule {
    let spv = erupt::utils::decode_spv(spv_bytes).expect("Cannot decode vertex shader");
    let module_info = vk::ShaderModuleCreateInfoBuilder::new().code(&spv);
    unsafe {
        device
            .create_shader_module(&module_info, None)
            .expect("Cannot create vertex shader module")
    }
}
