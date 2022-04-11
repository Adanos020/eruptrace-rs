use erupt::{vk, DeviceLoader};
use eruptrace_vk::AllocatedImage;

#[derive(Clone)]
pub struct GBuffers {
    pub out_positions: AllocatedImage,
    pub out_normals:   AllocatedImage,
    pub out_materials: AllocatedImage,
}

impl GBuffers {
    pub fn create_colour_attachment_infos(&self) -> Vec<vk::RenderingAttachmentInfoBuilder> {
        let make_attachment_info = |view| {
            vk::RenderingAttachmentInfoBuilder::new()
                .image_view(view)
                .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .clear_value(vk::ClearValue { color: vk::ClearColorValue { float32: [0.0, 0.0, 0.0, 0.0] } })
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
        };
        vec![
            make_attachment_info(self.out_positions.view),
            make_attachment_info(self.out_normals.view),
            make_attachment_info(self.out_materials.view),
        ]
    }

    pub fn destroy(&self, device: &DeviceLoader) {
        self.out_positions.destroy(device);
        self.out_normals.destroy(device);
        self.out_materials.destroy(device);
    }
}
