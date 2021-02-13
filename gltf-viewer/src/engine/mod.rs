use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Instant;

use nfd2::open_file_dialog;
use safe_vk::vk;

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
    scene: Option<gltf_wrapper::Scene>,
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
                safe_vk::name::instance::layer::khronos::VALIDATION,
                safe_vk::name::instance::layer::lunarg::MONITOR,
            ],
            &[
                safe_vk::name::instance::extension::khr::WIN32_SURFACE,
                safe_vk::name::instance::extension::khr::SURFACE,
                safe_vk::name::instance::extension::ext::DEBUG_UTILS,
            ],
        ));
        let surface = Arc::new(safe_vk::Surface::new(instance.clone(), window));

        let pdevice = Arc::new(safe_vk::PhysicalDevice::new(instance, Some(surface)));
        let device = Arc::new(safe_vk::Device::new(
            pdevice,
            &vk::PhysicalDeviceFeatures::default(),
            &[
                safe_vk::name::device::extension::khr::SWAPCHAIN,
                safe_vk::name::device::extension::khr::ACCELERATION_STRUCTURE,
                safe_vk::name::device::extension::khr::DEFERRED_HOST_OPERATIONS,
                safe_vk::name::device::extension::khr::BUFFER_DEVICE_ADDRESS,
            ],
        ));
        let swapchain = Arc::new(safe_vk::Swapchain::new(device.clone()));
        let queue = safe_vk::Queue::new(device.clone());
        let allocator = Arc::new(safe_vk::Allocator::new(device.clone()));
        let ui_pass = egui_backend::UiPass::new(allocator.clone());
        let command_pool = Arc::new(safe_vk::CommandPool::new(device.clone()));
        let time = Instant::now();
        let swapchain_images = safe_vk::Image::from_swapchain(swapchain.clone())
            .into_iter()
            .map(Arc::new)
            .collect::<Vec<_>>();
        let render_finish_semaphore = safe_vk::BinarySemaphore::new(device.clone());
        let render_finish_fence = Arc::new(safe_vk::Fence::new(device, true));

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
            scene: None,
        }
    }

    pub fn handle_event(&mut self, event: &winit::event::Event<()>) {
        self.ui_platform.handle_event(event);
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
                            nfd2::Response::Okay(p) => {
                                self.scene =
                                    Some(gltf_wrapper::Scene::from_file(self.allocator.clone(), p));
                            }
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
    }

    pub fn render(&mut self) {
        let (index, _) = self.swapchain.acquire_next_image();
        let mut command_buffer = safe_vk::CommandBuffer::new(self.command_pool.clone());

        let target_image = self.swapchain_images[index as usize].clone();
        command_buffer.encode(|recorder| {
            recorder.set_image_layout(
                target_image.clone(),
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