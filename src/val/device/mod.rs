mod debug;
mod queue;

use std::borrow::Cow;
use std::ffi::{CStr, CString};

use anyhow::{anyhow, bail, Result};
use ash::version::{DeviceV1_0, EntryV1_0, InstanceV1_0};
use ash::vk;

use super::Instance;
use super::Surface;
use super::Swapchain;
pub use queue::Queue;

pub struct Device {
    debug_call_messenger: vk::DebugUtilsMessengerEXT,
    device: ash::Device,
    pdevice: vk::PhysicalDevice,
    instance: ash::Instance,
    queue_family_index: u32,
}

impl Device {
    pub(super) fn new(
        entry: &ash::Entry,
        instance: &ash::Instance,
        surface: &vk::SurfaceKHR,
    ) -> Self {
        let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                    | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
            )
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            )
            .pfn_user_callback(Some(debug::vulkan_debug_callback));
        let debug_utils_loader = ash::extensions::ext::DebugUtils::new(entry, instance);
        unsafe {
            let debug_call_messenger = debug_utils_loader
                .create_debug_utils_messenger(&debug_info, None)
                .unwrap();
            let pdevices = instance
                .enumerate_physical_devices()
                .expect("Physical device error");
            let surface_loader = ash::extensions::khr::Surface::new(entry, instance);

            let (pdevice, queue_family_index) = pdevices
                .iter()
                .map(|pdevice| {
                    instance
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
                                            *surface,
                                        )
                                        .unwrap();
                            if supports_graphic_and_surface {
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
            let queue_family_index = queue_family_index as u32;

            let priorities = [1.0];

            let queue_info = [vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(queue_family_index)
                .queue_priorities(&priorities)
                .build()];

            let device_extension_names_raw = [ash::extensions::khr::Swapchain::name().as_ptr()];

            let features = vk::PhysicalDeviceFeatures::default();

            let device_create_info = vk::DeviceCreateInfo::builder()
                .queue_create_infos(&queue_info)
                .enabled_extension_names(&device_extension_names_raw)
                .enabled_features(&features);

            let device = instance
                .create_device(pdevice, &device_create_info, None)
                .unwrap();

            let queue = device.get_device_queue(queue_family_index, 0);

            Self {
                debug_call_messenger,
                device,
                pdevice,
                instance: instance.clone(),
                queue_family_index,
            }
        }
    }

    pub fn create_swapchain(&self, surface: &Surface) -> Swapchain {
        let surface_format = unsafe {
            surface
                .surface_loader
                .get_physical_device_surface_formats(self.pdevice, surface.surface)
        }
        .unwrap()[0];

        let surface_capabilities = unsafe {
            surface
                .surface_loader
                .get_physical_device_surface_capabilities(self.pdevice, surface.surface)
        }
        .unwrap();

        let pre_transform = if surface_capabilities
            .supported_transforms
            .contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
        {
            vk::SurfaceTransformFlagsKHR::IDENTITY
        } else {
            surface_capabilities.current_transform
        };

        let mut desired_image_count = surface_capabilities.min_image_count + 1;
        if surface_capabilities.max_image_count > 0
            && desired_image_count > surface_capabilities.max_image_count
        {
            desired_image_count = surface_capabilities.max_image_count;
        }

        let present_modes = unsafe {
            surface
                .surface_loader
                .get_physical_device_surface_present_modes(self.pdevice, surface.surface)
        }
        .unwrap();
        let present_mode = present_modes
            .iter()
            .cloned()
            .find(|&mode| mode == vk::PresentModeKHR::MAILBOX)
            .unwrap_or(vk::PresentModeKHR::FIFO);

        let swapchain_create_info = vk::SwapchainCreateInfoKHR::builder()
            .surface(surface.surface)
            .min_image_count(desired_image_count)
            .image_color_space(surface_format.color_space)
            .image_format(surface_format.format)
            .image_extent(surface_capabilities.current_extent)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(pre_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(false)
            .image_array_layers(1);
        Swapchain::new(&self.instance, &self.device, &swapchain_create_info)
    }

    pub fn get_queue(&self) -> Queue {
        Queue::new(&self.device, self.queue_family_index, 0)
    }
}
