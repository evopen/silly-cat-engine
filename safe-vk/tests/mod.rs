use std::sync::Arc;

use winit::event_loop::EventLoopWindowTarget;
use winit::platform::windows::EventLoopExtWindows;

use safe_vk::*;

#[test]
fn test_create_entry() {
    let entry = Entry::new().unwrap();
    println!("Vulkan version {}", entry.vulkan_version());
}

#[test]
fn test_create_instance() {
    let entry = Arc::new(Entry::new().unwrap());
    let event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::WindowBuilder::new()
        .with_inner_size(winit::dpi::LogicalSize::new(800, 600))
        .with_title("Box of Chocolates")
        .build(&event_loop)
        .unwrap();
    let surface_extensions = ash_window::enumerate_required_extensions(&window)
        .unwrap()
        .iter()
        .map(|s| s.to_str().unwrap())
        .collect::<Vec<_>>();
    let instance = Instance::new(
        entry.clone(),
        &["VK_LAYER_KHRONOS_validation", "VK_LAYER_LUNARG_monitor"],
        surface_extensions.as_slice(),
    );
}
