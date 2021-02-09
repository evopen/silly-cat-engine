mod shaders;

use epi::egui;
use std::rc::Rc;
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};

use shaders::Shaders;

use safe_vk::{vk, CommandBuffer, CommandRecorder, DescriptorSet, Framebuffer, ImageView};
use safe_vk::{Image, MemoryUsage};

use safe_vk::Pipeline;

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
    graphics_pipeline: Arc<safe_vk::GraphicsPipeline>,
    index_buffers: Vec<Arc<safe_vk::Buffer>>,
    vertex_buffers: Vec<Arc<safe_vk::Buffer>>,
    uniform_buffer: safe_vk::Buffer,
    uniform_descriptor_set: Arc<safe_vk::DescriptorSet>,
    texture_descriptor_set_layout: safe_vk::DescriptorSetLayout,
    texture_descriptor_set: Option<Arc<safe_vk::DescriptorSet>>,
    texture_version: Option<u64>,
    next_user_texture_id: u64,
    pending_user_textures: Vec<(u64, egui::Texture)>,
    user_textures: Vec<Option<Arc<safe_vk::DescriptorSet>>>,
    allocator: Arc<safe_vk::Allocator>,
    render_pass: Arc<safe_vk::RenderPass>,
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

        let graphics_pipeline = Arc::new(safe_vk::GraphicsPipeline::new(
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
                        .location(2)
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
                .ty(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(1)
                .build()],
            1,
        ));

        let uniform_descriptor_set = Arc::new(safe_vk::DescriptorSet::new(
            descriptor_pool.clone(),
            &uniform_descriptor_set_layout,
        ));

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

    pub fn execute(
        &mut self,
        recorder: &mut CommandRecorder,
        color_attachment: Arc<Image>,
        paint_jobs: &[egui::paint::PaintJob],
        screen_descriptor: &ScreenDescriptor,
    ) {
        let image_view = Arc::new(ImageView::new(color_attachment.clone()));
        let framebuffer = Arc::new(Framebuffer::new(
            self.render_pass.clone(),
            screen_descriptor.physical_width,
            screen_descriptor.physical_height,
            vec![image_view.clone()],
        ));

        let scale_factor = screen_descriptor.scale_factor;
        let physical_width = screen_descriptor.physical_width;
        let physical_height = screen_descriptor.physical_height;

        recorder.begin_render_pass(self.render_pass.clone(), framebuffer.clone(), |recorder| {
            recorder.bind_graphics_pipeline(
                self.graphics_pipeline.clone(),
                |recorder, pipeline| {
                    recorder.bind_descriptor_sets(
                        vec![self.uniform_descriptor_set.clone()],
                        pipeline.layout(),
                    );
                    for (((clip_rect, triangles), vertex_buffer), index_buffer) in paint_jobs
                        .iter()
                        .zip(self.vertex_buffers.iter())
                        .zip(self.index_buffers.iter())
                    {
                        // Transform clip rect to physical pixels.
                        let clip_min_x = scale_factor * clip_rect.min.x;
                        let clip_min_y = scale_factor * clip_rect.min.y;
                        let clip_max_x = scale_factor * clip_rect.max.x;
                        let clip_max_y = scale_factor * clip_rect.max.y;

                        // Make sure clip rect can fit within an `u32`.
                        let clip_min_x = egui::clamp(clip_min_x, 0.0..=physical_width as f32);
                        let clip_min_y = egui::clamp(clip_min_y, 0.0..=physical_height as f32);
                        let clip_max_x =
                            egui::clamp(clip_max_x, clip_min_x..=physical_width as f32);
                        let clip_max_y =
                            egui::clamp(clip_max_y, clip_min_y..=physical_height as f32);

                        let clip_min_x = clip_min_x.round() as u32;
                        let clip_min_y = clip_min_y.round() as u32;
                        let clip_max_x = clip_max_x.round() as u32;
                        let clip_max_y = clip_max_y.round() as u32;

                        let width = (clip_max_x - clip_min_x).max(1);
                        let height = (clip_max_y - clip_min_y).max(1);

                        {
                            // clip scissor rectangle to target size
                            let x = clip_min_x.min(physical_width);
                            let y = clip_min_y.min(physical_height);
                            let width = width.min(physical_width - x);
                            let height = height.min(physical_height - y);

                            // skip rendering with zero-sized clip areas
                            if width == 0 || height == 0 {
                                continue;
                            }

                            recorder.set_scissor(&[vk::Rect2D {
                                offset: vk::Offset2D {
                                    x: x as i32,
                                    y: y as i32,
                                },
                                extent: vk::Extent2D { width, height },
                            }]);
                        }
                        recorder.bind_descriptor_sets(
                            vec![self
                                .get_texture_descriptor_set(triangles.texture_id)
                                .clone()],
                            pipeline.layout(),
                        );

                        recorder.bind_index_buffer(index_buffer.clone(), 0, vk::IndexType::UINT32);
                        recorder.bind_vertex_buffer(vec![vertex_buffer.clone()], &[0]);
                        recorder.draw_indexed(triangles.indices.len() as u32, 1);
                    }
                },
            );
        });
    }

    fn get_texture_descriptor_set(&self, texture_id: egui::TextureId) -> &Arc<DescriptorSet> {
        match texture_id {
            egui::TextureId::Egui => self
                .texture_descriptor_set
                .as_ref()
                .expect("egui texture was not set before the first draw"),
            egui::TextureId::User(id) => {
                let id = id as usize;
                assert!(id < self.user_textures.len());
                self.user_textures
                    .get(id)
                    .unwrap_or_else(|| panic!("user texture {} not found", id))
                    .as_ref()
                    .unwrap_or_else(|| panic!("user texture {} freed", id))
            }
        }
    }
}
