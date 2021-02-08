use std::sync::Arc;

use ash::vk;
use bytemuck::{cast_slice, Pod};
use vk::WHOLE_SIZE;
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
        &[
            name::instance::layer::khronos::VALIDATION,
            "VK_LAYER_LUNARG_monitor",
        ],
        surface_extensions.as_slice(),
    );
}

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
            &[ash::extensions::khr::Swapchain::name().to_str().unwrap()],
        ));
        println!("swapchain images created");

        let allocator = Arc::new(Allocator::new(device.clone()));
        let descriptor_pool = DescriptorPool::new(
            device.clone(),
            &[vk::DescriptorPoolSize::builder()
                .descriptor_count(1)
                .ty(vk::DescriptorType::UNIFORM_BUFFER)
                .build()],
            1,
        );

        let mut buffer = Buffer::new(
            allocator.clone(),
            100,
            vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC,
            vk_mem::MemoryUsage::CpuToGpu,
        );
        let mut buffer_dst = Arc::new(Buffer::new(
            allocator.clone(),
            100,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            vk_mem::MemoryUsage::CpuToGpu,
        ));
        assert_eq!(buffer.size(), 100);
        buffer.map();
        buffer.device_address();
        dbg!(vk::MemoryPropertyFlags::DEVICE_LOCAL.as_raw() & buffer.memory_type());
        let mut queue = Queue::new(device.clone());

        let command_pool = Arc::new(CommandPool::new(device.clone()));

        let swapchain = Arc::new(Swapchain::new(device.clone(), surface.clone()));

        let image = Image::new(
            allocator.clone(),
            123,
            234,
            vk::ImageUsageFlags::STORAGE,
            vk_mem::MemoryUsage::GpuOnly,
        );

        let images = Image::from_swapchain(swapchain.clone());

        println!("swapchain images created");
        let command_buffer = CommandBuffer::new(command_pool.clone());
        command_buffer.encode(|buf| {
            buf.cmd_copy_buffer(
                &buffer,
                &buffer_dst,
                &[vk::BufferCopy::builder().size(buffer.size() as u64).build()],
            );
        });

        let semaphore = TimelineSemaphore::new(device.clone());
        queue.submit_timeline(
            command_buffer,
            &[&semaphore],
            &[0],
            &[vk::PipelineStageFlags::ALL_COMMANDS],
            &[1],
        );
        semaphore.wait_for(1);
        semaphore.signal(0);

        let command_buffer = CommandBuffer::new(command_pool.clone());
        command_buffer.encode(|buf| {
            buf.cmd_copy_buffer(
                &buffer,
                &buffer_dst,
                &[vk::BufferCopy::builder().size(buffer.size() as u64).build()],
            );
        });

        let semaphore = TimelineSemaphore::new(device.clone());
        queue.submit_timeline(
            command_buffer,
            &[&semaphore],
            &[0],
            &[vk::PipelineStageFlags::ALL_COMMANDS],
            &[1],
        );
        semaphore.wait_for(1);

        let matrix: [f32; 12] = [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let buffer = Buffer::new_init(
            allocator.clone(),
            vk::BufferUsageFlags::empty(),
            vk_mem::MemoryUsage::CpuToGpu,
            &mut queue,
            command_pool.clone(),
            bytemuck::cast_slice(&matrix),
        );
        assert_eq!(buffer.size(), 12 * 4);

        let buffer = Buffer::new_init(
            allocator.clone(),
            vk::BufferUsageFlags::STORAGE_BUFFER,
            vk_mem::MemoryUsage::GpuOnly,
            &mut queue,
            command_pool.clone(),
            bytemuck::cast_slice(&matrix),
        );
        assert_eq!(buffer.size(), 12 * 4);
    });
}
