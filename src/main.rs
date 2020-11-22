#![feature(in_band_lifetimes)]

use anyhow::{anyhow, bail, Result};
use std::sync::{Arc, RwLock};

mod engine;
mod triangle;
mod val;

fn init_logger() -> Result<Arc<RwLock<crossbeam::queue::ArrayQueue<String>>>> {
    let log_file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .append(false)
        .truncate(true)
        .open(format!("{}.log", env!("CARGO_PKG_NAME")))?;

    let ring_buf = Arc::new(RwLock::new(crossbeam::queue::ArrayQueue::new(50)));
    let ring_buf_write = ring_buf.clone();

    fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Error)
        .level_for(env!("CARGO_CRATE_NAME"), log::LevelFilter::Debug)
        .chain(std::io::stdout())
        .chain(log_file)
        .chain(fern::Output::call(move |rec| {
            ring_buf_write
                .write()
                .unwrap()
                .push(rec.args().to_string())
                .unwrap();
        }))
        .apply()?;
    Ok(ring_buf)
}

fn main() -> Result<()> {
    let bt = backtrace::Backtrace::new();
    init_logger()?;
    std::panic::set_hook(Box::new(move |p| {
        log::error!("{}", p.to_string());
        log::trace!("\n{:?}", bt);
    }));

    log::info!("Initializing...");
    let start_time = std::time::Instant::now();

    let event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::WindowBuilder::new()
        .with_title("Vulkan Renderer")
        .with_inner_size(winit::dpi::PhysicalSize::new(800, 600))
        .with_transparent(false)
        .build(&event_loop)?;

    let instance = val::Instance::new(val::InstanceDescription {
        extension_names: ash_window::enumerate_required_extensions(&window).unwrap(),
    });
    let surface = instance.create_surface(&window);
    let device = instance.create_device(&surface);
    let swapchain = device.create_swapchain(&surface);

    log::info!(
        "Initialized, took {} seconds",
        start_time.elapsed().as_secs_f32()
    );

    event_loop.run(move |event, _, control_flow| match event {
        winit::event::Event::NewEvents(_) => {}
        winit::event::Event::WindowEvent { window_id, event } => {
            engine.input(&event);
            match event {
                winit::event::WindowEvent::CloseRequested => {
                    *control_flow = winit::event_loop::ControlFlow::Exit;
                }
                winit::event::WindowEvent::KeyboardInput {
                    device_id,
                    input,
                    is_synthetic,
                } => match input {
                    winit::event::KeyboardInput {
                        virtual_keycode: Some(winit::event::VirtualKeyCode::Escape),
                        state: winit::event::ElementState::Pressed,
                        ..
                    } => {
                        *control_flow = winit::event_loop::ControlFlow::Exit;
                    }
                    _ => {}
                },
                winit::event::WindowEvent::ScaleFactorChanged {
                    scale_factor,
                    new_inner_size,
                } => {}
                _ => {}
            }
        }
        winit::event::Event::MainEventsCleared => {
            window.request_redraw();
        }
        winit::event::Event::RedrawRequested(_) => {
            engine.update();
            if let Ok(msg) = prime_rx.try_recv() {
                match msg {
                    Message::RequestUpdate { path } => {
                        action::self_update(&path);
                        *control_flow = winit::event_loop::ControlFlow::Exit;
                    }
                    Message::Exit => {
                        *control_flow = winit::event_loop::ControlFlow::Exit;
                    }
                    Message::AlwaysOnTop(b) => {
                        window.set_always_on_top(b);
                    }
                }
            }
            engine.render();
        }
        winit::event::Event::RedrawEventsCleared => {}
        winit::event::Event::LoopDestroyed => {}
        _ => {}
    });

    Ok(())
}
