use std::sync::Arc;

use safe_vk::vk;

pub struct Quad {
    pipeline: Arc<safe_vk::GraphicsPipeline>,
    texture_descriptor_set: safe_vk::DescriptorSet,
    render_pass: Arc<safe_vk::RenderPass>,
}

impl Quad {
    pub fn new(device: Arc<safe_vk::Device>) -> Self {
        let sampler = safe_vk::Sampler::new(device.clone());
        let set_layout = safe_vk::DescriptorSetLayout::new(
            device.clone(),
            Some("quad set layout"),
            vec![
                safe_vk::DescriptorSetLayoutBinding {
                    binding: 1,
                    descriptor_type: safe_vk::DescriptorType::Sampler(Some(Arc::new(sampler))),
                    stage_flags: vk::ShaderStageFlags::FRAGMENT,
                },
                safe_vk::DescriptorSetLayoutBinding {
                    binding: 0,
                    descriptor_type: safe_vk::DescriptorType::SampledImage,
                    stage_flags: vk::ShaderStageFlags::FRAGMENT,
                },
            ],
        );
        let pipeline_layout = Arc::new(safe_vk::PipelineLayout::new(
            device.clone(),
            Some("quad pipeline layout"),
            &[&set_layout],
        ));
        let vs_module = safe_vk::ShaderModule::new(
            device.clone(),
            shader::Shaders::get("quad.vert.spv").unwrap(),
        );
        let fs_module = safe_vk::ShaderModule::new(
            device.clone(),
            shader::Shaders::get("quad.frag.spv").unwrap(),
        );

        let render_pass = Arc::new(safe_vk::RenderPass::new(
            device.clone(),
            &vk::RenderPassCreateInfo::builder()
                .attachments(&[vk::AttachmentDescription::builder()
                    .format(vk::Format::B8G8R8A8_UNORM)
                    .samples(vk::SampleCountFlags::TYPE_1)
                    .load_op(vk::AttachmentLoadOp::LOAD)
                    .store_op(vk::AttachmentStoreOp::STORE)
                    .initial_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                    .build()])
                .subpasses(&[vk::SubpassDescription::builder()
                    .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
                    .color_attachments(&[vk::AttachmentReference::builder()
                        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                        .attachment(0)
                        .build()])
                    .build()])
                .build(),
        ));
        let pipeline = Arc::new(safe_vk::GraphicsPipeline::new(
            Some("quad pipeline"),
            pipeline_layout,
            vec![
                Arc::new(safe_vk::ShaderStage::new(
                    vs_module,
                    vk::ShaderStageFlags::VERTEX,
                    "main",
                )),
                Arc::new(safe_vk::ShaderStage::new(
                    fs_module,
                    vk::ShaderStageFlags::FRAGMENT,
                    "main",
                )),
            ],
            render_pass.clone(),
            &vk::PipelineVertexInputStateCreateInfo::builder().build(),
            &vk::PipelineInputAssemblyStateCreateInfo::builder()
                .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
                .build(),
            &vk::PipelineRasterizationStateCreateInfo::builder()
                .cull_mode(vk::CullModeFlags::NONE)
                .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
                .polygon_mode(vk::PolygonMode::FILL)
                .line_width(1.0)
                .build(),
            &vk::PipelineMultisampleStateCreateInfo::builder()
                .rasterization_samples(vk::SampleCountFlags::TYPE_1)
                .build(),
            &vk::PipelineDepthStencilStateCreateInfo::default(),
            &vk::PipelineColorBlendStateCreateInfo::builder()
                .attachments(&[vk::PipelineColorBlendAttachmentState::builder()
                    .blend_enable(true)
                    .color_blend_op(vk::BlendOp::ADD)
                    .src_color_blend_factor(vk::BlendFactor::ONE)
                    .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                    .alpha_blend_op(vk::BlendOp::ADD)
                    .src_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_DST_ALPHA)
                    .dst_alpha_blend_factor(vk::BlendFactor::ONE)
                    .color_write_mask(vk::ColorComponentFlags::all())
                    .build()])
                .build(),
            &vk::PipelineViewportStateCreateInfo::builder()
                .viewport_count(1)
                .scissor_count(1),
            &vk::PipelineDynamicStateCreateInfo::builder()
                .dynamic_states(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR])
                .build(),
        ));

        let descriptor_pool = Arc::new(safe_vk::DescriptorPool::new(
            device.clone(),
            &[vk::DescriptorPoolSize::builder()
                .ty(vk::DescriptorType::SAMPLED_IMAGE)
                .descriptor_count(1)
                .build()],
            1,
        ));

        let texture_descriptor_set = safe_vk::DescriptorSet::new(
            Some("texture descriptor set"),
            descriptor_pool,
            Arc::new(set_layout),
        );

        Self {
            pipeline,
            texture_descriptor_set,
            render_pass,
        }
    }

    pub fn update_texture(&mut self, image_view: Arc<safe_vk::ImageView>) {
        self.texture_descriptor_set
            .update(&[safe_vk::DescriptorSetUpdateInfo {
                binding: 0,
                detail: safe_vk::DescriptorSetUpdateDetail::Image(image_view),
            }])
    }

    pub fn execute(
        &self,
        recorder: &mut safe_vk::CommandRecorder,
        color_attachment: Arc<safe_vk::ImageView>,
    ) {
        let framebuffer = Arc::new(safe_vk::Framebuffer::new(
            self.render_pass.clone(),
            color_attachment.image().width(),
            color_attachment.image().height(),
            vec![color_attachment.clone()],
        ));

        recorder.begin_render_pass(self.render_pass.clone(), framebuffer, |recorder| {
            recorder.bind_graphics_pipeline(self.pipeline.clone(), |recorder, pipeline| {
                recorder.draw(3, 1);
            });
        });
    }
}
