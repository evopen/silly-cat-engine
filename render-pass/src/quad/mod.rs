use std::sync::Arc;

use safe_vk::vk;

pub struct Quad {
    pipeline: safe_vk::GraphicsPipeline,
}

impl Quad {
    pub fn new(device: Arc<safe_vk::Device>) -> Self {
        let set_layout = safe_vk::DescriptorSetLayout::new(device.clone(), Some("quad set layout"), &[vk::DescriptorSetLayoutBinding::builder().build()]);
        let pipeline_layout = Arc::new(safe_vk::PipelineLayout::new(device.clone(), Some("quad pipeline layout"), &[&]));
        let pipeline = safe_vk::GraphicsPipeline::new(Some("quad pipeline"), );
    }
}
