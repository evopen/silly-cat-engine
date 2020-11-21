#![feature(in_band_lifetimes)]

use anyhow::{anyhow, bail, Result};
use std::sync::{Arc, RwLock};

mod engine;

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
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Error)
        .level_for(env!("CARGO_CRATE_NAME"), log::LevelFilter::Trace)
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

    let engine = engine::Engine::new(&window)?;

    log::info!(
        "Initialized, took {} seconds",
        start_time.elapsed().as_secs_f32()
    );

    Ok(())
}
