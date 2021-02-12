mod engine;
use engine::Engine;

fn main() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::Window::new(&event_loop).unwrap();
    let mut engine = Engine::new(&window);

    rt.block_on(async {
        event_loop.run(move |event, _, control_flow| {
            engine.handle_event(&event);
            match event {
                winit::event::Event::NewEvents(_) => {}
                winit::event::Event::WindowEvent { window_id, event } => {
                    match event {
                        winit::event::WindowEvent::Resized(_) => {}
                        winit::event::WindowEvent::Moved(_) => {}
                        winit::event::WindowEvent::CloseRequested => {
                            *control_flow = winit::event_loop::ControlFlow::Exit;
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
                    }
                }
                winit::event::Event::DeviceEvent { device_id, event } => {}
                winit::event::Event::UserEvent(_) => {}
                winit::event::Event::Suspended => {}
                winit::event::Event::Resumed => {}
                winit::event::Event::MainEventsCleared => {
                    window.request_redraw();
                }
                winit::event::Event::RedrawRequested(_) => {
                    engine.update();
                    engine.render();
                }
                winit::event::Event::RedrawEventsCleared => {}
                winit::event::Event::LoopDestroyed => {}
            }
        });
    });
}
