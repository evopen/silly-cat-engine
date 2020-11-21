use std::borrow::Cow;
use std::ffi::{CStr, CString};

use anyhow::{anyhow, bail, Result};
use ash::version::{DeviceV1_0, EntryV1_0, InstanceV1_0};
use ash::vk;

unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    let callback_data = *p_callback_data;
    let message_id_number: i32 = callback_data.message_id_number as i32;

    let message_id_name = if callback_data.p_message_id_name.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy()
    };

    let message = if callback_data.p_message.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message).to_string_lossy()
    };

    println!(
        "{:?}:\n{:?} [{} ({})] : {}\n",
        message_severity,
        message_type,
        message_id_name,
        &message_id_number.to_string(),
        message,
    );

    vk::FALSE
}

pub struct Engine {
    size: winit::dpi::PhysicalSize<u32>,
    entry: ash::Entry,
    instance: ash::Instance,
    device: ash::Device,
    queue: vk::Queue,
}

impl Engine {
    pub fn new(window: &winit::window::Window) -> Result<Self> {
        let size = window.inner_size();

        let entry = ash::Entry::new()?;
        match entry.try_enumerate_instance_version()? {
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

        let layer_names = [CString::new("VK_LAYER_KHRONOS_validation")?];
        let layers_names_raw: Vec<*const i8> = layer_names
            .iter()
            .map(|raw_name| raw_name.as_ptr())
            .collect();

        let surface_extensions = ash_window::enumerate_required_extensions(window)?;

        let mut extension_names_raw = surface_extensions
            .iter()
            .map(|ext| ext.as_ptr())
            .collect::<Vec<_>>();
        extension_names_raw.push(ash::extensions::ext::DebugUtils::name().as_ptr());

        unsafe {
            let app_info = vk::ApplicationInfo::builder().api_version(vk::make_version(1, 2, 0));
            let instance_info = vk::InstanceCreateInfo::builder()
                .application_info(&app_info)
                .enabled_layer_names(&layers_names_raw)
                .enabled_extension_names(&extension_names_raw);
            let instance = entry.create_instance(&instance_info, None)?;
            let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
                .message_severity(
                    vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                        | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                        | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
                )
                .message_type(vk::DebugUtilsMessageTypeFlagsEXT::all())
                .pfn_user_callback(Some(vulkan_debug_callback));
            let debug_utils_loader = ash::extensions::ext::DebugUtils::new(&entry, &instance);
            let debug_call_messenger =
                debug_utils_loader.create_debug_utils_messenger(&debug_info, None)?;

            let surface = ash_window::create_surface(&entry, &instance, window, None)?;
            let surface_loader = ash::extensions::khr::Surface::new(&entry, &instance);

            let pdevices = instance
                .enumerate_physical_devices()
                .expect("Physical device error");
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
                                            surface,
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

            let device_extension_names_raw = [ash::extensions::khr::Swapchain::name().as_ptr()];

            let device_features = vk::PhysicalDeviceFeatures::default();

            let priorities = [1.0];

            let queue_info = [vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(queue_family_index)
                .queue_priorities(&priorities)
                .build()];

            let device_create_info = vk::DeviceCreateInfo::builder()
                .queue_create_infos(&queue_info)
                .enabled_extension_names(&device_extension_names_raw)
                .enabled_features(&device_features);

            let device = instance.create_device(pdevice, &device_create_info, None)?;

            let queue = device.get_device_queue(queue_family_index, 0);

            let surface_format =
                surface_loader.get_physical_device_surface_formats(pdevice, surface)?[0];

            let swapchain_loader = ash::extensions::khr::Swapchain::new(&instance, &device);

            let surface_capabilities = surface_loader
                .get_physical_device_surface_capabilities(pdevice, surface)
                .unwrap();
            let pre_transform = match surface_capabilities
                .supported_transforms
                .contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
            {
                true => vk::SurfaceTransformFlagsKHR::IDENTITY,
                false => surface_capabilities.current_transform,
            };

            let swapchain_info = vk::SwapchainCreateInfoKHR::builder()
                .surface(surface)
                .min_image_count(2)
                .image_format(surface_format.format)
                .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                .pre_transform(vk::SurfaceTransformFlagsKHR::IDENTITY)
                .present_mode(vk::PresentModeKHR::FIFO)
                .image_extent(vk::Extent2D {
                    width: size.width,
                    height: size.height,
                })
                .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
                .image_array_layers(1)
                .pre_transform(pre_transform);

            swapchain_loader.create_swapchain(&swapchain_info, None)?;

            Ok(Self {
                size,
                entry,
                instance,
                device,
                queue,
            })
        }
    }
}
