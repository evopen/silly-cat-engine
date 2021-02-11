use std::sync::Arc;
use std::time::Instant;

use egui_backend::*;
use safe_vk::{vk, Allocator, Device, Entry, Instance, PhysicalDevice, Surface};

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

        let mut ui_pass = UiPass::new(allocator.clone());

        let mut platform =
            egui_winit_platform::Platform::new(egui_winit_platform::PlatformDescriptor {
                physical_width: window.inner_size().width,
                physical_height: window.inner_size().height,
                scale_factor: window.scale_factor(),
                font_definitions: Default::default(),
                style: Default::default(),
            });

        event_loop.run(move |event, _, control_flow| match event {
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
                let (_output, paint_commands) = platform.end_frame();
                let paint_jobs = platform.context().tessellate(paint_commands);
                ui_pass.update_texture(&platform.context().texture());
            }
            winit::event::Event::RedrawEventsCleared => {}
            winit::event::Event::LoopDestroyed => {}
        });
    });
}
