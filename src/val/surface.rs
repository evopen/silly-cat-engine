use anyhow::{anyhow, bail, Result};
use ash::version::{DeviceV1_0, EntryV1_0, InstanceV1_0};
use ash::vk;

use super::Instance;

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct Surface {
    pub(super) surface: vk::SurfaceKHR,
    pub(super) size: winit::dpi::PhysicalSize<u32>,
}

impl Surface {
    pub(super) fn new(
        entry: &ash::Entry,
        instance: &ash::Instance,
        window: &winit::window::Window,
        surface_loader: &ash::extensions::khr::Surface,
    ) -> Self {
        let size = window.inner_size();
        unsafe {
            let surface = ash_window::create_surface(entry, instance, window, None).unwrap();
            Self { size, surface }
        }
    }
}
