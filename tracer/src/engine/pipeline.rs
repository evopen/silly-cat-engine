use super::Vulkan;
use anyhow::Result;
use ash::version::DeviceV1_0;
use ash::vk;
use std::sync::Arc;
use vk::PipelineCache;

pub struct PipelineLayout {
    handle: vk::PipelineLayout,
    vulkan: Arc<Vulkan>,
}

impl PipelineLayout {
    pub fn new<P>(vulkan: Arc<Vulkan>, set_layouts: &[&DescriptorSetLayout]) -> Result<Self>
    where
        P: AsRef<[u8]>,
    {
        let set_layouts = set_layouts
            .iter()
            .map(|layout| layout.handle)
            .collect::<Vec<_>>();
        let info = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(set_layouts.as_slice())
            .build();
        unsafe {
            let handle = vulkan.device.create_pipeline_layout(&info, None)?;
            Ok(Self { handle, vulkan })
        }
    }
}

impl Drop for PipelineLayout {
    fn drop(&mut self) {
        unsafe {
            self.vulkan
                .device
                .destroy_pipeline_layout(self.handle, None);
        }
    }
}
