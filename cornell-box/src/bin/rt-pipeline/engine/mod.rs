mod shaders;

use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Instant;

use bytemuck::cast_slice;
use camera::Camera;
use image::ImageBuffer;
use safe_vk::{vk, PipelineRecorder};
use vk::CommandBuffer;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

const WORKGROUP_WIDTH: u32 = 16;
const WORKGROUP_HEIGHT: u32 = 8;

pub struct Engine {
    ui_platform: egui_winit_platform::Platform,
    size: winit::dpi::PhysicalSize<u32>,
    scale_factor: f64,
    swapchain: Arc<safe_vk::Swapchain>,
    queue: safe_vk::Queue,
    ui_pass: egui_backend::UiPass,
    command_pool: Arc<safe_vk::CommandPool>,
    time: Instant,
    swapchain_images: Vec<Arc<safe_vk::Image>>,
    render_finish_semaphore: safe_vk::BinarySemaphore,
    render_finish_fence: Arc<safe_vk::Fence>,
    allocator: Arc<safe_vk::Allocator>,
    pipeline: Arc<safe_vk::RayTracingPipeline>,
    descriptor_set: Arc<safe_vk::DescriptorSet>,
    result_image: Arc<safe_vk::Image>,
    uniform_buffer: Arc<safe_vk::Buffer>,
    camera: Camera,
    scene: gltf_wrapper::Scene,
}

impl Engine {
    pub fn new(window: &winit::window::Window) -> Self {
        let size = window.inner_size();
        let scale_factor = window.scale_factor();
        let ui_platform =
            egui_winit_platform::Platform::new(egui_winit_platform::PlatformDescriptor {
                physical_width: size.width,
                physical_height: size.height,
                scale_factor,
                font_definitions: Default::default(),
                style: Default::default(),
            });
        let entry = Arc::new(safe_vk::Entry::new().unwrap());
        let instance = Arc::new(safe_vk::Instance::new(
            entry,
            &[
                safe_vk::name::instance::Layer::KhronosValidation,
                safe_vk::name::instance::Layer::LunargMonitor,
            ],
            &[
                safe_vk::name::instance::Extension::KhrWin32Surface,
                safe_vk::name::instance::Extension::KhrSurface,
                safe_vk::name::instance::Extension::ExtDebugUtils,
            ],
        ));
        let surface = Arc::new(safe_vk::Surface::new(instance.clone(), window));

        let pdevice = Arc::new(safe_vk::PhysicalDevice::new(instance, Some(surface)));
        let device = Arc::new(safe_vk::Device::new(
            pdevice,
            &vk::PhysicalDeviceFeatures {
                fragment_stores_and_atomics: vk::TRUE,
                vertex_pipeline_stores_and_atomics: vk::TRUE,
                ..Default::default()
            },
            &[
                safe_vk::name::device::Extension::KhrSwapchain,
                safe_vk::name::device::Extension::KhrAccelerationStructure,
                safe_vk::name::device::Extension::KhrDeferredHostOperations,
                safe_vk::name::device::Extension::KhrRayTracingPipeline,
                safe_vk::name::device::Extension::KhrShaderNonSemanticInfo,
            ],
        ));
        let swapchain = Arc::new(safe_vk::Swapchain::new(device.clone()));
        let mut queue = safe_vk::Queue::new(device.clone());
        let allocator = Arc::new(safe_vk::Allocator::new(device.clone()));
        let ui_pass = egui_backend::UiPass::new(allocator.clone());
        let command_pool = Arc::new(safe_vk::CommandPool::new(device.clone()));
        let time = Instant::now();
        let swapchain_images = safe_vk::Image::from_swapchain(swapchain.clone())
            .into_iter()
            .map(Arc::new)
            .collect::<Vec<_>>();
        let render_finish_semaphore = safe_vk::BinarySemaphore::new(device.clone());
        let render_finish_fence = Arc::new(safe_vk::Fence::new(device.clone(), true));

        let descriptor_set_layout = Arc::new(safe_vk::DescriptorSetLayout::new(
            device.clone(),
            Some("descriptor set layout"),
            &[
                safe_vk::DescriptorSetLayoutBinding {
                    binding: 0,
                    descriptor_type: safe_vk::DescriptorType::StorageImage,
                    stage_flags: vk::ShaderStageFlags::RAYGEN_KHR,
                },
                safe_vk::DescriptorSetLayoutBinding {
                    binding: 1,
                    descriptor_type: safe_vk::DescriptorType::AccelerationStructure,
                    stage_flags: vk::ShaderStageFlags::RAYGEN_KHR,
                },
                safe_vk::DescriptorSetLayoutBinding {
                    binding: 2,
                    descriptor_type: safe_vk::DescriptorType::StorageBuffer,
                    stage_flags: vk::ShaderStageFlags::CLOSEST_HIT_KHR,
                },
                safe_vk::DescriptorSetLayoutBinding {
                    binding: 3,
                    descriptor_type: safe_vk::DescriptorType::StorageBuffer,
                    stage_flags: vk::ShaderStageFlags::CLOSEST_HIT_KHR,
                },
            ],
        ));

        let pipeline_layout = Arc::new(safe_vk::PipelineLayout::new(
            device.clone(),
            Some("compute pipeline layout"),
            &[&descriptor_set_layout],
        ));

        let mut result_image = safe_vk::Image::new(
            Some("result image"),
            allocator.clone(),
            vk::Format::R32G32B32A32_SFLOAT,
            WIDTH,
            HEIGHT,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::STORAGE
                | vk::ImageUsageFlags::TRANSFER_DST
                | vk::ImageUsageFlags::TRANSFER_SRC,
            safe_vk::MemoryUsage::GpuOnly,
        );

        result_image.set_layout(vk::ImageLayout::GENERAL, &mut queue, command_pool.clone());

        let result_image = Arc::new(result_image);

        let result_image_view = Arc::new(safe_vk::ImageView::new(result_image.clone()));

        let mut descriptor_set = safe_vk::DescriptorSet::new(
            Some("Main descriptor set"),
            Arc::new(safe_vk::DescriptorPool::new(
                device.clone(),
                &[vk::DescriptorPoolSize::builder()
                    .ty(vk::DescriptorType::STORAGE_IMAGE)
                    .descriptor_count(1)
                    .build()],
                1,
            )),
            descriptor_set_layout.clone(),
        );

        let scene = gltf_wrapper::Scene::from_file(
            allocator.clone(),
            "./cornell-box/models/CornellBox.glb",
        );

        let uniform_buffer = Arc::new(safe_vk::Buffer::new(
            Some("camera buffer"),
            allocator.clone(),
            std::mem::size_of::<f32>() * 3,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            safe_vk::MemoryUsage::CpuToGpu,
        ));

        descriptor_set.update(&[
            safe_vk::DescriptorSetUpdateInfo {
                binding: 0,
                detail: safe_vk::DescriptorSetUpdateDetail::Image(result_image_view.clone()),
            },
            safe_vk::DescriptorSetUpdateInfo {
                binding: 1,
                detail: safe_vk::DescriptorSetUpdateDetail::AccelerationStructure(
                    scene.tlas().clone(),
                ),
            },
            safe_vk::DescriptorSetUpdateInfo {
                binding: 2,
                detail: safe_vk::DescriptorSetUpdateDetail::Buffer {
                    buffer: scene.sole_buffer().clone(),
                    offset: scene.sole_geometry_index_buffer_offset(),
                },
            },
            safe_vk::DescriptorSetUpdateInfo {
                binding: 3,
                detail: safe_vk::DescriptorSetUpdateDetail::Buffer {
                    buffer: scene.sole_buffer().clone(),
                    offset: scene.sole_geometry_vertex_buffer_offset(),
                },
            },
        ]);

        let descriptor_set = Arc::new(descriptor_set);

        let shader_stages = vec![
            Arc::new(safe_vk::ShaderStage::new(
                Arc::new(safe_vk::ShaderModule::new(
                    device.clone(),
                    shaders::Shaders::get("raytrace.rgen.spv").unwrap(),
                )),
                vk::ShaderStageFlags::RAYGEN_KHR,
                "main",
            )),
            Arc::new(safe_vk::ShaderStage::new(
                Arc::new(safe_vk::ShaderModule::new(
                    device.clone(),
                    shaders::Shaders::get("miss.rmiss.spv").unwrap(),
                )),
                vk::ShaderStageFlags::MISS_KHR,
                "main",
            )),
            Arc::new(safe_vk::ShaderStage::new(
                Arc::new(safe_vk::ShaderModule::new(
                    device.clone(),
                    shaders::Shaders::get("closest_hit.rchit.spv").unwrap(),
                )),
                vk::ShaderStageFlags::CLOSEST_HIT_KHR,
                "main",
            )),
        ];

        let pipeline = Arc::new(safe_vk::RayTracingPipeline::new(
            Some("rt pipeline"),
            allocator.clone(),
            pipeline_layout,
            shader_stages,
            1,
            &mut queue,
        ));

        let camera = camera::Camera::new(
            glam::Vec3A::new(-0.001, 0.0, 3.0),
            glam::Vec3A::new(0.0, 0.0, 0.0),
        );

        log::info!("pipeline created");

        Self {
            ui_platform,
            size,
            scale_factor,
            swapchain,
            queue,
            ui_pass,
            command_pool,
            time,
            swapchain_images,
            render_finish_semaphore,
            render_finish_fence,
            allocator,
            pipeline,
            descriptor_set,
            result_image,
            uniform_buffer,
            camera,
            scene,
        }
    }

    // pub fn render_once(&mut self) {
    //     let mut command_buffer = safe_vk::CommandBuffer::new(self.command_pool.clone());
    //     command_buffer.encode(|rec| {
    //         rec.bind_compute_pipeline(self.pipeline.clone(), |rec, pipeline| {
    //             rec.bind_descriptor_sets(vec![self.descriptor_set.clone()], pipeline.layout(), 0);

    //             rec.dispatch(
    //                 (WIDTH as f32 / WORKGROUP_WIDTH as f32).ceil() as u32,
    //                 (HEIGHT as f32 / WORKGROUP_HEIGHT as f32).ceil() as u32,
    //                 1,
    //             );
    //         });
    //     });
    //     self.queue
    //         .submit_binary(command_buffer, &[], &[], &[])
    //         .wait();
    //     let mapped = self.storage_buffer.map();
    //     let mapped = unsafe { std::mem::transmute(mapped) };
    //     let data: &[image::Rgb<f32>] =
    //         unsafe { std::slice::from_raw_parts(mapped, (WIDTH * HEIGHT) as usize) };
    //     let f = std::fs::File::create("./hello.hdr").unwrap();
    //     let encoder = image::hdr::HdrEncoder::new(f);

    //     encoder
    //         .encode(data, WIDTH as usize, HEIGHT as usize)
    //         .unwrap();
    //     self.storage_buffer.unmap();
    // }

    pub fn handle_event(&mut self, event: &winit::event::Event<()>) {
        self.ui_platform.handle_event(event);
        self.camera.input(event);
    }

    pub fn update(&mut self) {
        let current_dir = PathBuf::from_str(std::env::current_dir().unwrap().to_str().unwrap())
            .unwrap()
            .join("models\\2.0\\Box\\glTF");
        self.ui_platform
            .update_time(self.time.elapsed().as_secs_f64());
        self.ui_platform.begin_frame();

        egui::TopPanel::top(egui::Id::new("menu bar")).show(&self.ui_platform.context(), |ui| {
            egui::menu::bar(ui, |ui| {
                egui::menu::menu(ui, "File", |ui| {
                    if ui.button("Open").clicked {
                        match nfd2::open_file_dialog(Some("gltf,glb"), Some(current_dir.as_ref()))
                            .unwrap()
                        {
                            nfd2::Response::Okay(p) => {}
                            nfd2::Response::OkayMultiple(_) => {}
                            nfd2::Response::Cancel => {}
                        }
                    }
                });
            });
        });

        let (_, shapes) = self.ui_platform.end_frame();
        let paint_jobs = self.ui_platform.context().tessellate(shapes);
        self.ui_pass.update_buffers(
            &paint_jobs,
            &egui_backend::ScreenDescriptor {
                physical_width: self.size.width,
                physical_height: self.size.height,
                scale_factor: self.scale_factor as f32,
            },
        );
        self.ui_pass
            .update_texture(&self.ui_platform.context().texture());

        self.uniform_buffer.copy_from(bytemuck::cast_slice(
            self.camera.camera_uniform().origin.as_ref(),
        ));
    }

    pub fn render(&mut self) {
        let (index, _) = self.swapchain.acquire_next_image();
        let mut command_buffer = safe_vk::CommandBuffer::new(self.command_pool.clone());

        let target_image = self.swapchain_images[index as usize].clone();

        let start_address = self.pipeline.sbt_buffer().device_address();
        let stride = self.pipeline.sbt_stride() as u64;
        let sbt_ray_gen_region = vk::StridedDeviceAddressRegionKHR::builder()
            .device_address(start_address)
            .stride(stride)
            .size(stride)
            .build();
        let mut sbt_hit_region = sbt_ray_gen_region;
        sbt_hit_region.size = stride;
        sbt_hit_region.device_address = start_address + 2 * stride;
        let mut sbt_miss_region = sbt_ray_gen_region;
        sbt_miss_region.size = stride;
        sbt_miss_region.device_address = start_address + stride;

        let mut sbt_callable_region = sbt_ray_gen_region;
        sbt_callable_region.size = 0;

        command_buffer.encode(|recorder| {
            // recorder.bind_compute_pipeline(self.pipeline.clone(), |rec, pipeline| {
            //     rec.bind_descriptor_sets(vec![self.descriptor_set.clone()], pipeline.layout(), 0);

            //     rec.dispatch(
            //         (WIDTH as f32 / WORKGROUP_WIDTH as f32).ceil() as u32,
            //         (HEIGHT as f32 / WORKGROUP_HEIGHT as f32).ceil() as u32,
            //         1,
            //     );
            // });
            recorder.set_image_layout(
                self.result_image.clone(),
                Some(vk::ImageLayout::UNDEFINED),
                vk::ImageLayout::GENERAL,
            );
            recorder.bind_ray_tracing_pipeline(self.pipeline.clone(), |rec, pipeline| {
                rec.bind_descriptor_sets(vec![self.descriptor_set.clone()], pipeline.layout(), 0);
                rec.trace_ray(
                    &sbt_ray_gen_region,
                    &sbt_miss_region,
                    &sbt_hit_region,
                    &sbt_callable_region,
                    WIDTH,
                    HEIGHT,
                    1,
                );
            });
            recorder.set_image_layout(
                self.result_image.clone(),
                Some(vk::ImageLayout::GENERAL),
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            );
            recorder.set_image_layout(
                target_image.clone(),
                Some(vk::ImageLayout::UNDEFINED),
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            );
            // recorder.copy_buffer_to_image(
            //     self.storage_buffer.clone(),
            //     self.result_image.clone(),
            //     &[vk::BufferImageCopy::builder()
            //         .image_extent(vk::Extent3D {
            //             width: self.result_image.width(),
            //             height: self.result_image.height(),
            //             depth: 1,
            //         })
            //         .image_subresource(
            //             vk::ImageSubresourceLayers::builder()
            //                 .aspect_mask(vk::ImageAspectFlags::COLOR)
            //                 .layer_count(1)
            //                 .base_array_layer(0)
            //                 .mip_level(0)
            //                 .build(),
            //         )
            //         .build()],
            // );

            recorder.blit_image(
                self.result_image.clone(),
                target_image.clone(),
                &[vk::ImageBlit::builder()
                    .src_subresource(
                        vk::ImageSubresourceLayers::builder()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .layer_count(1)
                            .base_array_layer(0)
                            .mip_level(0)
                            .build(),
                    )
                    .src_offsets([
                        vk::Offset3D { x: 0, y: 0, z: 0 },
                        vk::Offset3D {
                            x: self.result_image.width() as i32,
                            y: self.result_image.height() as i32,
                            z: 1,
                        },
                    ])
                    .dst_offsets([
                        vk::Offset3D { x: 0, y: 0, z: 0 },
                        vk::Offset3D {
                            x: target_image.width() as i32,
                            y: target_image.height() as i32,
                            z: 1,
                        },
                    ])
                    .dst_subresource(
                        vk::ImageSubresourceLayers::builder()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .layer_count(1)
                            .base_array_layer(0)
                            .mip_level(0)
                            .build(),
                    )
                    .build()],
                vk::Filter::NEAREST,
            );
            recorder.set_image_layout(
                target_image.clone(),
                None,
                vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            );
            self.ui_pass.execute(
                recorder,
                target_image,
                &egui_backend::ScreenDescriptor {
                    physical_width: self.size.width,
                    physical_height: self.size.height,
                    scale_factor: self.scale_factor as f32,
                },
            );
        });
        self.render_finish_fence.wait();
        self.render_finish_fence = self.queue.submit_binary(
            command_buffer,
            &[&self.swapchain.image_available_semaphore()],
            &[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT],
            &[&self.render_finish_semaphore],
        );
        self.queue
            .present(&self.swapchain, index, &[&self.render_finish_semaphore])
    }
}
