use std::borrow::Cow;
use std::ffi::{CStr, CString};

use anyhow::{anyhow, bail, Result};
use ash::version::{DeviceV1_0, EntryV1_0, InstanceV1_0};
use ash::vk;

use super::Device;
use super::Surface;

#[derive(Debug, Default, Clone)]
pub struct InstanceDescription {
    pub extension_names: Vec<&'static CStr>,
}

pub struct Instance {
    pub(super) entry: ash::Entry,
    instance_desc: InstanceDescription,
    pub(super) instance: ash::Instance,
    pub(super) surface_loader: ash::extensions::khr::Surface,
}

impl Instance {
    pub fn new(desc: InstanceDescription) -> Self {
        let entry = ash::Entry::new().unwrap();
        match entry.try_enumerate_instance_version().unwrap() {
            // Vulkan 1.1+
            Some(version) => {
                let major = vk::version_major(version);
                let minor = vk::version_minor(version);
                let patch = vk::version_patch(version);
                log::info!("Found Vulkan {}.{}.{}", major, minor, patch);
            }
            // Vulkan 1.0
            None => {
                log::info!("Found Vulkan 1.0");
            }
        }
        let layer_names = [CString::new("VK_LAYER_KHRONOS_validation").unwrap()];
        let layers_names_raw: Vec<*const i8> = layer_names
            .iter()
            .map(|raw_name| raw_name.as_ptr())
            .collect();

        let mut extension_names_raw: Vec<*const i8> = desc
            .extension_names
            .iter()
            .map(|raw_name| raw_name.as_ptr())
            .collect();
        extension_names_raw.push(ash::extensions::ext::DebugUtils::name().as_ptr());
        extension_names_raw.push(ash::extensions::ext::DebugReport::name().as_ptr());

        let app_info = vk::ApplicationInfo::builder().api_version(vk::make_version(1, 2, 0));
        let instance_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_layer_names(&layers_names_raw)
            .enabled_extension_names(&extension_names_raw);

        unsafe {
            let instance = entry.create_instance(&instance_info, None).unwrap();
            let surface_loader = ash::extensions::khr::Surface::new(&entry, &instance);

            Self {
                entry,
                instance_desc: desc,
                instance,
                surface_loader,
            }
        }
    }

    pub fn create_surface(&self, window: &winit::window::Window) -> Surface {
        if !self
            .instance_desc
            .extension_names
            .contains(&ash::extensions::khr::Surface::name())
        {
            panic!("instance does not load surface extension");
        }
        Surface::new(&self.entry, &self.instance, window, &self.surface_loader)
    }

    pub fn create_device(&self, surface: &Surface) -> Device {
        Device::new(&self.entry, &self.instance, &surface.surface)
    }
}
