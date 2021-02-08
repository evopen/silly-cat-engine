use std::sync::Arc;

use egui_backend::*;
use safe_vk::{vk, Allocator, Device, Entry, Instance, PhysicalDevice, Surface};

#[test]
fn test_all() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let window = create_window();
    println!("swapchain images created");

    rt.block_on(async {
        let entry = Arc::new(Entry::new().unwrap());
        let surface_extensions = ash_window::enumerate_required_extensions(&window)
            .unwrap()
            .iter()
            .map(|s| s.to_str().unwrap())
            .collect::<Vec<_>>();
        let instance = Arc::new(Instance::new(
            entry.clone(),
            &["VK_LAYER_KHRONOS_validation", "VK_LAYER_LUNARG_monitor"],
            surface_extensions.as_slice(),
        ));
        let surface = Arc::new(Surface::new(instance.clone(), &window));
        let pdevice = Arc::new(PhysicalDevice::new(instance.clone(), &surface));
        let device = Arc::new(Device::new(
            pdevice.clone(),
            &vk::PhysicalDeviceFeatures::default(),
            &[safe_vk::name::device::extension::khr::SWAPCHAIN],
        ));
        println!("swapchain images created");

        let allocator = Arc::new(Allocator::new(device.clone()));
    });
}

fn create_window() -> winit::window::Window {
    let event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::WindowBuilder::new()
        .with_inner_size(winit::dpi::LogicalSize::new(800, 600))
        .with_title("Box of Chocolates")
        .build(&event_loop)
        .unwrap();
    window
}
