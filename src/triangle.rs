use anyhow::{anyhow, bail, Result};
use ash::version::{DeviceV1_0, EntryV1_0, InstanceV1_0};
use ash::vk;

pub struct Triangle {
    pipeline: vk::Pipeline,
    render_pass: vk::RenderPass,
}

impl Triangle {
    pub fn new(device: &ash::Device, clear_color: Option<vk::ClearColorValue>, format: vk::Format) {
        let load_op = if clear_color.is_some() {
            vk::AttachmentLoadOp::CLEAR
        } else {
            vk::AttachmentLoadOp::LOAD
        };
        let render_pass_info = vk::RenderPassCreateInfo::builder()
            .attachments(&[vk::AttachmentDescription::builder()
                .load_op(load_op)
                .store_op(vk::AttachmentStoreOp::STORE)
                .initial_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .final_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .format(format)
                .samples(vk::SampleCountFlags::TYPE_1)
                .build()])
            .build();
    }
}
