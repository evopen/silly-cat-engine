use ash::version::{EntryV1_0, InstanceV1_0};

use anyhow::Result;
use ash::vk;

use std::ffi::{CStr, CString};
use std::sync::Arc;

pub struct Entry {
    handle: ash::Entry,
}

impl Entry {
    pub fn new() -> Result<Self> {
        let handle = ash::Entry::new()?;

        let result = Self { handle };

        Ok(result)
    }

    pub fn vulkan_version(&self) -> String {
        let version_str = match self.handle.try_enumerate_instance_version().unwrap() {
            // Vulkan 1.1+
            Some(version) => {
                let major = vk::version_major(version);
                let minor = vk::version_minor(version);
                let patch = vk::version_patch(version);
                format!("{}.{}.{}", major, minor, patch)
            }
            // Vulkan 1.0
            None => String::from("1.0"),
        };
        version_str
    }
}

pub struct Instance {
    handle: ash::Instance,
    entry: Arc<Entry>,
}

impl Instance {
    pub fn new(entry: Arc<Entry>, layer_names: &[&str], extension_names: &[&str]) -> Self {
        let app_name = CString::new(env!("CARGO_PKG_NAME")).unwrap();
        let engine_name = CString::new("Silly Cat Engine").unwrap();

        let appinfo = vk::ApplicationInfo::builder()
            .application_name(&app_name)
            .application_version(0)
            .engine_name(&engine_name)
            .engine_version(0)
            .api_version(vk::make_version(1, 2, 0));

        let layer_names = layer_names
            .iter()
            .map(|s| CString::new(*s).unwrap())
            .collect::<Vec<_>>();
        let layers_names_raw: Vec<*const i8> = layer_names
            .iter()
            .map(|raw_name| raw_name.as_ptr())
            .collect();

        let extension_names = extension_names
            .iter()
            .map(|s| CString::new(*s).unwrap())
            .collect::<Vec<_>>();
        let extension_names_raw = extension_names
            .iter()
            .map(|ext| ext.as_ptr())
            .collect::<Vec<_>>();

        let create_info = vk::InstanceCreateInfo::builder()
            .application_info(&appinfo)
            .enabled_layer_names(&layers_names_raw)
            .enabled_extension_names(&extension_names_raw);
        let handle = unsafe { entry.handle.create_instance(&create_info, None).unwrap() };

        let result = Self { handle, entry };

        result
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            self.handle.destroy_instance(None);
        }
    }
}

pub struct PhysicalDevice {
    handle: vk::PhysicalDevice,
    instance: Arc<Instance>,
    queue_family_index: u32,
}

impl PhysicalDevice {
    pub fn new(instance: Arc<Instance>, surface: &Surface) -> Self {
        let surface_loader =
            ash::extensions::khr::Surface::new(&instance.entry.handle, &instance.handle);
        let pdevices =
            unsafe { instance.handle.enumerate_physical_devices() }.expect("Physical device error");

        unsafe {
            let (pdevice, queue_family_index) = pdevices
                .iter()
                .map(|pdevice| {
                    let prop = instance.handle.get_physical_device_properties(*pdevice);

                    instance
                        .handle
                        .get_physical_device_queue_family_properties(*pdevice)
                        .iter()
                        .enumerate()
                        .filter_map(|(index, ref info)| {
                            let supports_graphic_and_surface =
                                info.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                                    && surface_loader
                                        .get_physical_device_surface_support(
                                            *pdevice,
                                            index as u32,
                                            surface.handle,
                                        )
                                        .unwrap();
                            if supports_graphic_and_surface
                                && prop.device_type == vk::PhysicalDeviceType::DISCRETE_GPU
                            {
                                Some((*pdevice, index))
                            } else {
                                None
                            }
                        })
                        .next()
                })
                .filter_map(|v| v)
                .next()
                .expect("Couldn't find suitable device.");

            Self {
                handle: pdevice,
                instance,
                queue_family_index: queue_family_index as u32,
            }
        }
    }
}

pub struct Surface {
    handle: vk::SurfaceKHR,
    instance: Arc<Instance>,
    required_extensions: Vec<String>,
}

impl Surface {
    pub fn new(
        instance: Arc<Instance>,
        window: &dyn raw_window_handle::HasRawWindowHandle,
    ) -> Self {
        let handle = unsafe {
            ash_window::create_surface(&instance.entry.handle, &instance.handle, window, None)
                .unwrap()
        };

        let required_extensions = ash_window::enumerate_required_extensions(window)
            .unwrap()
            .iter()
            .map(|s| s.to_str().unwrap().to_string())
            .collect::<Vec<_>>();

        Self {
            handle,
            instance,
            required_extensions,
        }
    }
}

pub struct Device {
    handle: ash::Device,
    pdevice: Arc<PhysicalDevice>,
}

impl Device {
    pub fn new(
        pdevice: Arc<PhysicalDevice>,
        device_features: &vk::PhysicalDeviceFeatures,
        device_extension_names: &[&str],
    ) -> Self {
        unsafe {
            let priorities = [1.0];

            let queue_info = [vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(pdevice.queue_family_index)
                .queue_priorities(&priorities)
                .build()];

            let device_extension_names = device_extension_names
                .iter()
                .map(|s| CString::new(*s).unwrap())
                .collect::<Vec<_>>();
            let device_extension_names_raw: Vec<*const i8> = device_extension_names
                .iter()
                .map(|raw_name| raw_name.as_ptr())
                .collect();

            let device_create_info = vk::DeviceCreateInfo::builder()
                .queue_create_infos(&queue_info)
                .enabled_extension_names(&device_extension_names_raw)
                .enabled_features(&device_features)
                .push_next(
                    &mut vk::PhysicalDeviceRayTracingPipelineFeaturesKHR::builder()
                        .ray_tracing_pipeline(true)
                        .build(),
                )
                .push_next(
                    &mut vk::PhysicalDeviceBufferDeviceAddressFeatures::builder()
                        .buffer_device_address(true)
                        .build(),
                )
                .push_next(
                    &mut vk::PhysicalDeviceAccelerationStructureFeaturesKHR::builder()
                        .acceleration_structure(true)
                        .build(),
                )
                .build();
            let handle = pdevice
                .instance
                .handle
                .create_device(pdevice.handle, &device_create_info, None)
                .unwrap();

            Self { handle, pdevice }
        }
    }
}

pub struct Allocator {
    handle: vk_mem::Allocator,
    device: Arc<Device>,
}

impl Allocator {
    pub fn new(device: Arc<Device>) -> Self {
        unsafe {
            let handle = vk_mem::Allocator::new(&vk_mem::AllocatorCreateInfo {
                physical_device: device.pdevice.handle,
                device: device.handle.clone(),
                instance: device.pdevice.instance.handle.clone(),
                flags: vk_mem::AllocatorCreateFlags::from_bits_unchecked(0x0000_0020),
                ..Default::default()
            })
            .unwrap();

            Self { handle, device }
        }
    }
}
