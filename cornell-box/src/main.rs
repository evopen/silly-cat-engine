mod engine;
use engine::Engine;

fn main() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::Window::new(&event_loop).unwrap();
    let mut engine = Engine::new(&window);

    rt.block_on(async {
        engine.render_once();
    });

    // rt.block_on(async {
    //     event_loop.run(move |event, _, control_flow| {
    //         engine.handle_event(&event);
    //         match event {
    //             winit::event::Event::NewEvents(_) => {}
    //             winit::event::Event::WindowEvent {
    //                 window_id: _,
    //                 event,
    //             } => {
    //                 match event {
    //                     winit::event::WindowEvent::Resized(_) => {}
    //                     winit::event::WindowEvent::Moved(_) => {}
    //                     winit::event::WindowEvent::CloseRequested => {
    //                         *control_flow = winit::event_loop::ControlFlow::Exit;
    //                     }
    //                     _ => {}
    //                 }
    //             }
    //             winit::event::Event::DeviceEvent {
    //                 device_id: _,
    //                 event: _,
    //             } => {}
    //             winit::event::Event::UserEvent(_) => {}
    //             winit::event::Event::Suspended => {}
    //             winit::event::Event::Resumed => {}
    //             winit::event::Event::MainEventsCleared => {
    //                 window.request_redraw();
    //             }
    //             winit::event::Event::RedrawRequested(_) => {
    //                 engine.update();
    //                 engine.render();
    //             }
    //             winit::event::Event::RedrawEventsCleared => {}
    //             winit::event::Event::LoopDestroyed => {}
    //         }
    //     });
    // });
}
