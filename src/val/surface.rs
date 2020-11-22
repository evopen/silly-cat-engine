use anyhow::{anyhow, bail, Result};
use ash::version::{DeviceV1_0, EntryV1_0, InstanceV1_0};
use ash::vk;

use super::Instance;

pub struct Surface {
    pub(super) surface: vk::SurfaceKHR,
    pub(super) surface_loader: ash::extensions::khr::Surface,
    pub(super) size: winit::dpi::PhysicalSize<u32>,
}

impl Surface {
    pub fn new(
        entry: &ash::Entry,
        instance: &ash::Instance,
        window: &winit::window::Window,
    ) -> Self {
        let size = window.inner_size();
        unsafe {
            let surface = ash_window::create_surface(entry, instance, window, None).unwrap();
            let surface_loader = ash::extensions::khr::Surface::new(entry, instance);
            Self {
                size,
                surface,
                surface_loader,
            }
        }
    }
}
