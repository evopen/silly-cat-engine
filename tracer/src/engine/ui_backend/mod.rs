mod shaders;

use ash::version::{DeviceV1_0, DeviceV1_2};
use ash::vk;
use epi::egui;
use std::rc::Rc;
use std::sync::Arc;
use vk_mem::MemoryUsage;

use bytemuck::{Pod, Zeroable};

use shaders::Shaders;

use super::command_buffer::CommandBuffer;
use super::image::Image;
use super::resource::{
    self, DescriptorPool, DescriptorSet, DescriptorSetLayout, GraphicsPipeline, PipelineLayout,
    Sampler, ShaderStage,
};
use super::Vulkan;
use super::{buffer::Buffer, resource::ShaderModule};

use anyhow::Result;

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
pub struct RenderPass {
    graphics_pipeline: GraphicsPipeline,
    index_buffers: Vec<Buffer>,
    vertex_buffers: Vec<Buffer>,
    uniform_buffer: Buffer,
    descriptor_pool: Rc<DescriptorPool>,
    uniform_descriptor_set: DescriptorSet,
    texture_descriptor_set_layout: DescriptorSetLayout,
    texture_descriptor_set: Option<DescriptorSet>,
    texture_version: Option<u64>,
    next_user_texture_id: u64,
    pending_user_textures: Vec<(u64, egui::Texture)>,
    user_textures: Vec<Option<DescriptorSet>>,
    vulkan: Arc<Vulkan>,
    render_pass: resource::RenderPass,
}

impl RenderPass {
    /// Creates a new render pass to render a egui UI. `output_format` needs to be either `wgpu::TextureFormat::Rgba8UnormSrgb` or `wgpu::TextureFormat::Bgra8UnormSrgb`. Panics if it's not a Srgb format.
    pub fn new(vulkan: Arc<Vulkan>) -> Result<Self> {
        let vs_module = ShaderModule::new(vulkan, Shaders::get("egui.vert.spv").unwrap())?;
        let fs_module = ShaderModule::new(vulkan, Shaders::get("egui.frag.spv").unwrap())?;

        let uniform_buffer = Buffer::new(
            std::mem::size_of::<UniformBuffer>(),
            vk::BufferUsageFlags::UNIFORM_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
            vk_mem::MemoryUsage::CpuToGpu,
            vulkan.clone(),
        )?;

        let sampler = Sampler::new(vulkan.clone())?;

        let uniform_descriptor_set_layout = DescriptorSetLayout::new(
            vulkan.clone(),
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
        )?;

        let texture_descriptor_set_layout = DescriptorSetLayout::new(
            vulkan.clone(),
            &[vk::DescriptorSetLayoutBinding::builder()
                .binding(0)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                .descriptor_count(1)
                .build()],
        )?;

        let pipeline_layout = PipelineLayout::new(
            vulkan.clone(),
            &[
                &uniform_descriptor_set_layout,
                &texture_descriptor_set_layout,
            ],
        )?;

        let graphics_pipeline = GraphicsPipeline::new(
            vulkan.clone(),
            pipeline_layout,
            &[
                &ShaderStage::new(vs_module, vk::ShaderStageFlags::VERTEX, "main"),
                &ShaderStage::new(fs_module, vk::ShaderStageFlags::FRAGMENT, "main"),
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
        )?;

        let descriptor_pool = Rc::new(DescriptorPool::new(
            vulkan.clone(),
            &[vk::DescriptorPoolSize::builder()
                .ty(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(1)
                .build()],
            1,
        )?);

        let uniform_descriptor_set = DescriptorSet::new(
            vulkan.clone(),
            descriptor_pool.clone(),
            &uniform_descriptor_set_layout,
        )?;

        let render_pass = resource::RenderPass::new(
            vulkan.clone(),
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

        Ok(Self {
            graphics_pipeline,
            vertex_buffers: Vec::with_capacity(64),
            index_buffers: Vec::with_capacity(64),
            uniform_buffer,
            descriptor_pool,
            uniform_descriptor_set,
            texture_descriptor_set_layout,
            texture_version: None,
            texture_descriptor_set: None,
            next_user_texture_id: 0,
            pending_user_textures: Vec::new(),
            user_textures: Vec::new(),
            vulkan,
            render_pass,
        })
    }

    /// Executes the egui render pass. When `clear_on_draw` is set, the output target will get cleared before writing to it.
    pub fn execute(
        &mut self,
        command_buffer: &mut CommandBuffer,
        color_attachment: &Image,
        paint_jobs: &[egui::paint::PaintJob],
        screen_descriptor: &ScreenDescriptor,
    ) {
        let image_view = color_attachment.view();
        let framebuffer = resource::Framebuffer::new(
            self.vulkan.clone(),
            &vk::FramebufferCreateInfo::builder()
                .attachments(&[image_view])
                .render_pass(self.render_pass.handle())
                .layers(1)
                .width(screen_descriptor.physical_width)
                .height(screen_descriptor.physical_height)
                .build(),
        );

        let scale_factor = screen_descriptor.scale_factor;
        let physical_width = screen_descriptor.physical_width;
        let physical_height = screen_descriptor.physical_height;

        let vulkan = self.vulkan.clone();
        let commands = |handle: vk::CommandBuffer| unsafe {
            vulkan.device.cmd_begin_render_pass(
                handle,
                &vk::RenderPassBeginInfo::builder()
                    .render_pass(self.render_pass.handle())
                    .framebuffer(framebuffer.handle())
                    .render_area(
                        vk::Rect2D::builder()
                            .extent(vk::Extent2D {
                                width: physical_height,
                                height: physical_height,
                            })
                            .build(),
                    )
                    .build(),
                vk::SubpassContents::INLINE,
            );
            vulkan.device.cmd_bind_pipeline(
                handle,
                vk::PipelineBindPoint::GRAPHICS,
                self.graphics_pipeline.handle(),
            );
            vulkan.device.cmd_bind_descriptor_sets(
                handle,
                vk::PipelineBindPoint::GRAPHICS,
                self.graphics_pipeline.layout().handle(),
                0,
                &[self.uniform_descriptor_set.handle()],
                &[0],
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
                let clip_max_x = egui::clamp(clip_max_x, clip_min_x..=physical_width as f32);
                let clip_max_y = egui::clamp(clip_max_y, clip_min_y..=physical_height as f32);

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

                    vulkan.device.cmd_set_scissor(
                        handle,
                        1,
                        &[vk::Rect2D {
                            offset: vk::Offset2D {
                                x: x as i32,
                                y: y as i32,
                            },
                            extent: vk::Extent2D { width, height },
                        }],
                    );
                }

                vulkan.device.cmd_bind_descriptor_sets(
                    handle,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.graphics_pipeline.layout().handle(),
                    1,
                    &[self.get_texture_bind_group(triangles.texture_id).handle()],
                    &[0],
                );
                vulkan.device.cmd_bind_index_buffer(
                    handle,
                    index_buffer.handle,
                    0,
                    vk::IndexType::UINT32,
                );
                vulkan
                    .device
                    .cmd_bind_vertex_buffers(handle, 0, &[vertex_buffer.handle], &[0]);

                vulkan
                    .device
                    .cmd_draw_indexed(handle, triangles.indices.len() as u32, 1, 0, 0, 0);
            }
            vulkan.device.cmd_end_render_pass(handle);
        };
    }

    fn get_texture_bind_group(&self, texture_id: egui::TextureId) -> &DescriptorSet {
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

    /// Updates the texture used by egui for the fonts etc. Should be called before `execute()`.
    pub fn update_texture(&mut self, queue: &wgpu::Queue, egui_texture: &egui::Texture) {
        // Don't update the texture if it hasn't changed.
        if self.texture_version == Some(egui_texture.version) {
            return;
        }
        // we need to convert the texture into rgba format
        let egui_texture = egui::Texture {
            version: egui_texture.version,
            width: egui_texture.width,
            height: egui_texture.height,
            pixels: egui_texture
                .pixels
                .iter()
                .flat_map(|p| std::iter::repeat(*p).take(4))
                .collect(),
        };
        let descriptor_set = self.egui_texture_to_wgpu(queue, &egui_texture, "egui");

        self.texture_version = Some(egui_texture.version);
        self.texture_descriptor_set = Some(descriptor_set);
    }

    /// Updates the user textures that the app allocated. Should be called before `execute()`.
    pub fn update_user_textures(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let pending_user_textures = std::mem::take(&mut self.pending_user_textures);
        for (id, texture) in pending_user_textures {
            let bind_group = self.egui_texture_to_wgpu(
                device,
                queue,
                &texture,
                format!("user_texture{}", id).as_str(),
            );
            self.user_textures.push(Some(bind_group));
        }
    }

    fn egui_texture_to_wgpu(
        &self,
        queue: &wgpu::Queue,
        egui_texture: &egui::Texture,
        label: &str,
    ) -> DescriptorSet {
        let texture = Image::new(
            egui_texture.width as u32,
            egui_texture.height as u32,
            vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
            vk_mem::MemoryUsage::CpuToGpu,
            vk::ImageLayout::UNDEFINED,
            self.vulkan.clone(),
        )
        .unwrap();

        queue.write_texture(
            wgpu::TextureCopyView {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            egui_texture.pixels.as_slice(),
            wgpu::TextureDataLayout {
                offset: 0,
                bytes_per_row: (egui_texture.pixels.len() / egui_texture.height) as u32,
                rows_per_image: egui_texture.height as u32,
            },
            size,
        );

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(format!("{}_texture_bind_group", label).as_str()),
            layout: &self.texture_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(
                    &texture.create_view(&wgpu::TextureViewDescriptor::default()),
                ),
            }],
        });

        bind_group
    }

    /// Uploads the uniform, vertex and index data used by the render pass. Should be called before `execute()`.
    pub fn update_buffers(
        &mut self,
        queue: &wgpu::Queue,
        paint_jobs: &[egui::paint::PaintJob],
        screen_descriptor: &ScreenDescriptor,
    ) {
        let index_size = self.index_buffers.len();
        let vertex_size = self.vertex_buffers.len();

        let (logical_width, logical_height) = screen_descriptor.logical_size();

        self.update_buffer(
            device,
            queue,
            BufferType::Uniform,
            0,
            bytemuck::cast_slice(&[UniformBuffer {
                screen_size: [logical_width as f32, logical_height as f32],
            }]),
        );

        for (i, (_, triangles)) in paint_jobs.iter().enumerate() {
            let data: &[u8] = bytemuck::cast_slice(&triangles.indices);
            if i < index_size {
                self.update_buffer(device, queue, BufferType::Index, i, data)
            } else {
                let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("egui_index_buffer"),
                    contents: data,
                    usage: wgpu::BufferUsage::INDEX | wgpu::BufferUsage::COPY_DST,
                });
                self.index_buffers.push(SizedBuffer {
                    buffer,
                    size: data.len(),
                });
            }

            let data: &[u8] = as_byte_slice(&triangles.vertices);
            if i < vertex_size {
                self.update_buffer(device, queue, BufferType::Vertex, i, data)
            } else {
                let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("egui_vertex_buffer"),
                    contents: data,
                    usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
                });

                self.vertex_buffers.push(SizedBuffer {
                    buffer,
                    size: data.len(),
                });
            }
        }
    }

    /// Updates the buffers used by egui. Will properly re-size the buffers if needed.
    fn update_buffer(&mut self, buffer_type: BufferType, index: usize, data: &[u8]) {
        let (buffer, storage, name) = match buffer_type {
            BufferType::Index => (
                &mut self.index_buffers[index],
                vk::BufferUsageFlags::INDEX_BUFFER,
                "index",
            ),
            BufferType::Vertex => (
                &mut self.vertex_buffers[index],
                vk::BufferUsageFlags::VERTEX_BUFFER,
                "vertex",
            ),
            BufferType::Uniform => (
                &mut self.uniform_buffer,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                "uniform",
            ),
        };

        if data.len() > buffer.size() {
            // TODO: unimpl
            // buffer.size = data.len();
            // buffer.buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            //     label: Some(format!("egui_{}_buffer", name).as_str()),
            //     contents: bytemuck::cast_slice(data),
            //     usage: storage | vk::BufferUsageFlags::TRANSFER_DST,
            // });
            unimplemented!();
        } else {
            buffer.copy_from(data.as_ptr());
        }
    }
}

impl epi::TextureAllocator for RenderPass {
    fn alloc_srgba_premultiplied(
        &mut self,
        size: (usize, usize),
        srgba_pixels: &[egui::Color32],
    ) -> egui::TextureId {
        let id = self.next_user_texture_id;
        self.next_user_texture_id += 1;

        let mut pixels = vec![0u8; srgba_pixels.len() * 4];
        for (target, given) in pixels.chunks_exact_mut(4).zip(srgba_pixels.iter()) {
            target.copy_from_slice(&given.to_array());
        }

        let (width, height) = size;
        self.pending_user_textures.push((
            id,
            egui::Texture {
                version: 0,
                width,
                height,
                pixels,
            },
        ));

        egui::TextureId::User(id)
    }

    fn free(&mut self, id: egui::TextureId) {
        if let egui::TextureId::User(id) = id {
            self.user_textures
                .get_mut(id as usize)
                .and_then(|option| option.take());
        }
    }
}

// Needed since we can't use bytemuck for external types.
fn as_byte_slice<T>(slice: &[T]) -> &[u8] {
    let len = slice.len() * std::mem::size_of::<T>();
    let ptr = slice.as_ptr() as *const u8;
    unsafe { std::slice::from_raw_parts(ptr, len) }
}
