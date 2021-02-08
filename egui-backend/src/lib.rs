mod shaders;

use epi::egui;
use std::rc::Rc;
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};

use shaders::Shaders;

use safe_vk::vk;
use safe_vk::MemoryUsage;

/// Enum for selecting the right buffer type.
#[derive(Debug)]
enum BufferType {
    Uniform,
    Index,
    Vertex,
}

/// Information about the screen used for rendering.
pub struct ScreenDescriptor {
    /// Width of the window in physical pixel.
    pub physical_width: u32,
    /// Height of the window in physical pixel.
    pub physical_height: u32,
    /// HiDPI scale factor.
    pub scale_factor: f32,
}

impl ScreenDescriptor {
    fn logical_size(&self) -> (u32, u32) {
        let logical_width = self.physical_width as f32 / self.scale_factor;
        let logical_height = self.physical_height as f32 / self.scale_factor;
        (logical_width as u32, logical_height as u32)
    }
}

/// Uniform buffer used when rendering.
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
struct UniformBuffer {
    screen_size: [f32; 2],
}

/// RenderPass to render a egui based GUI.
pub struct UiPass {
    graphics_pipeline: safe_vk::GraphicsPipeline,
    index_buffers: Vec<safe_vk::Buffer>,
    vertex_buffers: Vec<safe_vk::Buffer>,
    uniform_buffer: safe_vk::Buffer,
    uniform_descriptor_set: safe_vk::DescriptorSet,
    texture_descriptor_set_layout: safe_vk::DescriptorSetLayout,
    texture_descriptor_set: Option<safe_vk::DescriptorSet>,
    texture_version: Option<u64>,
    next_user_texture_id: u64,
    pending_user_textures: Vec<(u64, egui::Texture)>,
    user_textures: Vec<Option<safe_vk::DescriptorSet>>,
    allocator: Arc<safe_vk::Allocator>,
    render_pass: safe_vk::RenderPass,
}

impl UiPass {
    /// Creates a new render pass to render a egui UI. `output_format` needs to be either `wgpu::TextureFormat::Rgba8UnormSrgb` or `wgpu::TextureFormat::Bgra8UnormSrgb`. Panics if it's not a Srgb format.
    pub fn new(allocator: Arc<safe_vk::Allocator>) -> Self {
        let device = allocator.device();
        let vs_module =
            safe_vk::ShaderModule::new(device.clone(), Shaders::get("egui.vert.spv").unwrap());
        let fs_module =
            safe_vk::ShaderModule::new(device.clone(), Shaders::get("egui.frag.spv").unwrap());

        let uniform_buffer = safe_vk::Buffer::new(
            allocator.clone(),
            std::mem::size_of::<UniformBuffer>(),
            vk::BufferUsageFlags::UNIFORM_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            MemoryUsage::CpuToGpu,
        );

        let sampler = safe_vk::Sampler::new(device.clone());

        let uniform_descriptor_set_layout = safe_vk::DescriptorSetLayout::new(
            device.clone(),
            &[
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(0)
                    .stage_flags(vk::ShaderStageFlags::VERTEX)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .descriptor_count(1)
                    .build(),
                vk::DescriptorSetLayoutBinding::builder()
                    .binding(1)
                    .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                    .descriptor_type(vk::DescriptorType::SAMPLER)
                    .descriptor_count(1)
                    .build(),
            ],
        );

        let texture_descriptor_set_layout = safe_vk::DescriptorSetLayout::new(
            device.clone(),
            &[vk::DescriptorSetLayoutBinding::builder()
                .binding(0)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                .descriptor_count(1)
                .build()],
        );

        let pipeline_layout = Arc::new(safe_vk::PipelineLayout::new(
            device.clone(),
            &[
                &uniform_descriptor_set_layout,
                &texture_descriptor_set_layout,
            ],
        ));

        let graphics_pipeline = safe_vk::GraphicsPipeline::new(
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
            &vk::PipelineVertexInputStateCreateInfo::builder()
                .vertex_binding_descriptions(&[vk::VertexInputBindingDescription::builder()
                    .stride(5 * 4)
                    .input_rate(vk::VertexInputRate::VERTEX)
                    .binding(0)
                    .build()])
                .vertex_attribute_descriptions(&[
                    vk::VertexInputAttributeDescription::builder()
                        .binding(0)
                        .location(0)
                        .format(vk::Format::R32G32_SFLOAT)
                        .offset(0)
                        .build(),
                    vk::VertexInputAttributeDescription::builder()
                        .binding(0)
                        .location(1)
                        .format(vk::Format::R32G32_SFLOAT)
                        .offset(4 * 2)
                        .build(),
                    vk::VertexInputAttributeDescription::builder()
                        .binding(0)
                        .location(0)
                        .format(vk::Format::R32_UINT)
                        .offset(4 * 4)
                        .build(),
                ])
                .build(),
            &vk::PipelineInputAssemblyStateCreateInfo::builder()
                .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
                .build(),
            &vk::PipelineRasterizationStateCreateInfo::builder()
                .cull_mode(vk::CullModeFlags::NONE)
                .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
                .polygon_mode(vk::PolygonMode::FILL)
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
        );

        let descriptor_pool = Arc::new(safe_vk::DescriptorPool::new(
            device.clone(),
            &[vk::DescriptorPoolSize::builder()
                .ty(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(1)
                .build()],
            1,
        ));

        let uniform_descriptor_set =
            safe_vk::DescriptorSet::new(descriptor_pool.clone(), &uniform_descriptor_set_layout);

        let render_pass = safe_vk::RenderPass::new(
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
                .build(),
        );

        Self {
            graphics_pipeline,
            vertex_buffers: Vec::with_capacity(64),
            index_buffers: Vec::with_capacity(64),
            uniform_buffer,
            uniform_descriptor_set,
            texture_descriptor_set_layout,
            texture_version: None,
            texture_descriptor_set: None,
            next_user_texture_id: 0,
            pending_user_textures: Vec::new(),
            user_textures: Vec::new(),
            render_pass,
            allocator,
        }
    }
}
