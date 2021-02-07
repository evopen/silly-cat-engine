use std::sync::Arc;

use ash::vk;
use winit::event_loop::EventLoopWindowTarget;
use winit::platform::windows::EventLoopExtWindows;

use safe_vk::*;

fn create_window() -> winit::window::Window {
    let event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::WindowBuilder::new()
        .with_inner_size(winit::dpi::LogicalSize::new(800, 600))
        .with_title("Box of Chocolates")
        .build(&event_loop)
        .unwrap();
    window
}

#[test]
fn test_create_entry() {
    let entry = Entry::new().unwrap();
    println!("Vulkan version {}", entry.vulkan_version());
}

#[test]
fn test_create_instance() {
    let entry = Arc::new(Entry::new().unwrap());
    let window = create_window();
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

#[test]
fn test_all() {
    let entry = Arc::new(Entry::new().unwrap());
    let window = create_window();
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
        &[ash::extensions::khr::Swapchain::name().to_str().unwrap()],
    ));

    let allocator = Arc::new(Allocator::new(device.clone()));
    let descriptor_pool = DescriptorPool::new(device.clone(), &[], 0);

    let mut buffer = Buffer::new(
        allocator.clone(),
        100,
        vk::BufferUsageFlags::VERTEX_BUFFER,
        vk_mem::MemoryUsage::CpuToGpu,
    );
    assert_eq!(buffer.size(), 100);
    buffer.map();
    buffer.device_address();
    dbg!(vk::MemoryPropertyFlags::DEVICE_LOCAL.as_raw() & buffer.memory_type());
    let queue = Queue::new(device.clone());

    let command_pool = Arc::new(CommandPool::new(device.clone()));

    let command_buffer = CommandBuffer::new(command_pool.clone());
    let swapchain = Arc::new(Swapchain::new(device.clone(), surface.clone()));

    let image = Image::new(
        allocator.clone(),
        123,
        234,
        vk::ImageUsageFlags::STORAGE,
        vk_mem::MemoryUsage::GpuOnly,
    );

    let images = Image::from_swapchain(swapchain.clone());
}
