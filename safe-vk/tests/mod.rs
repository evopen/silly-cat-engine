use std::sync::Arc;

use ash::vk;





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
    let _surface_extensions = ash_window::enumerate_required_extensions(&window)
        .unwrap()
        .iter()
        .map(|s| s.to_str().unwrap())
        .collect::<Vec<_>>();
    let _instance = Instance::new(
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
    );
}

#[test]
fn test_all() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let window = create_window();
    println!("swapchain images created");

    rt.block_on(async {
        let entry = Arc::new(Entry::new().unwrap());
        let _surface_extensions = ash_window::enumerate_required_extensions(&window)
            .unwrap()
            .iter()
            .map(|s| s.to_str().unwrap())
            .collect::<Vec<_>>();
        let instance = Arc::new(Instance::new(
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
        let surface = Arc::new(Surface::new(instance.clone(), &window));
        let pdevice = Arc::new(PhysicalDevice::new(instance, Some(surface)));
        let device = Arc::new(Device::new(
            pdevice,
            &vk::PhysicalDeviceFeatures::default(),
            &[ash::extensions::khr::Swapchain::name().to_str().unwrap()],
        ));
        println!("swapchain images created");

        let allocator = Arc::new(Allocator::new(device.clone()));
        let _descriptor_pool = DescriptorPool::new(
            device.clone(),
            &[vk::DescriptorPoolSize::builder()
                .descriptor_count(1)
                .ty(vk::DescriptorType::UNIFORM_BUFFER)
                .build()],
            1,
        );

        let buffer = Arc::new(Buffer::new(
            None,
            allocator.clone(),
            100,
            vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC,
            vk_mem::MemoryUsage::CpuToGpu,
        ));
        let buffer_dst = Arc::new(Buffer::new(
            None,
            allocator.clone(),
            100,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            vk_mem::MemoryUsage::CpuToGpu,
        ));
        assert_eq!(buffer.size(), 100);
        buffer.device_address();
        dbg!(vk::MemoryPropertyFlags::DEVICE_LOCAL.as_raw() & buffer.memory_type());
        let mut queue = Queue::new(device.clone());

        let command_pool = Arc::new(CommandPool::new(device.clone()));

        let swapchain = Arc::new(Swapchain::new(device.clone()));

        let _image = Image::new(
            allocator.clone(),
            vk::Format::B8G8R8A8_UNORM,
            123,
            234,
            vk::ImageUsageFlags::STORAGE,
            vk_mem::MemoryUsage::GpuOnly,
        );

        let _images = Image::from_swapchain(swapchain);

        println!("swapchain images created");
        let mut command_buffer = CommandBuffer::new(command_pool.clone());
        command_buffer.encode(|recorder| {
            recorder.copy_buffer(
                buffer.clone(),
                buffer_dst.clone(),
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
        semaphore.signal(2);

        let mut command_buffer = CommandBuffer::new(command_pool.clone());
        command_buffer.encode(|recorder| {
            recorder.copy_buffer(
                buffer.clone(),
                buffer_dst.clone(),
                &[vk::BufferCopy::builder().size(buffer.size() as u64).build()],
            );
        });

        let semaphore = TimelineSemaphore::new(device);
        queue.submit_timeline(
            command_buffer,
            &[&semaphore],
            &[0],
            &[vk::PipelineStageFlags::ALL_COMMANDS],
            &[1],
        );
        semaphore.wait_for(1);

        let matrix: [f32; 12] = [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let buffer = Buffer::new_init_device(
            None,
            allocator.clone(),
            vk::BufferUsageFlags::empty(),
            vk_mem::MemoryUsage::CpuToGpu,
            &mut queue,
            command_pool.clone(),
            bytemuck::cast_slice(&matrix),
        );
        assert_eq!(buffer.size(), 12 * 4);

        let buffer = Buffer::new_init_device(
            None,
            allocator.clone(),
            vk::BufferUsageFlags::STORAGE_BUFFER,
            vk_mem::MemoryUsage::GpuOnly,
            &mut queue,
            command_pool,
            bytemuck::cast_slice(&matrix),
        );
        assert_eq!(buffer.size(), 12 * 4);

        let image = Arc::new(Image::new(
            allocator,
            vk::Format::B8G8R8A8_UNORM,
            800,
            600,
            vk::ImageUsageFlags::SAMPLED,
            MemoryUsage::GpuOnly,
        ));
        let _image_view = ImageView::new(image);
    });
}
