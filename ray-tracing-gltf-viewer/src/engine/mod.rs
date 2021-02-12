use std::sync::Arc;

use safe_vk::Swapchain;

use safe_vk::vk;

pub struct Engine {
    ui_platform: egui_winit_platform::Platform,
    size: winit::dpi::PhysicalSize<u32>,
    scale_factor: f64,
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
        let instance = Arc::new(safe_vk::Instance::new(entry, &[], &[]));
        let surface = Arc::new(safe_vk::Surface::new(instance.clone(), window));
        let pdevice = Arc::new(safe_vk::PhysicalDevice::new(instance, Some(surface)));
        let device = Arc::new(safe_vk::Device::new(
            pdevice,
            &vk::PhysicalDeviceFeatures::default(),
            &[],
        ));
        let swapchain = Arc::new(safe_vk::Swapchain::new(device.clone()));
        Self {
            ui_platform,
            size,
            scale_factor,
        }
    }

    pub fn handle_event(&mut self, event: &winit::event::Event<()>) {
        self.ui_platform.handle_event(event);
    }

    pub fn update(&mut self) {}

    pub fn render(&self) {}
}
