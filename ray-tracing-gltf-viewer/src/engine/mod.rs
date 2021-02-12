use std::sync::Arc;

use safe_vk::Swapchain;

use safe_vk::vk;

pub struct Engine {
    ui_platform: egui_winit_platform::Platform,
    size: winit::dpi::PhysicalSize<u32>,
    scale_factor: f64,
    swapchain: Arc<safe_vk::Swapchain>,
    queue: safe_vk::Queue,
    ui_pass: egui_backend::UiPass,
}

impl Engine {
    pub fn new(window: &winit::window::Window) -> Self {
        let size = window.inner_size();
        let scale_factor = window.scale_factor();
        let ui_platform =
            egui_winit_platform::Platform::new(egui_winit_platform::PlatformDescriptor {
                physical_width: size.width,
                physical_height: size.height,
                scale_factor: scale_factor,
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
            ],
        ));
        let swapchain = Arc::new(safe_vk::Swapchain::new(device.clone()));
        let queue = safe_vk::Queue::new(device.clone());
        let allocator = Arc::new(safe_vk::Allocator::new(device.clone()));
        let ui_pass = egui_backend::UiPass::new(allocator.clone());
        Self {
            ui_platform,
            size,
            scale_factor,
            swapchain,
            queue,
            ui_pass,
        }
    }

    pub fn handle_event(&mut self, event: &winit::event::Event<()>) {
        self.ui_platform.handle_event(event);
    }

    pub fn update(&mut self) {}

    pub fn render(&self) {
        let (index, _) = self.swapchain.acquire_next_image();
        self.queue.present(
            &self.swapchain,
            index,
            &[&self.swapchain.image_available_semaphore()],
        )
    }
}
