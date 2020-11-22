#![allow(unused)]

use anyhow::Result;
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use std::sync::atomic;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::{path::PathBuf, sync::Arc};

use crate::val;

pub struct Engine {
    size: winit::dpi::PhysicalSize<u32>,
    surface: val::Surface,
    device: val::Device,
    queue: val::Queue,
    swapchain: val::Swapchain,
    start_time: std::time::Instant,
    scale_factor: f64,
}

impl Engine {
    pub async fn new(
        window: &winit::window::Window,
        log_rx: crossbeam::channel::Receiver<String>,
    ) -> Self {
        let size = window.inner_size();
        let instance = val::Instance::new(val::InstanceDescription {
            extension_names: ash_window::enumerate_required_extensions(window).unwrap(),
        });
        let surface = unsafe { instance.create_surface(window) };
        let device = instance.create_device(&surface);
        let queue = device.get_queue();

        let swapchain = device.create_swapchain(&surface);

        let scale_factor = window.scale_factor();

        Self {
            surface,
            device,
            queue,
            size,
            start_time: std::time::Instant::now(),
            scale_factor,
            swapchain,
        }
    }

    pub fn input(&mut self, event: &winit::event::WindowEvent) {
        match event {
            winit::event::WindowEvent::Resized(new_inner_size) => {
                self.resize(new_inner_size);
            }
            _ => {}
        }
    }

    fn resize(&mut self, new_size: &winit::dpi::PhysicalSize<u32>) {
        self.size.clone_from(new_size);
        self.swapchain = self.device.create_swapchain(&self.surface);
        log::info!(
            "swap chain resized to {}, {}",
            self.size.width,
            self.size.height
        );
        self.app.resize(&new_size);
    }

    pub fn update(&mut self) {
        self.app.update();
    }

    pub fn render(&mut self) {
        let frame = self.swap_chain.get_current_frame().unwrap().output;
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("main encoder"),
            });
        self.app.encode(&mut encoder, &frame.view);
        self.queue.submit(std::iter::once(encoder.finish()));
    }
}
