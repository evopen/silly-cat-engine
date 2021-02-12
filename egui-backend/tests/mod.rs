use std::sync::Arc;
use std::time::Instant;

use egui_backend::*;
use epi::egui;
use safe_vk::{
    vk, Allocator, BinarySemaphore, CommandBuffer, CommandPool, Device, Entry, Fence, Instance,
    PhysicalDevice, Surface, Swapchain,
};

#[test]
fn test_all() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::WindowBuilder::new()
        .with_inner_size(winit::dpi::LogicalSize::new(800, 600))
        .with_title("Box of Chocolates")
        .build(&event_loop)
        .unwrap();

    let start_time = Instant::now();

    rt.block_on(async {
        let entry = Arc::new(Entry::new().unwrap());
        let surface_extensions = ash_window::enumerate_required_extensions(&window)
            .unwrap()
            .iter()
            .map(|s| s.to_str().unwrap())
            .collect::<Vec<_>>();
        let mut extensions = surface_extensions;
        extensions.push(safe_vk::name::instance::extension::ext::DEBUG_UTILS);
        let instance = Arc::new(Instance::new(
            entry.clone(),
            &[
                safe_vk::name::instance::layer::khronos::VALIDATION,
                safe_vk::name::instance::layer::lunarg::MONITOR,
            ],
            extensions.as_slice(),
        ));

        let surface = Arc::new(Surface::new(instance.clone(), &window));
        let pdevice = Arc::new(PhysicalDevice::new(instance.clone(), Some(&surface)));
        let device = Arc::new(Device::new(
            pdevice.clone(),
            &vk::PhysicalDeviceFeatures::default(),
            &[safe_vk::name::device::extension::khr::SWAPCHAIN],
        ));
        println!("swapchain images created");

        let allocator = Arc::new(Allocator::new(device.clone()));

        let mut ui_pass = UiPass::new(allocator.clone());

        let mut platform =
            egui_winit_platform::Platform::new(egui_winit_platform::PlatformDescriptor {
                physical_width: window.inner_size().width,
                physical_height: window.inner_size().height,
                scale_factor: window.scale_factor(),
                font_definitions: Default::default(),
                style: Default::default(),
            });

        let image_available_semaphore = Arc::new(BinarySemaphore::new(device.clone()));
        let render_finish_semaphore = Arc::new(BinarySemaphore::new(device.clone()));
        let mut swapchain = Arc::new(Swapchain::new(device.clone(), surface.clone()));
        let command_pool = Arc::new(CommandPool::new(device.clone()));
        let swapchain_images = safe_vk::Image::from_swapchain(swapchain.clone())
            .into_iter()
            .map(|image| Arc::new(image))
            .collect::<Vec<_>>();
        let mut queue = safe_vk::Queue::new(device.clone());

        let mut fence = Arc::new(Fence::new(device.clone(), true));

        event_loop.run(move |event, _, control_flow| {
            platform.handle_event(&event);
            match event {
                winit::event::Event::NewEvents(_) => {}
                winit::event::Event::WindowEvent { window_id, event } => match event {
                    winit::event::WindowEvent::Resized(_) => {}
                    winit::event::WindowEvent::Moved(_) => {}
                    winit::event::WindowEvent::CloseRequested => {
                        *control_flow = winit::event_loop::ControlFlow::Exit
                    }
                    winit::event::WindowEvent::Destroyed => {}
                    winit::event::WindowEvent::DroppedFile(_) => {}
                    winit::event::WindowEvent::HoveredFile(_) => {}
                    winit::event::WindowEvent::HoveredFileCancelled => {}
                    winit::event::WindowEvent::ReceivedCharacter(_) => {}
                    winit::event::WindowEvent::Focused(_) => {}
                    winit::event::WindowEvent::KeyboardInput {
                        device_id,
                        input,
                        is_synthetic,
                    } => {}
                    winit::event::WindowEvent::ModifiersChanged(_) => {}
                    winit::event::WindowEvent::CursorMoved {
                        device_id,
                        position,
                        modifiers,
                    } => {}
                    winit::event::WindowEvent::CursorEntered { device_id } => {}
                    winit::event::WindowEvent::CursorLeft { device_id } => {}
                    winit::event::WindowEvent::MouseWheel {
                        device_id,
                        delta,
                        phase,
                        modifiers,
                    } => {}
                    winit::event::WindowEvent::MouseInput {
                        device_id,
                        state,
                        button,
                        modifiers,
                    } => {}
                    winit::event::WindowEvent::TouchpadPressure {
                        device_id,
                        pressure,
                        stage,
                    } => {}
                    winit::event::WindowEvent::AxisMotion {
                        device_id,
                        axis,
                        value,
                    } => {}
                    winit::event::WindowEvent::Touch(_) => {}
                    winit::event::WindowEvent::ScaleFactorChanged {
                        scale_factor,
                        new_inner_size,
                    } => {}
                    winit::event::WindowEvent::ThemeChanged(_) => {}
                },
                winit::event::Event::DeviceEvent { device_id, event } => {}
                winit::event::Event::UserEvent(_) => {}
                winit::event::Event::Suspended => {}
                winit::event::Event::Resumed => {}
                winit::event::Event::MainEventsCleared => {
                    window.request_redraw();
                }
                winit::event::Event::RedrawRequested(_) => {
                    platform.update_time(start_time.elapsed().as_secs_f64());
                    platform.begin_frame();
                    egui::TopPanel::top(egui::Id::new("menu bar"))
                        .show(&platform.context().clone(), |ui| ui.button("fuck"));

                    let (_output, paint_commands) = platform.end_frame();
                    let paint_jobs = platform.context().tessellate(paint_commands);
                    ui_pass.update_texture(&platform.context().texture());
                    let screen_descriptor = ScreenDescriptor {
                        physical_width: window.inner_size().width,
                        physical_height: window.inner_size().height,
                        scale_factor: window.scale_factor() as f32,
                    };
                    ui_pass.update_buffers(&paint_jobs, &screen_descriptor);

                    let (index, _) =
                        swapchain.acquire_next_image(image_available_semaphore.clone());
                    let mut command_buffer = CommandBuffer::new(command_pool.clone());
                    command_buffer.encode(|recorder| {
                        recorder.set_image_layout(
                            swapchain_images[index as usize].clone(),
                            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                        );
                        ui_pass.execute(
                            recorder,
                            swapchain_images[index as usize].clone(),
                            &paint_jobs,
                            &screen_descriptor,
                        );
                    });
                    fence.wait();
                    fence = queue.submit_binary(
                        command_buffer,
                        &[&image_available_semaphore],
                        &[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT],
                        &[&render_finish_semaphore],
                    );
                    queue.present(&swapchain, index, &[&render_finish_semaphore]);
                }
                winit::event::Event::RedrawEventsCleared => {}
                winit::event::Event::LoopDestroyed => {}
            }
        });
    });
}
