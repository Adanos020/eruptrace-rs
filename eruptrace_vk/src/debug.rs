use std::ffi::{c_void, CStr};

use erupt::vk;

pub unsafe extern "system" fn debug_callback(
    severity: vk::DebugUtilsMessageSeverityFlagBitsEXT,
    _types: vk::DebugUtilsMessageTypeFlagsEXT,
    callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut c_void,
) -> vk::Bool32 {
    if callback_data.is_null() {
        return vk::FALSE;
    }
    if severity != vk::DebugUtilsMessageSeverityFlagBitsEXT::VERBOSE_EXT {
        let callback_data = &*callback_data;
        eprintln!("Validation Message: [{}]", CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy(),);
        eprintln!("Affected objects:");
        let objects = std::slice::from_raw_parts(callback_data.p_objects, callback_data.object_count as _);
        for (i, object) in objects.iter().enumerate() {
            eprintln!("- Object {}: handle = {:#x}, type = {:?}", i, object.object_handle, object.object_type);
        }
        eprintln!(
            "{}\n-------------------------------------",
            CStr::from_ptr(callback_data.p_message).to_string_lossy().split("| ").last().unwrap()
        );
    }
    vk::FALSE
}
