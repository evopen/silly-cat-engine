#![feature(negative_impls)]
#![allow(unused)]

use ash::version::{DeviceV1_0, DeviceV1_2, EntryV1_0, InstanceV1_0, InstanceV1_1};

use anyhow::Result;

use bytemuck::cast_slice;
use vk::Handle;

use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap, LinkedList};
use std::ffi::{CStr, CString};

use std::sync::{Arc, Mutex};

pub use ash::vk;
pub use vk_mem::MemoryUsage;

pub mod name {
    pub mod instance {
        pub enum Layer {
            KhronosValidation,
            LunargMonitor,
            LunargGfxreconstruct,
        }
        impl Into<&'static str> for &Layer {
            fn into(self) -> &'static str {
                match self {
                    Layer::KhronosValidation => "VK_LAYER_KHRONOS_validation",
                    Layer::LunargMonitor => "VK_LAYER_LUNARG_monitor",
                    Layer::LunargGfxreconstruct => "VK_LAYER_LUNARG_gfxreconstruct",
                }
            }
        }

        pub enum Extension {
            ExtDebugUtils,
            KhrWin32Surface,
            KhrSurface,
            KhrXlibSurface,
            KhrXcbSurface,
            KhrDisplay,
        }

        impl Into<&'static str> for &Extension {
            fn into(self) -> &'static str {
                match self {
                    Extension::ExtDebugUtils => "VK_EXT_debug_utils",
                    Extension::KhrWin32Surface => "VK_KHR_win32_surface",
                    Extension::KhrSurface => "VK_KHR_surface",
                    Extension::KhrXlibSurface => "VK_KHR_xlib_surface",
                    Extension::KhrXcbSurface => "VK_KHR_xcb_surface",
                    Extension::KhrDisplay => "VK_KHR_display",
                }
            }
        }
    }
    pub mod device {
        mod layer {}

        #[derive(Debug, PartialEq)]
        pub enum Extension {
            KhrSwapchain,
            KhrDeferredHostOperations,
            KhrRayTracingPipeline,
            KhrAccelerationStructure,
            KhrShaderNonSemanticInfo,
            KhrRayQuery,
        }

        impl Into<&'static str> for &Extension {
            fn into(self) -> &'static str {
                match self {
                    Extension::KhrSwapchain => "VK_KHR_swapchain",
                    Extension::KhrDeferredHostOperations => "VK_KHR_deferred_host_operations",
                    Extension::KhrRayTracingPipeline => "VK_KHR_ray_tracing_pipeline",
                    Extension::KhrAccelerationStructure => "VK_KHR_acceleration_structure",
                    Extension::KhrShaderNonSemanticInfo => "VK_KHR_shader_non_semantic_info",
                    Extension::KhrRayQuery => "VK_KHR_ray_query",
                }
            }
        }
    }
}

pub struct Entry {
    handle: ash::Entry,
}

impl Entry {
    pub fn new() -> Result<Self> {
        let handle = unsafe { ash::Entry::new()? };

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

    pub fn supported_instance_layers(&self) -> Vec<String> {
        self.handle
            .enumerate_instance_layer_properties()
            .unwrap()
            .iter()
            .map(|layer| {
                unsafe { CStr::from_ptr(layer.layer_name.as_ptr() as *const std::os::raw::c_char) }
                    .to_str()
                    .unwrap()
                    .to_owned()
            })
            .collect::<Vec<_>>()
    }

    pub fn supported_instance_extensions(&self) -> Vec<String> {
        self.handle
            .enumerate_instance_extension_properties()
            .unwrap()
            .iter()
            .map(|ext| {
                unsafe {
                    CStr::from_ptr(ext.extension_name.as_ptr() as *const std::os::raw::c_char)
                }
                .to_str()
                .unwrap()
                .to_owned()
            })
            .collect::<Vec<_>>()
    }
}

pub struct Instance {
    handle: ash::Instance,
    entry: Arc<Entry>,
    surface_loader: ash::extensions::khr::Surface,
    debug_utils_loader: ash::extensions::ext::DebugUtils,
    display_loader: ash::extensions::khr::Display,
}

impl Instance {
    pub fn new(
        entry: Arc<Entry>,
        layers: &[name::instance::Layer],
        extensions: &[name::instance::Extension],
    ) -> Self {
        let app_name = CString::new(env!("CARGO_PKG_NAME")).unwrap();
        let engine_name = CString::new("Silly Cat Engine").unwrap();

        let appinfo = vk::ApplicationInfo::builder()
            .application_name(&app_name)
            .application_version(0)
            .engine_name(&engine_name)
            .engine_version(0)
            .api_version(vk::make_version(1, 2, 0));

        let layer_names = layers
            .iter()
            .map(|layer| CString::new::<&'static str>(layer.into()).unwrap())
            .collect::<Vec<_>>();
        let layers_names_raw: Vec<*const i8> = layer_names
            .iter()
            .map(|raw_name| raw_name.as_ptr())
            .collect();

        let supported_layers = entry.supported_instance_layers();
        for layer in layers {
            let name: &str = layer.into();
            if !supported_layers.contains(&name.to_owned()) {
                panic!("not support layer {}", &name);
            }
        }

        let extension_names = extensions
            .iter()
            .map(|extension| CString::new::<&'static str>(extension.into()).unwrap())
            .collect::<Vec<_>>();
        let extension_names_raw = extension_names
            .iter()
            .map(|ext| ext.as_ptr())
            .collect::<Vec<_>>();

        let supported_extensions = entry.supported_instance_extensions();
        for extension in extensions {
            let name: &str = extension.into();
            if !supported_extensions.contains(&name.to_owned()) {
                panic!("not support extension {}", &name);
            }
        }

        let create_info = vk::InstanceCreateInfo::builder()
            .application_info(&appinfo)
            .enabled_layer_names(&layers_names_raw)
            .enabled_extension_names(&extension_names_raw);
        let handle = unsafe { entry.handle.create_instance(&create_info, None).unwrap() };

        let surface_loader = ash::extensions::khr::Surface::new(&entry.handle, &handle);

        let debug_utils_loader = ash::extensions::ext::DebugUtils::new(&entry.handle, &handle);

        let display_loader = ash::extensions::khr::Display::new(&entry.handle, &handle);

        let result = Self {
            handle,
            entry,
            surface_loader,
            debug_utils_loader,
            display_loader,
        };

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

pub struct PhysicalDeviceRayTracingPipelineProperties {
    pub shader_group_handle_size: u32,
    pub max_ray_recursion_depth: u32,
    pub max_shader_group_stride: u32,
    pub shader_group_base_alignment: u32,
    pub max_ray_dispatch_invocation_count: u32,
    pub shader_group_handle_alignment: u32,
    pub max_ray_hit_attribute_size: u32,
}

pub struct PhysicalDevice {
    handle: vk::PhysicalDevice,
    instance: Arc<Instance>,
    queue_family_index: u32,
    ray_tracing_pipeline_properties: PhysicalDeviceRayTracingPipelineProperties,
}

impl PhysicalDevice {
    pub fn new(instance: Arc<Instance>, surface: Option<&Surface>) -> Self {
        let surface_loader = &instance.surface_loader;
        let pdevices =
            unsafe { instance.handle.enumerate_physical_devices() }.expect("Physical device error");

        unsafe {
            let (pdevice, queue_family_index) = pdevices
                .iter()
                .filter_map(|pdevice| {
                    let prop = instance.handle.get_physical_device_properties(*pdevice);
                    let queue_families_props = instance
                        .handle
                        .get_physical_device_queue_family_properties(*pdevice);
                    if prop.device_type == vk::PhysicalDeviceType::INTEGRATED_GPU {
                        return None;
                    }

                    let a = match &surface {
                        Some(surface) => {
                            queue_families_props
                                .iter()
                                .enumerate()
                                .filter_map(|(index, info)| {
                                    let supports_graphic_and_surface =
                                        info.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                                            && surface_loader
                                                .get_physical_device_surface_support(
                                                    *pdevice,
                                                    index as u32,
                                                    surface.handle,
                                                )
                                                .unwrap();
                                    if supports_graphic_and_surface {
                                        Some((*pdevice, index))
                                    } else {
                                        None
                                    }
                                })
                                .next()
                                .unwrap()
                        }
                        None => {
                            queue_families_props
                                .iter()
                                .enumerate()
                                .filter_map(|(index, info)| {
                                    let supports_graphic =
                                        info.queue_flags.contains(vk::QueueFlags::GRAPHICS);
                                    if supports_graphic {
                                        Some((*pdevice, index))
                                    } else {
                                        None
                                    }
                                })
                                .next()
                                .unwrap()
                        }
                    };
                    Some(a)
                })
                .next()
                .unwrap();

            let mut props = vk::PhysicalDeviceRayTracingPipelinePropertiesKHR::default();
            instance.handle.get_physical_device_properties2(
                pdevice,
                &mut vk::PhysicalDeviceProperties2::builder()
                    .push_next(&mut props)
                    .build(),
            );
            let prop = instance.handle.get_physical_device_properties(pdevice);
            let device_name = unsafe { CStr::from_ptr(prop.device_name.as_ptr()) }
                .to_str()
                .unwrap();
            log::info!("Selected Device: {}", device_name);
            let ray_tracing_pipeline_properties = PhysicalDeviceRayTracingPipelineProperties {
                shader_group_handle_size: props.shader_group_handle_size,
                max_ray_recursion_depth: props.max_ray_recursion_depth,
                max_shader_group_stride: props.max_shader_group_stride,
                shader_group_base_alignment: props.shader_group_base_alignment,
                max_ray_dispatch_invocation_count: props.max_ray_dispatch_invocation_count,
                shader_group_handle_alignment: props.shader_group_handle_alignment,
                max_ray_hit_attribute_size: props.max_ray_hit_attribute_size,
            };

            Self {
                handle: pdevice,
                instance,
                queue_family_index: queue_family_index as u32,
                ray_tracing_pipeline_properties,
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

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            self.instance
                .surface_loader
                .destroy_surface(self.handle, None);
        }
    }
}

struct PhysicalDeviceFeatureEnablement {
    ray_tracing_pipeline: vk::PhysicalDeviceRayTracingPipelineFeaturesKHR,
    acceleration_structure: vk::PhysicalDeviceAccelerationStructureFeaturesKHR,
    ray_query: vk::PhysicalDeviceRayQueryFeaturesKHR,
}

pub struct Device {
    handle: ash::Device,
    pdevice: Arc<PhysicalDevice>,
    acceleration_structure_loader: ash::extensions::khr::AccelerationStructure,
    swapchain_loader: ash::extensions::khr::Swapchain,
    ray_tracing_pipeline_loader: ash::extensions::khr::RayTracingPipeline,
}

impl Device {
    pub fn new(
        pdevice: Arc<PhysicalDevice>,
        device_features: &vk::PhysicalDeviceFeatures,
        device_extensions: &[name::device::Extension],
    ) -> Self {
        unsafe {
            let priorities = [1.0];

            let queue_info = [vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(pdevice.queue_family_index)
                .queue_priorities(&priorities)
                .build()];

            let device_extension_names = device_extensions
                .iter()
                .map(|extension| CString::new::<&'static str>(extension.into()).unwrap())
                .collect::<Vec<_>>();
            let device_extension_names_raw: Vec<*const i8> = device_extension_names
                .iter()
                .map(|raw_name| raw_name.as_ptr())
                .collect();

            let mut ray_tracing_pipeline_pnext =
                vk::PhysicalDeviceRayTracingPipelineFeaturesKHR::builder()
                    .ray_tracing_pipeline(true)
                    .build();
            let mut acceleration_structure_pnext =
                vk::PhysicalDeviceAccelerationStructureFeaturesKHR::builder()
                    .acceleration_structure(true)
                    .build();
            let mut ray_query_pnext = vk::PhysicalDeviceRayQueryFeaturesKHR::builder()
                .ray_query(true)
                .build();
            let mut device_buffer_address_pnext =
                vk::PhysicalDeviceBufferDeviceAddressFeatures::builder()
                    .buffer_device_address(true)
                    .build();
            let mut fea_16_bit_storage_pnext = vk::PhysicalDevice16BitStorageFeatures::builder()
                .uniform_and_storage_buffer16_bit_access(true)
                .storage_buffer16_bit_access(true)
                .storage_input_output16(false)
                .storage_push_constant16(true)
                .build();
            let mut scalar_block_layout_pnext =
                vk::PhysicalDeviceScalarBlockLayoutFeatures::builder()
                    .scalar_block_layout(true)
                    .build();

            let mut device_create_info = vk::DeviceCreateInfo::builder()
                .queue_create_infos(&queue_info)
                .enabled_extension_names(&device_extension_names_raw)
                .enabled_features(&device_features);

            device_create_info =
                if device_extensions.contains(&name::device::Extension::KhrRayTracingPipeline) {
                    device_create_info.push_next(&mut ray_tracing_pipeline_pnext)
                } else {
                    device_create_info
                };
            device_create_info =
                if device_extensions.contains(&name::device::Extension::KhrRayQuery) {
                    device_create_info.push_next(&mut ray_query_pnext)
                } else {
                    device_create_info
                };
            device_create_info =
                if device_extensions.contains(&name::device::Extension::KhrAccelerationStructure) {
                    device_create_info.push_next(&mut acceleration_structure_pnext)
                } else {
                    device_create_info
                };

            device_create_info = device_create_info
                .push_next(&mut device_buffer_address_pnext)
                .push_next(&mut fea_16_bit_storage_pnext)
                .push_next(&mut scalar_block_layout_pnext);

            let handle = pdevice
                .instance
                .handle
                .create_device(pdevice.handle, &device_create_info, None)
                .unwrap();

            let acceleration_structure_loader =
                ash::extensions::khr::AccelerationStructure::new(&pdevice.instance.handle, &handle);

            let swapchain_loader =
                ash::extensions::khr::Swapchain::new(&pdevice.instance.handle, &handle);

            let ray_tracing_pipeline_loader =
                ash::extensions::khr::RayTracingPipeline::new(&pdevice.instance.handle, &handle);

            Self {
                handle,
                pdevice,
                acceleration_structure_loader,
                swapchain_loader,
                ray_tracing_pipeline_loader,
            }
        }
    }

    pub fn pdevice(&self) -> &PhysicalDevice {
        &self.pdevice
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            self.handle.destroy_device(None);
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

    pub fn stats(&self) -> vk_mem::ffi::VmaStats {
        self.handle.calculate_stats().unwrap()
    }

    pub fn device(&self) -> &Arc<Device> {
        &self.device
    }
}

impl Drop for Allocator {
    fn drop(&mut self) {
        self.handle.destroy();
    }
}

pub struct DescriptorPool {
    handle: vk::DescriptorPool,
    device: Arc<Device>,
}

impl DescriptorPool {
    pub fn new(
        device: Arc<Device>,
        descriptor_pool_size: &[vk::DescriptorPoolSize],
        max_sets: u32,
    ) -> Self {
        unsafe {
            let info = vk::DescriptorPoolCreateInfo::builder()
                .pool_sizes(descriptor_pool_size)
                .max_sets(max_sets)
                .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET)
                .build();
            let handle = device.handle.create_descriptor_pool(&info, None).unwrap();
            Self { handle, device }
        }
    }
}

impl Drop for DescriptorPool {
    fn drop(&mut self) {
        unsafe {
            self.device
                .handle
                .destroy_descriptor_pool(self.handle, None);
        }
    }
}

pub struct Buffer {
    allocator: Arc<Allocator>,
    handle: vk::Buffer,
    allocation: vk_mem::Allocation,
    mapped: std::sync::atomic::AtomicBool,
    device_address: vk::DeviceAddress,
    size: usize,
    allocation_info: vk_mem::AllocationInfo,
    property_flags: vk::MemoryPropertyFlags,
}

impl std::fmt::Debug for Buffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Buffer")
            .field("handle", &self.handle)
            .field("size", &self.size)
            .field("mapped", &self.mapped)
            .finish()
    }
}

impl Buffer {
    pub fn new<I>(
        name: Option<&str>,
        allocator: Arc<Allocator>,
        size: I,
        buffer_usage: vk::BufferUsageFlags,
        memory_usage: vk_mem::MemoryUsage,
    ) -> Self
    where
        I: num_traits::PrimInt,
    {
        let (handle, allocation, allocation_info) = allocator
            .handle
            .create_buffer(
                &vk::BufferCreateInfo::builder()
                    .usage(
                        buffer_usage
                            | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                            | vk::BufferUsageFlags::TRANSFER_DST,
                    )
                    .size(size.to_u64().unwrap())
                    .build(),
                &vk_mem::AllocationCreateInfo {
                    usage: memory_usage,
                    ..Default::default()
                },
            )
            .unwrap();

        let device = &allocator.device;
        unsafe {
            if let Some(name) = name {
                device
                    .pdevice
                    .instance
                    .debug_utils_loader
                    .debug_utils_set_object_name(
                        device.handle.handle(),
                        &vk::DebugUtilsObjectNameInfoEXT::builder()
                            .object_handle(handle.as_raw())
                            .object_type(vk::ObjectType::BUFFER)
                            .object_name(CString::new(name).unwrap().as_ref())
                            .build(),
                    )
                    .unwrap();
            }
            let device_address = allocator.device.handle.get_buffer_device_address(
                &vk::BufferDeviceAddressInfo::builder()
                    .buffer(handle)
                    .build(),
            );

            let property_flags = allocator
                .handle
                .get_memory_type_properties(allocation_info.get_memory_type())
                .unwrap();

            Self {
                handle,
                allocation,
                mapped: std::sync::atomic::AtomicBool::new(false),
                device_address,
                size: size.to_usize().unwrap(),
                allocator,
                allocation_info,
                property_flags,
            }
        }
    }

    pub fn new_init_host<I: AsRef<[u8]>>(
        name: Option<&str>,
        allocator: Arc<Allocator>,
        buffer_usage: vk::BufferUsageFlags,
        memory_usage: vk_mem::MemoryUsage,
        data: I,
    ) -> Self {
        let data = data.as_ref();
        let mut buffer = Self::new(
            name,
            allocator,
            data.len(),
            buffer_usage
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::TRANSFER_DST,
            memory_usage,
        );
        let mapped = buffer.map();
        let mapped_slice = unsafe { std::slice::from_raw_parts_mut(mapped, buffer.size) };
        mapped_slice.copy_from_slice(data.as_ref());
        buffer.unmap();
        buffer
    }

    pub fn new_init_device<I: AsRef<[u8]>>(
        name: Option<&str>,
        allocator: Arc<Allocator>,
        buffer_usage: vk::BufferUsageFlags,
        memory_usage: vk_mem::MemoryUsage,
        queue: &mut Queue,
        command_pool: Arc<CommandPool>,
        data: I,
    ) -> Self {
        let data = data.as_ref();
        let buffer = Self::new(
            name,
            allocator.clone(),
            data.len(),
            buffer_usage
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::TRANSFER_DST,
            memory_usage,
        );
        if !buffer.is_mappable() {
            let staging_buffer = Arc::new(Self::new(
                Some("staging buffer"),
                allocator.clone(),
                data.len(),
                vk::BufferUsageFlags::TRANSFER_SRC,
                vk_mem::MemoryUsage::CpuToGpu,
            ));
            staging_buffer.copy_from(data);
            let mut cmd_buf = CommandBuffer::new(command_pool);
            cmd_buf.encode(|manager| unsafe {
                manager.copy_buffer_raw(
                    &staging_buffer,
                    &buffer,
                    &[vk::BufferCopy::builder().size(data.len() as u64).build()],
                );
            });
            let timeline_semaphore = TimelineSemaphore::new(allocator.device.clone());
            queue.submit_timeline(
                cmd_buf,
                &[&timeline_semaphore],
                &[0],
                &[vk::PipelineStageFlags::ALL_COMMANDS],
                &[1],
            );
            timeline_semaphore.wait_for(1);
        } else {
            buffer.copy_from(data);
            buffer.flush();
        }
        buffer
    }

    pub fn map(&self) -> *mut u8 {
        if !self.is_mappable() {
            panic!("memory is not host visible");
        }

        let ptr = self.allocator.handle.map_memory(&self.allocation).unwrap();
        self.mapped
            .compare_exchange(
                false,
                true,
                std::sync::atomic::Ordering::SeqCst,
                std::sync::atomic::Ordering::SeqCst,
            )
            .expect("already mapped");
        ptr
    }

    pub fn unmap(&self) {
        self.mapped
            .compare_exchange(
                true,
                false,
                std::sync::atomic::Ordering::SeqCst,
                std::sync::atomic::Ordering::SeqCst,
            )
            .expect("not mapped");
        self.allocator.handle.unmap_memory(&self.allocation);
    }

    pub fn memory_type(&self) -> u32 {
        self.allocation_info.get_memory_type()
    }

    pub fn device_address(&self) -> vk::DeviceAddress {
        self.device_address
    }

    pub fn copy_from<I: AsRef<[u8]>>(&self, data: I) {
        let data = data.as_ref();
        let mapped = self.map();
        let mapped_bytes = unsafe { std::slice::from_raw_parts_mut(mapped, self.size) };
        mapped_bytes.copy_from_slice(data);
        self.unmap();
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn is_device_local(&self) -> bool {
        self.property_flags & vk::MemoryPropertyFlags::DEVICE_LOCAL
            != vk::MemoryPropertyFlags::empty()
    }

    pub fn is_mappable(&self) -> bool {
        self.property_flags & vk::MemoryPropertyFlags::HOST_VISIBLE
            != vk::MemoryPropertyFlags::empty()
    }

    pub fn flush(&self) {
        self.allocator
            .handle
            .flush_allocation(&self.allocation, 0, vk::WHOLE_SIZE as usize);
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        if self.mapped.load(std::sync::atomic::Ordering::SeqCst) {
            self.unmap();
        }
        self.allocator
            .handle
            .destroy_buffer(self.handle, &self.allocation);
    }
}

pub struct Queue {
    handle: vk::Queue,
    device: Arc<Device>,
    command_buffers:
        HashMap<vk::CommandBuffer, (Arc<std::sync::atomic::AtomicBool>, CommandBuffer)>,
}

impl Queue {
    pub fn new(device: Arc<Device>) -> Self {
        unsafe {
            let handle = device
                .handle
                .get_device_queue(device.pdevice.queue_family_index, 0);
            Self {
                handle,
                device,
                command_buffers: HashMap::new(),
            }
        }
    }

    pub fn clean_command_buffers(&mut self) {
        let mut removal_list = Vec::with_capacity(self.command_buffers.len());
        for (handle, (in_use, _)) in self.command_buffers.iter() {
            if !in_use.load(std::sync::atomic::Ordering::SeqCst) {
                removal_list.push(*handle);
            }
        }
        for removal in removal_list {
            self.command_buffers.remove(&removal);
        }
    }

    pub fn submit_binary(
        &mut self,
        command_buffer: CommandBuffer,
        wait_semaphore: &[&BinarySemaphore],
        wait_stages: &[vk::PipelineStageFlags],
        signal_semaphore: &[&BinarySemaphore],
    ) -> Arc<Fence> {
        self.clean_command_buffers();

        let wait_handles = wait_semaphore.iter().map(|s| s.handle).collect::<Vec<_>>();
        let signal_handles = signal_semaphore
            .iter()
            .map(|s| s.handle)
            .collect::<Vec<_>>();

        let submit_info = vk::SubmitInfo::builder()
            .command_buffers(&[command_buffer.handle])
            .wait_semaphores(wait_handles.as_slice())
            .wait_dst_stage_mask(wait_stages)
            .signal_semaphores(signal_handles.as_slice())
            .build();

        let fence = Arc::new(Fence::new(self.device.clone(), false));

        let in_use = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let in_use_signaler = in_use.clone();

        unsafe {
            self.device
                .handle
                .queue_submit(self.handle, &[submit_info], fence.handle)
                .unwrap();
        }
        let fence_cloned = fence.clone();
        let _task = tokio::task::spawn(async move {
            fence_cloned.wait();
            in_use_signaler.store(false, std::sync::atomic::Ordering::SeqCst);
        });

        self.command_buffers
            .insert(command_buffer.handle, (in_use, command_buffer));

        fence
    }

    pub fn submit_timeline(
        &mut self,
        command_buffer: CommandBuffer,
        timeline_semaphores: &[&TimelineSemaphore],
        wait_values: &[u64],
        wait_stages: &[vk::PipelineStageFlags],
        signal_values: &[u64],
    ) {
        self.clean_command_buffers();
        unsafe {
            let semaphore_handles = timeline_semaphores
                .iter()
                .map(|s| s.handle)
                .collect::<Vec<vk::Semaphore>>();

            let fence = Fence::new(self.device.clone(), false);
            self.device
                .handle
                .queue_submit(
                    self.handle,
                    &[vk::SubmitInfo::builder()
                        .command_buffers(&[command_buffer.handle])
                        .wait_semaphores(&semaphore_handles)
                        .wait_dst_stage_mask(wait_stages)
                        .signal_semaphores(&semaphore_handles)
                        .push_next(
                            &mut vk::TimelineSemaphoreSubmitInfo::builder()
                                .wait_semaphore_values(wait_values)
                                .signal_semaphore_values(signal_values)
                                .build(),
                        )
                        .build()],
                    fence.handle,
                )
                .unwrap();

            let in_use = Arc::new(std::sync::atomic::AtomicBool::new(true));
            let in_use_signaler = in_use.clone();

            self.command_buffers
                .insert(command_buffer.handle, (in_use, command_buffer));

            tokio::task::spawn(async move {
                fence.wait();
                in_use_signaler.store(false, std::sync::atomic::Ordering::SeqCst);
            });
        }
    }

    pub fn present(&self, swapchain: &Swapchain, index: u32, wait_semaphore: &[&BinarySemaphore]) {
        let wait_handles = wait_semaphore.iter().map(|s| s.handle).collect::<Vec<_>>();

        let info = vk::PresentInfoKHR::builder()
            .swapchains(&[swapchain.vk_handle()])
            .wait_semaphores(wait_handles.as_slice())
            .image_indices(&[index])
            .build();
        unsafe {
            if let Err(e) = self
                .device
                .swapchain_loader
                .queue_present(self.handle, &info)
            {
                log::warn!("{:?}", e);
            }
        }
    }
}

pub struct Fence {
    handle: vk::Fence,
    device: Arc<Device>,
}

impl Fence {
    pub fn new(device: Arc<Device>, signaled: bool) -> Self {
        let handle = unsafe {
            device.handle.create_fence(
                &vk::FenceCreateInfo::builder()
                    .flags(match signaled {
                        true => vk::FenceCreateFlags::SIGNALED,
                        false => vk::FenceCreateFlags::empty(),
                    })
                    .build(),
                None,
            )
        }
        .unwrap();
        Self { handle, device }
    }

    pub fn wait(&self) {
        unsafe {
            self.device
                .handle
                .wait_for_fences(&[self.handle], true, std::u64::MAX)
                .unwrap();
        }
    }

    pub fn reset(&self) {
        unsafe {
            self.device.handle.reset_fences(&[self.handle]).unwrap();
        }
    }
}

impl Drop for Fence {
    fn drop(&mut self) {
        unsafe { self.device.handle.destroy_fence(self.handle, None) };
    }
}

pub struct TimelineSemaphore {
    handle: vk::Semaphore,
    device: Arc<Device>,
}

impl TimelineSemaphore {
    pub fn new(device: Arc<Device>) -> Self {
        unsafe {
            let handle = device
                .handle
                .create_semaphore(
                    &vk::SemaphoreCreateInfo::builder()
                        .push_next(
                            &mut vk::SemaphoreTypeCreateInfo::builder()
                                .semaphore_type(vk::SemaphoreType::TIMELINE)
                                .initial_value(0)
                                .build(),
                        )
                        .build(),
                    None,
                )
                .unwrap();
            Self { handle, device }
        }
    }

    pub fn wait_for(&self, value: u64) {
        unsafe {
            self.device
                .handle
                .wait_semaphores(
                    &vk::SemaphoreWaitInfo::builder()
                        .semaphores(&[self.handle])
                        .values(&[value])
                        .build(),
                    std::u64::MAX,
                )
                .unwrap();
        }
    }

    pub fn signal(&self, value: u64) {
        unsafe {
            self.device
                .handle
                .signal_semaphore(
                    &vk::SemaphoreSignalInfo::builder()
                        .semaphore(self.handle)
                        .value(value)
                        .build(),
                )
                .unwrap();
        }
    }
}

impl Drop for TimelineSemaphore {
    fn drop(&mut self) {
        unsafe {
            self.device.handle.destroy_semaphore(self.handle, None);
        }
    }
}

pub struct BinarySemaphore {
    handle: vk::Semaphore,
    device: Arc<Device>,
}

impl BinarySemaphore {
    pub fn new(device: Arc<Device>) -> Self {
        unsafe {
            let handle = device
                .handle
                .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
                .unwrap();
            Self { handle, device }
        }
    }
}

impl Drop for BinarySemaphore {
    fn drop(&mut self) {
        unsafe {
            self.device.handle.destroy_semaphore(self.handle, None);
        }
    }
}

pub struct CommandPool {
    handle: vk::CommandPool,
    device: Arc<Device>,
}

impl CommandPool {
    pub fn new(device: Arc<Device>) -> Self {
        unsafe {
            let handle = device
                .handle
                .create_command_pool(
                    &vk::CommandPoolCreateInfo::builder()
                        .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
                        .queue_family_index(device.pdevice.queue_family_index)
                        .build(),
                    None,
                )
                .unwrap();

            Self { handle, device }
        }
    }
}

impl Drop for CommandPool {
    fn drop(&mut self) {
        unsafe {
            self.device.handle.destroy_command_pool(self.handle, None);
        }
    }
}

pub trait GraphicsPipelineRecorder: PipelineRecorder {
    fn bind_index_buffer(&mut self, buffer: Arc<Buffer>, offset: u64, index_type: vk::IndexType);
    fn set_scissor(&self, scissors: &[vk::Rect2D]);
    fn set_viewport(&self, viewport: vk::Viewport);
    fn bind_vertex_buffer(&mut self, buffers: Vec<Arc<Buffer>>, offsets: &[u64]);
    fn draw_indexed(&self, index_count: u32, instance_count: u32);
    fn draw(&self, vertex_count: u32, instance_count: u32);
}

pub trait ComputePipelineRecorder: PipelineRecorder {
    fn dispatch(&self, group_count_x: u32, group_count_y: u32, group_count_z: u32);
}

pub trait RayTracingPipelineRecorder: PipelineRecorder {
    fn trace_ray(
        &self,
        raygen_shader_binding_table: &vk::StridedDeviceAddressRegionKHR,
        miss_shader_binding_table: &vk::StridedDeviceAddressRegionKHR,
        hit_shader_binding_table: &vk::StridedDeviceAddressRegionKHR,
        callable_shader_binding_table: &vk::StridedDeviceAddressRegionKHR,
        width: u32,
        height: u32,
        depth: u32,
    );
}

pub trait PipelineRecorder {
    fn bind_descriptor_sets(
        &mut self,
        descriptor_sets: Vec<Arc<DescriptorSet>>,
        layout: &PipelineLayout,
        first_set: u32,
    );
    fn push_constants(
        &mut self,
        layout: &PipelineLayout,
        stage_flags: vk::ShaderStageFlags,
        offset: u32,
        constants: &[u8],
    );
}

pub trait GeneralRecorder {}

impl<'a> PipelineRecorder for CommandRecorder<'a> {
    fn bind_descriptor_sets(
        &mut self,
        descriptor_sets: Vec<Arc<DescriptorSet>>,
        layout: &PipelineLayout,
        first_set: u32,
    ) {
        unsafe {
            let descriptor_set_handles = descriptor_sets
                .iter()
                .map(|set| set.handle)
                .collect::<Vec<_>>();
            self.device().handle.cmd_bind_descriptor_sets(
                self.command_buffer.handle,
                self.bind_point.unwrap(),
                layout.handle,
                first_set,
                descriptor_set_handles.as_slice(),
                &[],
            );
        }

        descriptor_sets
            .into_iter()
            .for_each(|set| self.command_buffer.resources.push(set));
    }
    fn push_constants(
        &mut self,
        layout: &PipelineLayout,
        stage_flags: vk::ShaderStageFlags,
        offset: u32,
        constants: &[u8],
    ) {
        unsafe {
            self.device().handle.cmd_push_constants(
                self.command_buffer.handle,
                layout.handle,
                stage_flags,
                offset,
                constants,
            )
        }
    }
}

impl<'a> RayTracingPipelineRecorder for CommandRecorder<'a> {
    fn trace_ray(
        &self,
        raygen_shader_binding_table: &vk::StridedDeviceAddressRegionKHR,
        miss_shader_binding_table: &vk::StridedDeviceAddressRegionKHR,
        hit_shader_binding_table: &vk::StridedDeviceAddressRegionKHR,
        callable_shader_binding_table: &vk::StridedDeviceAddressRegionKHR,
        width: u32,
        height: u32,
        depth: u32,
    ) {
        unsafe {
            self.device().ray_tracing_pipeline_loader.cmd_trace_rays(
                self.command_buffer.handle,
                raygen_shader_binding_table,
                miss_shader_binding_table,
                hit_shader_binding_table,
                callable_shader_binding_table,
                width,
                height,
                depth,
            );
        }
    }
}

impl<'a> ComputePipelineRecorder for CommandRecorder<'a> {
    fn dispatch(&self, group_count_x: u32, group_count_y: u32, group_count_z: u32) {
        unsafe {
            self.device().handle.cmd_dispatch(
                self.command_buffer.handle,
                group_count_x,
                group_count_y,
                group_count_z,
            );
        }
    }
}

impl<'a> GraphicsPipelineRecorder for CommandRecorder<'a> {
    fn bind_index_buffer(&mut self, buffer: Arc<Buffer>, offset: u64, index_type: vk::IndexType) {
        unsafe {
            self.command_buffer
                .pool
                .device
                .handle
                .cmd_bind_index_buffer(
                    self.command_buffer.handle,
                    buffer.handle,
                    offset,
                    index_type,
                );
        }
        self.command_buffer.resources.push(buffer);
    }

    fn set_scissor(&self, scissors: &[vk::Rect2D]) {
        unsafe {
            self.device()
                .handle
                .cmd_set_scissor(self.command_buffer.handle, 0, scissors);
        }
    }

    fn bind_vertex_buffer(&mut self, buffers: Vec<Arc<Buffer>>, offsets: &[u64]) {
        let buffer_handles = buffers.iter().map(|b| b.handle).collect::<Vec<_>>();
        unsafe {
            self.device().handle.cmd_bind_vertex_buffers(
                self.command_buffer.handle,
                0,
                buffer_handles.as_slice(),
                offsets,
            );
        }
        buffers
            .into_iter()
            .for_each(|b| self.command_buffer.resources.push(b));
    }

    fn draw_indexed(&self, index_count: u32, instance_count: u32) {
        unsafe {
            self.device().handle.cmd_draw_indexed(
                self.command_buffer.handle,
                index_count,
                instance_count,
                0,
                0,
                0,
            );
        }
    }

    fn set_viewport(&self, viewport: vk::Viewport) {
        unsafe {
            self.device()
                .handle
                .cmd_set_viewport(self.command_buffer.handle, 0, &[viewport]);
        }
    }

    fn draw(&self, vertex_count: u32, instance_count: u32) {
        unsafe {
            self.device().handle.cmd_draw(
                self.command_buffer.handle,
                vertex_count,
                instance_count,
                0,
                0,
            );
        }
    }
}

pub struct CommandRecorder<'a> {
    command_buffer: &'a mut CommandBuffer,
    bind_point: Option<vk::PipelineBindPoint>,
}

impl<'a> CommandRecorder<'a> {
    pub fn update_buffer(&mut self, buffer: Arc<Buffer>, offset: u64, data: &[u8]) {
        unsafe {
            self.device().handle.cmd_update_buffer(
                self.command_buffer.handle,
                buffer.handle,
                offset,
                data,
            );
        }
        self.command_buffer.resources.push(buffer);
    }
    pub fn copy_buffer(&mut self, src: Arc<Buffer>, dst: Arc<Buffer>, region: &[vk::BufferCopy]) {
        unsafe {
            self.copy_buffer_raw(src.as_ref(), dst.as_ref(), region);
        }
        self.command_buffer.resources.push(src);
        self.command_buffer.resources.push(dst);
    }

    unsafe fn copy_buffer_raw(&mut self, src: &Buffer, dst: &Buffer, region: &[vk::BufferCopy]) {
        unsafe {
            self.device().handle.cmd_copy_buffer(
                self.command_buffer.handle,
                src.handle,
                dst.handle,
                region,
            );
        }
    }

    pub fn begin_render_pass<I>(
        &mut self,
        render_pass: Arc<RenderPass>,
        framebuffer: Arc<Framebuffer>,
        f: I,
    ) where
        I: FnOnce(&mut CommandRecorder),
    {
        unsafe {
            let info = vk::RenderPassBeginInfo::builder()
                .render_pass(render_pass.handle)
                .framebuffer(framebuffer.handle)
                .render_area(
                    vk::Rect2D::builder()
                        .extent(vk::Extent2D {
                            width: framebuffer.width,
                            height: framebuffer.height,
                        })
                        .build(),
                )
                .build();
            self.device().handle.cmd_begin_render_pass(
                self.command_buffer.handle,
                &info,
                vk::SubpassContents::INLINE,
            );

            f(self);

            self.device()
                .handle
                .cmd_end_render_pass(self.command_buffer.handle);
            self.command_buffer.resources.push(render_pass);
            self.command_buffer.resources.push(framebuffer);
        }
    }

    pub fn bind_graphics_pipeline<I>(&mut self, pipeline: Arc<GraphicsPipeline>, f: I)
    where
        I: FnOnce(&mut dyn GraphicsPipelineRecorder, &dyn Pipeline),
    {
        unsafe {
            self.device().handle.cmd_bind_pipeline(
                self.command_buffer.handle,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.handle,
            );
            self.bind_point = Some(vk::PipelineBindPoint::GRAPHICS);
            f(self, pipeline.as_ref());
        }
        self.command_buffer.resources.push(pipeline);
    }

    pub fn bind_compute_pipeline<I>(&mut self, pipeline: Arc<ComputePipeline>, f: I)
    where
        I: FnOnce(&mut dyn ComputePipelineRecorder, &dyn Pipeline),
    {
        unsafe {
            self.device().handle.cmd_bind_pipeline(
                self.command_buffer.handle,
                vk::PipelineBindPoint::COMPUTE,
                pipeline.handle,
            );
            self.bind_point = Some(vk::PipelineBindPoint::COMPUTE);
            f(self, pipeline.as_ref());
        }
        self.command_buffer.resources.push(pipeline);
    }

    pub fn bind_ray_tracing_pipeline<I>(&mut self, pipeline: Arc<RayTracingPipeline>, f: I)
    where
        I: FnOnce(&mut dyn RayTracingPipelineRecorder, &dyn Pipeline),
    {
        unsafe {
            self.device().handle.cmd_bind_pipeline(
                self.command_buffer.handle,
                vk::PipelineBindPoint::RAY_TRACING_KHR,
                pipeline.handle,
            );
            self.bind_point = Some(vk::PipelineBindPoint::RAY_TRACING_KHR);
            f(self, pipeline.as_ref());
        }
        self.command_buffer.resources.push(pipeline);
    }

    fn device(&self) -> &Device {
        &self.command_buffer.pool.device
    }

    pub fn copy_buffer_to_image(
        &mut self,
        src: Arc<Buffer>,
        dst: Arc<Image>,
        regions: &[vk::BufferImageCopy],
    ) {
        unsafe {
            self.device().handle.cmd_copy_buffer_to_image(
                self.command_buffer.handle,
                src.handle,
                dst.handle,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                regions,
            );
        }
    }

    unsafe fn copy_buffer_to_image_raw(
        &mut self,
        src: &Buffer,
        dst: &Image,
        regions: &[vk::BufferImageCopy],
    ) {
        self.device().handle.cmd_copy_buffer_to_image(
            self.command_buffer.handle,
            src.handle,
            dst.handle,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            regions,
        );
    }

    pub fn blit_image(
        &mut self,
        src: Arc<Image>,
        dst: Arc<Image>,
        regions: &[vk::ImageBlit],
        filter: vk::Filter,
    ) {
        unsafe {
            self.device().handle.cmd_blit_image(
                self.command_buffer.handle,
                src.handle,
                src.layout(),
                dst.handle,
                dst.layout(),
                regions,
                filter,
            );
        }
        self.command_buffer.resources.push(src);
        self.command_buffer.resources.push(dst);
    }

    pub fn set_image_layout(
        &mut self,
        image: Arc<Image>,
        old_layout: Option<vk::ImageLayout>,
        new_layout: vk::ImageLayout,
    ) {
        let old = match old_layout {
            Some(l) => l,
            None => {
                vk::ImageLayout::from_raw(image.layout.load(std::sync::atomic::Ordering::SeqCst))
            }
        };
        cmd_set_image_layout(old, &self.command_buffer, image.handle, new_layout);
        image
            .layout
            .store(new_layout.as_raw(), std::sync::atomic::Ordering::SeqCst);
        self.command_buffer.resources.push(image);
    }

    unsafe fn set_image_layout_raw(&mut self, image: &Image, new_layout: vk::ImageLayout) {
        cmd_set_image_layout(
            vk::ImageLayout::from_raw(image.layout.load(std::sync::atomic::Ordering::SeqCst)),
            &self.command_buffer,
            image.handle,
            new_layout,
        );
    }

    fn build_acceleration_structure_raw(
        &mut self,
        info: vk::AccelerationStructureBuildGeometryInfoKHR,
        build_range_infos: &[vk::AccelerationStructureBuildRangeInfoKHR],
    ) {
        unsafe {
            self.device()
                .acceleration_structure_loader
                .cmd_build_acceleration_structures(
                    self.command_buffer.handle,
                    &[info],
                    &[build_range_infos],
                );
        }
    }
}

trait Resource {}

impl Resource for Buffer {}
impl Resource for Image {}
impl Resource for Sampler {}
impl Resource for ImageView {}
impl Resource for RenderPass {}
impl Resource for Framebuffer {}
impl Resource for GraphicsPipeline {}
impl Resource for ComputePipeline {}
impl Resource for RayTracingPipeline {}
impl Resource for DescriptorSet {}
impl Resource for PipelineLayout {}
impl Resource for AccelerationStructure {}

pub struct CommandBuffer {
    handle: vk::CommandBuffer,
    pool: Arc<CommandPool>,
    in_use: bool,
    resources: Vec<Arc<dyn Resource>>,
}
impl !Send for CommandBuffer {}
impl !Sync for CommandBuffer {}

impl PartialEq for CommandBuffer {
    fn eq(&self, other: &Self) -> bool {
        self.handle == other.handle
    }
}

impl Eq for CommandBuffer {}

impl core::hash::Hash for CommandBuffer {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u64(self.handle.as_raw());
    }
}

impl CommandBuffer {
    pub fn new(pool: Arc<CommandPool>) -> Self {
        unsafe {
            let device = &pool.device.handle;
            let handle = device
                .allocate_command_buffers(
                    &vk::CommandBufferAllocateInfo::builder()
                        .command_pool(pool.handle)
                        .command_buffer_count(1)
                        .level(vk::CommandBufferLevel::PRIMARY)
                        .build(),
                )
                .unwrap()
                .first()
                .unwrap()
                .to_owned();

            Self {
                handle,
                pool,
                in_use: false,
                resources: Vec::new(),
            }
        }
    }

    pub fn encode<F>(&mut self, func: F)
    where
        F: FnOnce(&mut CommandRecorder),
    {
        unsafe {
            let device = self.pool.device.handle.clone();
            device
                .begin_command_buffer(self.handle, &vk::CommandBufferBeginInfo::default())
                .unwrap();
            let mut manager = CommandRecorder {
                command_buffer: self,
                bind_point: None,
            };
            func(&mut manager);
            device.end_command_buffer(self.handle).unwrap();
        }
    }

    fn free_resources(&mut self) {
        self.resources.clear();
    }
}

impl Drop for CommandBuffer {
    fn drop(&mut self) {
        unsafe {
            if !self.in_use {
                self.pool
                    .device
                    .handle
                    .free_command_buffers(self.pool.handle, &[self.handle]);
            } else {
                panic!("don't");
            }
        }
    }
}

pub struct Swapchain {
    handle: std::sync::atomic::AtomicU64,
    device: Arc<Device>,
    surface: Arc<Surface>,
    width: std::sync::atomic::AtomicU32,
    height: std::sync::atomic::AtomicU32,
    format: vk::Format,
    image_available_semaphore: BinarySemaphore,
    present_mode: vk::PresentModeKHR,
}

impl Swapchain {
    pub fn new(
        device: Arc<Device>,
        surface: Arc<Surface>,
        present_mode: vk::PresentModeKHR,
    ) -> Self {
        unsafe {
            let surface_loader = &device.pdevice.instance.surface_loader;
            let surface_capabilities = surface_loader
                .get_physical_device_surface_capabilities(device.pdevice.handle, surface.handle)
                .unwrap();

            let surface_format = surface_loader
                .get_physical_device_surface_formats(device.pdevice.handle, surface.handle)
                .unwrap()[0];

            let format = surface_format.format;

            let swapchain_create_info = vk::SwapchainCreateInfoKHR::builder()
                .surface(surface.handle)
                .min_image_count(2)
                .image_color_space(surface_format.color_space)
                .image_format(format)
                .image_extent(surface_capabilities.current_extent)
                .image_usage(
                    vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_DST,
                )
                .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                .pre_transform(vk::SurfaceTransformFlagsKHR::IDENTITY)
                .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                .present_mode(present_mode)
                .clipped(true)
                .image_array_layers(1);
            let handle = device
                .swapchain_loader
                .create_swapchain(&swapchain_create_info, None)
                .unwrap()
                .as_raw();
            let image_available_semaphore = BinarySemaphore::new(device.clone());

            Self {
                handle: std::sync::atomic::AtomicU64::new(handle),
                device,
                surface,
                width: std::sync::atomic::AtomicU32::new(surface_capabilities.current_extent.width),
                height: std::sync::atomic::AtomicU32::new(
                    surface_capabilities.current_extent.height,
                ),
                format,
                image_available_semaphore,
                present_mode,
            }
        }
    }

    pub fn acquire_next_image(&self) -> (u32, bool) {
        unsafe {
            let (index, sub) = self
                .device
                .swapchain_loader
                .acquire_next_image(
                    vk::SwapchainKHR::from_raw(
                        self.handle.load(std::sync::atomic::Ordering::SeqCst),
                    ),
                    0,
                    self.image_available_semaphore.handle,
                    vk::Fence::null(),
                )
                .unwrap();
            (index, sub)
        }
    }

    pub fn renew(&self) {
        let swapchain_loader = &self.device.swapchain_loader;
        let surface_loader = &self.device.pdevice.instance.surface_loader;
        let pdevice = &self.device.pdevice;
        unsafe {
            let surface_capabilities = surface_loader
                .get_physical_device_surface_capabilities(pdevice.handle, self.surface.handle)
                .unwrap();

            let surface_format = surface_loader
                .get_physical_device_surface_formats(pdevice.handle, self.surface.handle)
                .unwrap()[0];

            let old_swapchain = self.vk_handle();
            let swapchain_create_info = vk::SwapchainCreateInfoKHR::builder()
                .surface(self.surface.handle)
                .min_image_count(2)
                .image_color_space(surface_format.color_space)
                .image_format(surface_format.format)
                .image_extent(surface_capabilities.current_extent)
                .image_usage(
                    vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_DST,
                )
                .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                .pre_transform(vk::SurfaceTransformFlagsKHR::IDENTITY)
                .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                .present_mode(self.present_mode)
                .clipped(true)
                .image_array_layers(1)
                .old_swapchain(old_swapchain);

            self.handle.store(
                swapchain_loader
                    .create_swapchain(&swapchain_create_info, None)
                    .unwrap()
                    .as_raw(),
                std::sync::atomic::Ordering::SeqCst,
            );
            self.device
                .swapchain_loader
                .destroy_swapchain(old_swapchain, None);
            self.width.store(
                surface_capabilities.current_extent.width,
                std::sync::atomic::Ordering::SeqCst,
            );
            self.height.store(
                surface_capabilities.current_extent.height,
                std::sync::atomic::Ordering::SeqCst,
            );
        }
    }

    pub fn image_available_semaphore(&self) -> &BinarySemaphore {
        &self.image_available_semaphore
    }

    pub fn vk_handle(&self) -> vk::SwapchainKHR {
        vk::SwapchainKHR::from_raw(self.handle.load(std::sync::atomic::Ordering::SeqCst))
    }

    pub fn width(&self) -> u32 {
        self.width.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn height(&self) -> u32 {
        self.height.load(std::sync::atomic::Ordering::SeqCst)
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        unsafe {
            self.device.swapchain_loader.destroy_swapchain(
                vk::SwapchainKHR::from_raw(self.handle.load(std::sync::atomic::Ordering::SeqCst)),
                None,
            )
        }
    }
}

enum ImageType {
    Allocated {
        allocator: Arc<Allocator>,
        allocation: vk_mem::Allocation,
        allocation_info: vk_mem::AllocationInfo,
    },
    Swapchain {
        swapchain: Arc<Swapchain>,
    },
}

pub struct Image {
    handle: vk::Image,
    image_type: ImageType,
    width: u32,
    height: u32,
    layout: std::sync::atomic::AtomicI32,
    format: vk::Format,
}

impl Image {
    pub fn new(
        name: Option<&str>,
        allocator: Arc<Allocator>,
        format: vk::Format,
        width: u32,
        height: u32,
        tiling: vk::ImageTiling,
        image_usage: vk::ImageUsageFlags,
        memory_usage: vk_mem::MemoryUsage,
    ) -> Self {
        let (handle, allocation, allocation_info) = allocator
            .handle
            .create_image(
                &vk::ImageCreateInfo::builder()
                    .image_type(vk::ImageType::TYPE_2D)
                    .format(format)
                    .extent(vk::Extent3D {
                        width,
                        height,
                        depth: 1,
                    })
                    .samples(vk::SampleCountFlags::TYPE_1)
                    .mip_levels(1)
                    .array_layers(1)
                    .tiling(tiling)
                    .usage(image_usage)
                    .sharing_mode(vk::SharingMode::EXCLUSIVE)
                    .initial_layout(vk::ImageLayout::UNDEFINED)
                    .build(),
                &vk_mem::AllocationCreateInfo {
                    usage: memory_usage,
                    ..Default::default()
                },
            )
            .unwrap();

        let device = allocator.device();
        unsafe {
            if let Some(name) = name {
                device
                    .pdevice
                    .instance
                    .debug_utils_loader
                    .debug_utils_set_object_name(
                        device.handle.handle(),
                        &vk::DebugUtilsObjectNameInfoEXT::builder()
                            .object_handle(handle.as_raw())
                            .object_type(vk::ObjectType::IMAGE)
                            .object_name(CString::new(name).unwrap().as_ref())
                            .build(),
                    )
                    .unwrap();
            }
        }

        let image_type = ImageType::Allocated {
            allocator,
            allocation,
            allocation_info,
        };

        let layout = std::sync::atomic::AtomicI32::new(vk::ImageLayout::UNDEFINED.as_raw());

        Self {
            handle,
            width,
            height,
            layout,
            image_type,
            format,
        }
    }

    pub fn layout(&self) -> vk::ImageLayout {
        vk::ImageLayout::from_raw(self.layout.load(std::sync::atomic::Ordering::SeqCst))
    }

    pub fn new_init_host<I: AsRef<[u8]>>(
        name: Option<&str>,
        allocator: Arc<Allocator>,
        format: vk::Format,
        width: u32,
        height: u32,
        tiling: vk::ImageTiling,
        image_usage: vk::ImageUsageFlags,
        memory_usage: vk_mem::MemoryUsage,
        queue: &mut Queue,
        command_pool: Arc<CommandPool>,
        data: I,
    ) -> Self {
        let mut image = Self::new(
            name,
            allocator.clone(),
            format,
            width,
            height,
            tiling,
            image_usage,
            memory_usage,
        );
        let data = data.as_ref();

        let staging_buffer = Buffer::new_init_host(
            Some("staging buffer"),
            allocator,
            vk::BufferUsageFlags::TRANSFER_SRC,
            MemoryUsage::CpuToGpu,
            data,
        );

        image.copy_from_buffer(&staging_buffer, queue, command_pool);

        image
    }

    pub fn copy_from_buffer(
        &self,
        buffer: &Buffer,
        queue: &mut Queue,
        command_pool: Arc<CommandPool>,
    ) {
        let mut command_buffer = CommandBuffer::new(command_pool);

        unsafe {
            command_buffer.encode(|recorder| {
                recorder.set_image_layout_raw(self, vk::ImageLayout::TRANSFER_DST_OPTIMAL);
                recorder.copy_buffer_to_image_raw(
                    buffer,
                    self,
                    &[vk::BufferImageCopy::builder()
                        .image_extent(vk::Extent3D {
                            width: self.width,
                            height: self.height,
                            depth: 1,
                        })
                        .image_offset(vk::Offset3D::default())
                        .image_subresource(
                            vk::ImageSubresourceLayers::builder()
                                .layer_count(1)
                                .base_array_layer(0)
                                .aspect_mask(vk::ImageAspectFlags::COLOR)
                                .mip_level(0)
                                .build(),
                        )
                        .buffer_offset(0)
                        .buffer_image_height(0)
                        .buffer_row_length(0)
                        .build()],
                );
            });
        }
        self.layout.store(
            vk::ImageLayout::TRANSFER_DST_OPTIMAL.as_raw(),
            std::sync::atomic::Ordering::SeqCst,
        );

        let semaphore = TimelineSemaphore::new(self.device().clone());
        queue.submit_timeline(
            command_buffer,
            &[&semaphore],
            &[0],
            &[vk::PipelineStageFlags::ALL_COMMANDS],
            &[1],
        );
        semaphore.wait_for(1);
    }

    pub fn set_layout(
        &mut self,
        layout: vk::ImageLayout,
        queue: &mut Queue,
        command_pool: Arc<CommandPool>,
    ) {
        let mut command_buffer = CommandBuffer::new(command_pool);
        unsafe {
            command_buffer.encode(|recorder| {
                recorder.set_image_layout_raw(self, layout);
            });
        }
        self.layout
            .store(layout.as_raw(), std::sync::atomic::Ordering::SeqCst);

        let semaphore = TimelineSemaphore::new(self.device().clone());
        queue.submit_timeline(
            command_buffer,
            &[&semaphore],
            &[0],
            &[vk::PipelineStageFlags::ALL_COMMANDS],
            &[1],
        );
        semaphore.wait_for(1);
    }

    pub fn from_swapchain(swapchain: Arc<Swapchain>) -> Vec<Self> {
        unsafe {
            let device = swapchain.device.as_ref();
            let images = device
                .swapchain_loader
                .get_swapchain_images(swapchain.vk_handle())
                .unwrap();

            let results = images
                .into_iter()
                .map(|handle| {
                    Self {
                        handle,
                        image_type: ImageType::Swapchain {
                            swapchain: swapchain.clone(),
                        },
                        width: swapchain.width(),
                        height: swapchain.height(),
                        layout: std::sync::atomic::AtomicI32::new(
                            vk::ImageLayout::UNDEFINED.as_raw(),
                        ),
                        format: swapchain.format,
                    }
                })
                .collect::<Vec<_>>();
            results.iter().for_each(|image| {
                device
                    .pdevice
                    .instance
                    .debug_utils_loader
                    .debug_utils_set_object_name(
                        device.handle.handle(),
                        &vk::DebugUtilsObjectNameInfoEXT::builder()
                            .object_handle(image.handle.as_raw())
                            .object_type(vk::ObjectType::IMAGE)
                            .object_name(CString::new("swapchain image").unwrap().as_ref())
                            .build(),
                    )
                    .unwrap();
            });

            results
        }
    }

    fn device(&self) -> &Arc<Device> {
        let device = match self.image_type.borrow() {
            ImageType::Allocated { allocator, .. } => &allocator.device,
            ImageType::Swapchain { swapchain } => &swapchain.device,
        };
        device
    }

    pub fn cmd_set_layout(
        &mut self,
        command_buffer: &CommandBuffer,
        layout: vk::ImageLayout,
        need_old_data: bool,
    ) {
        let old_layout = match need_old_data {
            true => {
                vk::ImageLayout::from_raw(self.layout.load(std::sync::atomic::Ordering::SeqCst))
            }
            false => vk::ImageLayout::UNDEFINED,
        };
        cmd_set_image_layout(old_layout, command_buffer, self.handle, layout);
        self.layout
            .store(layout.as_raw(), std::sync::atomic::Ordering::SeqCst);
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        match &self.image_type {
            ImageType::Allocated {
                allocator,
                allocation,
                ..
            } => {
                allocator.handle.destroy_image(self.handle, &allocation);
            }
            ImageType::Swapchain { .. } => {}
        }
    }
}

pub struct ImageView {
    handle: vk::ImageView,
    image: Arc<Image>,
}

impl ImageView {
    pub fn new(image: Arc<Image>) -> Self {
        unsafe {
            let device = match &image.image_type {
                ImageType::Allocated { allocator, .. } => &allocator.device,
                ImageType::Swapchain { swapchain } => &swapchain.device,
            };
            let handle = device
                .handle
                .create_image_view(
                    &vk::ImageViewCreateInfo::builder()
                        .components(
                            vk::ComponentMapping::builder()
                                .r(vk::ComponentSwizzle::IDENTITY)
                                .g(vk::ComponentSwizzle::IDENTITY)
                                .b(vk::ComponentSwizzle::IDENTITY)
                                .a(vk::ComponentSwizzle::IDENTITY)
                                .build(),
                        )
                        .view_type(vk::ImageViewType::TYPE_2D)
                        .format(image.format)
                        .subresource_range(
                            vk::ImageSubresourceRange::builder()
                                .aspect_mask(vk::ImageAspectFlags::COLOR)
                                .base_mip_level(0)
                                .level_count(1)
                                .base_array_layer(0)
                                .layer_count(1)
                                .build(),
                        )
                        .image(image.handle)
                        .build(),
                    None,
                )
                .unwrap();
            Self { image, handle }
        }
    }

    pub fn image(&self) -> &Image {
        self.image.as_ref()
    }
}

impl Drop for ImageView {
    fn drop(&mut self) {
        unsafe {
            let device = match &self.image.image_type {
                ImageType::Allocated { allocator, .. } => &allocator.device,
                ImageType::Swapchain { swapchain } => &swapchain.device,
            };
            device.handle.destroy_image_view(self.handle, None);
        }
    }
}

fn cmd_set_image_layout(
    old_layout: vk::ImageLayout,
    command_buffer: &CommandBuffer,
    image: vk::Image,
    new_layout: vk::ImageLayout,
) {
    use vk::AccessFlags;
    use vk::ImageLayout;

    let device = &command_buffer.pool.device.handle;
    unsafe {
        let src_access_mask = match old_layout {
            ImageLayout::UNDEFINED => AccessFlags::default(),
            ImageLayout::GENERAL => AccessFlags::default(),
            ImageLayout::COLOR_ATTACHMENT_OPTIMAL => AccessFlags::COLOR_ATTACHMENT_WRITE,
            ImageLayout::TRANSFER_DST_OPTIMAL => AccessFlags::TRANSFER_WRITE,
            ImageLayout::TRANSFER_SRC_OPTIMAL => AccessFlags::TRANSFER_READ,
            ImageLayout::PRESENT_SRC_KHR => AccessFlags::COLOR_ATTACHMENT_READ,
            _ => {
                unimplemented!("unknown old layout {:?}", old_layout);
            }
        };
        let dst_access_mask = match new_layout {
            ImageLayout::COLOR_ATTACHMENT_OPTIMAL => AccessFlags::COLOR_ATTACHMENT_WRITE,
            ImageLayout::GENERAL => AccessFlags::default(),
            ImageLayout::TRANSFER_SRC_OPTIMAL => AccessFlags::TRANSFER_READ,
            ImageLayout::TRANSFER_DST_OPTIMAL => AccessFlags::TRANSFER_WRITE,
            ImageLayout::PRESENT_SRC_KHR => AccessFlags::COLOR_ATTACHMENT_READ,
            ImageLayout::SHADER_READ_ONLY_OPTIMAL => AccessFlags::SHADER_READ,
            _ => {
                unimplemented!("unknown new layout {:?}", new_layout);
            }
        };
        device.cmd_pipeline_barrier(
            command_buffer.handle,
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[vk::ImageMemoryBarrier::builder()
                .image(image)
                .old_layout(old_layout)
                .new_layout(new_layout)
                .src_access_mask(src_access_mask)
                .dst_access_mask(dst_access_mask)
                .subresource_range(
                    vk::ImageSubresourceRange::builder()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .base_mip_level(0)
                        .level_count(1)
                        .base_array_layer(0)
                        .layer_count(1)
                        .build(),
                )
                .build()],
        );
    }
}

pub struct Framebuffer {
    handle: vk::Framebuffer,
    render_pass: Arc<RenderPass>,
    attachments: Vec<Arc<ImageView>>,
    width: u32,
    height: u32,
}

impl Framebuffer {
    pub fn new(
        render_pass: Arc<RenderPass>,
        width: u32,
        height: u32,
        attachments: Vec<Arc<ImageView>>,
    ) -> Self {
        unsafe {
            let device = &render_pass.device;
            let attachment_handles = attachments
                .iter()
                .map(|view| view.handle)
                .collect::<Vec<_>>();
            let handle = device
                .handle
                .create_framebuffer(
                    &vk::FramebufferCreateInfo::builder()
                        .width(width)
                        .height(height)
                        .layers(1)
                        .attachments(attachment_handles.as_slice())
                        .render_pass(render_pass.handle)
                        .build(),
                    None,
                )
                .unwrap();
            Self {
                handle,
                render_pass,
                attachments,
                width,
                height,
            }
        }
    }

    pub fn handle(&self) -> vk::Framebuffer {
        self.handle
    }
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        let device = &self.render_pass.device;
        unsafe {
            device.handle.destroy_framebuffer(self.handle, None);
        }
    }
}

pub struct RenderPass {
    handle: vk::RenderPass,
    device: Arc<Device>,
}

impl RenderPass {
    pub fn new(device: Arc<Device>, info: &vk::RenderPassCreateInfo) -> Self {
        unsafe {
            let handle = device.handle.create_render_pass(&info, None).unwrap();
            Self { handle, device }
        }
    }

    pub fn handle(&self) -> vk::RenderPass {
        self.handle
    }
}

impl Drop for RenderPass {
    fn drop(&mut self) {
        unsafe {
            self.device.handle.destroy_render_pass(self.handle, None);
        }
    }
}

#[derive(Clone)]
pub enum DescriptorType {
    Sampler(Option<Arc<Sampler>>),
    SampledImage,
    UniformBuffer,
    StorageBuffer,
    AccelerationStructure,
    StorageImage,
}

#[derive(Clone)]
pub struct DescriptorSetLayoutBinding {
    pub binding: u32,
    pub descriptor_type: DescriptorType,
    pub stage_flags: vk::ShaderStageFlags,
}

pub struct DescriptorSetLayout {
    handle: vk::DescriptorSetLayout,
    device: Arc<Device>,
    bindings: Vec<DescriptorSetLayoutBinding>,
    vk_bindings: Vec<vk::DescriptorSetLayoutBinding>,
}

impl DescriptorSetLayout {
    pub fn new(
        device: Arc<Device>,
        name: Option<&str>,
        bindings: &[DescriptorSetLayoutBinding],
    ) -> Self {
        let vk_bindings = bindings
            .iter()
            .map(|binding| {
                match &binding.descriptor_type {
                    DescriptorType::Sampler(immutable_sampler) => {
                        if let Some(sampler) = immutable_sampler {
                            vk::DescriptorSetLayoutBinding::builder()
                                .binding(binding.binding)
                                .descriptor_type(vk::DescriptorType::SAMPLER)
                                .descriptor_count(1)
                                .immutable_samplers(&[sampler.handle])
                                .stage_flags(binding.stage_flags)
                                .build()
                        } else {
                            vk::DescriptorSetLayoutBinding::builder()
                                .binding(binding.binding)
                                .descriptor_type(vk::DescriptorType::SAMPLER)
                                .descriptor_count(1)
                                .stage_flags(binding.stage_flags)
                                .build()
                        }
                    }
                    DescriptorType::SampledImage => {
                        vk::DescriptorSetLayoutBinding::builder()
                            .binding(binding.binding)
                            .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                            .descriptor_count(1)
                            .stage_flags(binding.stage_flags)
                            .build()
                    }
                    DescriptorType::UniformBuffer => {
                        vk::DescriptorSetLayoutBinding::builder()
                            .binding(binding.binding)
                            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                            .descriptor_count(1)
                            .stage_flags(binding.stage_flags)
                            .build()
                    }
                    DescriptorType::StorageBuffer => {
                        vk::DescriptorSetLayoutBinding::builder()
                            .binding(binding.binding)
                            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                            .descriptor_count(1)
                            .stage_flags(binding.stage_flags)
                            .build()
                    }
                    DescriptorType::AccelerationStructure => {
                        vk::DescriptorSetLayoutBinding::builder()
                            .binding(binding.binding)
                            .descriptor_type(vk::DescriptorType::ACCELERATION_STRUCTURE_KHR)
                            .descriptor_count(1)
                            .stage_flags(binding.stage_flags)
                            .build()
                    }
                    DescriptorType::StorageImage => {
                        vk::DescriptorSetLayoutBinding::builder()
                            .binding(binding.binding)
                            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                            .descriptor_count(1)
                            .stage_flags(binding.stage_flags)
                            .build()
                    }
                }
            })
            .collect::<Vec<_>>();
        let info = vk::DescriptorSetLayoutCreateInfo::builder()
            .bindings(vk_bindings.as_slice())
            .build();
        unsafe {
            let handle = device
                .handle
                .create_descriptor_set_layout(&info, None)
                .unwrap();
            if let Some(name) = name {
                device
                    .pdevice
                    .instance
                    .debug_utils_loader
                    .debug_utils_set_object_name(
                        device.handle.handle(),
                        &vk::DebugUtilsObjectNameInfoEXT::builder()
                            .object_handle(handle.as_raw())
                            .object_type(vk::ObjectType::DESCRIPTOR_SET_LAYOUT)
                            .object_name(CString::new(name).unwrap().as_ref())
                            .build(),
                    )
                    .unwrap();
            }

            Self {
                handle,
                device,
                bindings: bindings.to_owned(),
                vk_bindings,
            }
        }
    }
}

impl Drop for DescriptorSetLayout {
    fn drop(&mut self) {
        unsafe {
            self.device
                .handle
                .destroy_descriptor_set_layout(self.handle, None);
        }
    }
}

pub struct PipelineLayout {
    handle: vk::PipelineLayout,
    device: Arc<Device>,
}

impl PipelineLayout {
    pub fn new(
        device: Arc<Device>,
        name: Option<&str>,
        set_layouts: &[&DescriptorSetLayout],
        push_constant_ranges: &[vk::PushConstantRange],
    ) -> Self {
        let set_layouts = set_layouts
            .iter()
            .map(|layout| layout.handle)
            .collect::<Vec<_>>();
        let info = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(set_layouts.as_slice())
            .push_constant_ranges(push_constant_ranges)
            .build();
        unsafe {
            let handle = device.handle.create_pipeline_layout(&info, None).unwrap();
            if let Some(name) = name {
                device
                    .pdevice
                    .instance
                    .debug_utils_loader
                    .debug_utils_set_object_name(
                        device.handle.handle(),
                        &vk::DebugUtilsObjectNameInfoEXT::builder()
                            .object_handle(handle.as_raw())
                            .object_type(vk::ObjectType::PIPELINE_LAYOUT)
                            .object_name(CString::new(name).unwrap().as_ref())
                            .build(),
                    )
                    .unwrap();
            }
            Self { handle, device }
        }
    }
}

impl Drop for PipelineLayout {
    fn drop(&mut self) {
        unsafe {
            self.device
                .handle
                .destroy_pipeline_layout(self.handle, None);
        }
    }
}

pub trait Pipeline {
    fn layout(&self) -> &Arc<PipelineLayout>;
}

pub struct GraphicsPipeline {
    handle: vk::Pipeline,
    layout: Arc<PipelineLayout>,
    stages: Vec<Arc<ShaderStage>>,
    render_pass: Arc<RenderPass>,
}

impl GraphicsPipeline {
    pub fn new(
        name: Option<&str>,
        layout: Arc<PipelineLayout>,
        stages: Vec<Arc<ShaderStage>>,
        render_pass: Arc<RenderPass>,
        vertex_input_state: &vk::PipelineVertexInputStateCreateInfo,
        input_assembly_state: &vk::PipelineInputAssemblyStateCreateInfo,
        rasterization_state: &vk::PipelineRasterizationStateCreateInfo,
        multisample_state: &vk::PipelineMultisampleStateCreateInfo,
        depth_stencil_state: &vk::PipelineDepthStencilStateCreateInfo,
        color_blend_state: &vk::PipelineColorBlendStateCreateInfo,
        viewport_state: &vk::PipelineViewportStateCreateInfo,
        dynamic_state: &vk::PipelineDynamicStateCreateInfo,
    ) -> Self {
        let device = &layout.device;
        let stage_create_infos = stages
            .iter()
            .map(|s| s.shader_stage_create_info())
            .collect::<Vec<_>>();
        let info = vk::GraphicsPipelineCreateInfo::builder()
            .layout(layout.handle)
            .stages(&stage_create_infos)
            .vertex_input_state(vertex_input_state)
            .input_assembly_state(input_assembly_state)
            .rasterization_state(rasterization_state)
            .multisample_state(multisample_state)
            .depth_stencil_state(depth_stencil_state)
            .color_blend_state(color_blend_state)
            .viewport_state(viewport_state)
            .dynamic_state(dynamic_state)
            .render_pass(render_pass.handle)
            .build();
        unsafe {
            let handle = device
                .handle
                .create_graphics_pipelines(vk::PipelineCache::null(), &[info], None)
                .unwrap()
                .first()
                .unwrap()
                .to_owned();
            if let Some(name) = name {
                device
                    .pdevice
                    .instance
                    .debug_utils_loader
                    .debug_utils_set_object_name(
                        device.handle.handle(),
                        &vk::DebugUtilsObjectNameInfoEXT::builder()
                            .object_handle(handle.as_raw())
                            .object_type(vk::ObjectType::PIPELINE)
                            .object_name(CString::new(name).unwrap().as_ref())
                            .build(),
                    )
                    .unwrap();
            }
            Self {
                handle,
                layout,
                stages,
                render_pass,
            }
        }
    }
}

impl Drop for GraphicsPipeline {
    fn drop(&mut self) {
        unsafe {
            self.layout
                .device
                .handle
                .destroy_pipeline(self.handle, None);
        }
    }
}

impl Pipeline for GraphicsPipeline {
    fn layout(&self) -> &Arc<PipelineLayout> {
        &self.layout
    }
}

pub struct ComputePipeline {
    handle: vk::Pipeline,
    layout: Arc<PipelineLayout>,
    stage: Arc<ShaderStage>,
}

impl ComputePipeline {
    pub fn new(name: Option<&str>, layout: Arc<PipelineLayout>, stage: Arc<ShaderStage>) -> Self {
        unsafe {
            let device = layout.device.as_ref();
            let handle = device
                .handle
                .create_compute_pipelines(
                    vk::PipelineCache::null(),
                    &[vk::ComputePipelineCreateInfo::builder()
                        .layout(layout.handle)
                        .stage(stage.shader_stage_create_info())
                        .build()],
                    None,
                )
                .unwrap()
                .first()
                .unwrap()
                .to_owned();

            if let Some(name) = name {
                device
                    .pdevice
                    .instance
                    .debug_utils_loader
                    .debug_utils_set_object_name(
                        device.handle.handle(),
                        &vk::DebugUtilsObjectNameInfoEXT::builder()
                            .object_handle(handle.as_raw())
                            .object_type(vk::ObjectType::PIPELINE)
                            .object_name(CString::new(name).unwrap().as_ref())
                            .build(),
                    )
                    .unwrap();
            }

            Self {
                handle,
                layout,
                stage,
            }
        }
    }
}

impl Drop for ComputePipeline {
    fn drop(&mut self) {
        unsafe {
            self.layout
                .device
                .handle
                .destroy_pipeline(self.handle, None);
        }
    }
}

impl Pipeline for ComputePipeline {
    fn layout(&self) -> &Arc<PipelineLayout> {
        &self.layout
    }
}

pub struct RayTracingPipeline {
    handle: vk::Pipeline,
    layout: Arc<PipelineLayout>,
    stages: Vec<Arc<ShaderStage>>,
    sbt_buffer: Buffer,
    sbt_stride: u32,
}

impl RayTracingPipeline {
    pub fn new(
        name: Option<&str>,
        allocator: Arc<Allocator>,
        layout: Arc<PipelineLayout>,
        stages: Vec<Arc<ShaderStage>>,
        recursion_depth: u32,
        queue: &mut Queue,
    ) -> Self {
        let device = &layout.device;
        let stage_create_infos = stages
            .iter()
            .map(|s| s.shader_stage_create_info())
            .collect::<Vec<_>>();
        let group_create_infos = stage_create_infos
            .iter()
            .enumerate()
            .map(|(i, info)| {
                match info.stage {
                    vk::ShaderStageFlags::RAYGEN_KHR => {
                        vk::RayTracingShaderGroupCreateInfoKHR::builder()
                            .ty(vk::RayTracingShaderGroupTypeKHR::GENERAL)
                            .closest_hit_shader(vk::SHADER_UNUSED_KHR)
                            .general_shader(i as u32)
                            .any_hit_shader(vk::SHADER_UNUSED_KHR)
                            .intersection_shader(vk::SHADER_UNUSED_KHR)
                            .build()
                    }
                    vk::ShaderStageFlags::CLOSEST_HIT_KHR => {
                        vk::RayTracingShaderGroupCreateInfoKHR::builder()
                            .ty(vk::RayTracingShaderGroupTypeKHR::TRIANGLES_HIT_GROUP)
                            .closest_hit_shader(i as u32)
                            .general_shader(vk::SHADER_UNUSED_KHR)
                            .any_hit_shader(vk::SHADER_UNUSED_KHR)
                            .intersection_shader(vk::SHADER_UNUSED_KHR)
                            .build()
                    }
                    vk::ShaderStageFlags::MISS_KHR => {
                        vk::RayTracingShaderGroupCreateInfoKHR::builder()
                            .ty(vk::RayTracingShaderGroupTypeKHR::GENERAL)
                            .closest_hit_shader(vk::SHADER_UNUSED_KHR)
                            .general_shader(i as u32)
                            .any_hit_shader(vk::SHADER_UNUSED_KHR)
                            .intersection_shader(vk::SHADER_UNUSED_KHR)
                            .build()
                    }
                    _ => {
                        unimplemented!()
                    }
                }
            })
            .collect::<Vec<_>>();
        unsafe {
            let handle = device
                .ray_tracing_pipeline_loader
                .create_ray_tracing_pipelines(
                    vk::DeferredOperationKHR::null(),
                    vk::PipelineCache::null(),
                    &[vk::RayTracingPipelineCreateInfoKHR::builder()
                        .layout(layout.handle)
                        .stages(stage_create_infos.as_slice())
                        .groups(group_create_infos.as_slice())
                        .max_pipeline_ray_recursion_depth(recursion_depth)
                        .build()],
                    None,
                )
                .unwrap()
                .first()
                .unwrap()
                .to_owned();

            if let Some(name) = name {
                device
                    .pdevice
                    .instance
                    .debug_utils_loader
                    .debug_utils_set_object_name(
                        device.handle.handle(),
                        &vk::DebugUtilsObjectNameInfoEXT::builder()
                            .object_handle(handle.as_raw())
                            .object_type(vk::ObjectType::PIPELINE)
                            .object_name(CString::new(name).unwrap().as_ref())
                            .build(),
                    )
                    .unwrap();
            }

            let rt_p = &device.pdevice.ray_tracing_pipeline_properties;

            let shader_handle_storage = device
                .ray_tracing_pipeline_loader
                .get_ray_tracing_shader_group_handles(
                    handle,
                    0,
                    group_create_infos.len() as u32,
                    rt_p.shader_group_handle_size as usize * group_create_infos.len(),
                )
                .unwrap();
            assert!(rt_p.shader_group_base_alignment % rt_p.shader_group_handle_alignment == 0);
            let sbt_stride = rt_p.shader_group_base_alignment
                * ((rt_p.shader_group_handle_size + rt_p.shader_group_base_alignment - 1)
                    / rt_p.shader_group_base_alignment);
            assert!(sbt_stride <= rt_p.max_shader_group_stride);
            assert!(sbt_stride == 64);

            let sbt_size = sbt_stride * group_create_infos.len() as u32;

            let mut temp: Vec<u8> = vec![0; sbt_size as usize];
            for group_index in 0..group_create_infos.len() {
                std::ptr::copy_nonoverlapping(
                    shader_handle_storage
                        .as_ptr()
                        .add(group_index * rt_p.shader_group_handle_size as usize),
                    temp.as_mut_ptr().add(group_index * sbt_stride as usize),
                    rt_p.shader_group_handle_size as usize,
                );
            }
            let command_pool = Arc::new(CommandPool::new(device.clone()));
            let sbt_buffer = Buffer::new_init_device(
                Some("sbt buffer"),
                allocator.clone(),
                vk::BufferUsageFlags::SHADER_BINDING_TABLE_KHR
                    | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
                MemoryUsage::GpuOnly,
                queue,
                command_pool.clone(),
                temp,
            );

            Self {
                handle,
                layout,
                stages,
                sbt_buffer,
                sbt_stride,
            }
        }
    }

    pub fn sbt_buffer(&self) -> &Buffer {
        &self.sbt_buffer
    }

    pub fn sbt_stride(&self) -> u32 {
        self.sbt_stride
    }
}

impl Drop for RayTracingPipeline {
    fn drop(&mut self) {
        unsafe {
            self.layout
                .device
                .handle
                .destroy_pipeline(self.handle, None);
        }
    }
}

impl Pipeline for RayTracingPipeline {
    fn layout(&self) -> &Arc<PipelineLayout> {
        &self.layout
    }
}

pub struct ShaderModule {
    handle: vk::ShaderModule,
    device: Arc<Device>,
}

#[repr(C, align(32))]
struct AlignedSpirv {
    pub code: Vec<u8>,
}

impl ShaderModule {
    pub fn new<P>(device: Arc<Device>, spv: P) -> Self
    where
        P: AsRef<[u8]>,
    {
        let aligned = AlignedSpirv {
            code: spv.as_ref().to_vec(),
        };
        let info = vk::ShaderModuleCreateInfo::builder()
            .code(bytemuck::cast_slice(aligned.code.as_slice()))
            .build();
        unsafe {
            let handle = device.handle.create_shader_module(&info, None).unwrap();
            Self { handle, device }
        }
    }
}

impl Drop for ShaderModule {
    fn drop(&mut self) {
        unsafe {
            self.device.handle.destroy_shader_module(self.handle, None);
        }
    }
}

impl std::fmt::Debug for DescriptorSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DescriptorSet")
            .field("handle", &self.handle)
            .finish()
    }
}

pub struct DescriptorSet {
    handle: vk::DescriptorSet,
    descriptor_pool: Arc<DescriptorPool>,
    descriptor_set_layout: Arc<DescriptorSetLayout>,
    resources: RefCell<BTreeMap<u32, Arc<dyn Resource>>>,
}

impl DescriptorSet {
    pub fn new(
        name: Option<&str>,
        descriptor_pool: Arc<DescriptorPool>,
        descriptor_set_layout: Arc<DescriptorSetLayout>,
    ) -> Self {
        let device = &descriptor_pool.device;
        let info = vk::DescriptorSetAllocateInfo::builder()
            .set_layouts(&[descriptor_set_layout.handle])
            .descriptor_pool(descriptor_pool.handle)
            .build();

        unsafe {
            let handles = device.handle.allocate_descriptor_sets(&info).unwrap();
            assert_eq!(handles.len(), 1);
            let handle = handles.first().unwrap().to_owned();
            if let Some(name) = name {
                device
                    .pdevice
                    .instance
                    .debug_utils_loader
                    .debug_utils_set_object_name(
                        device.handle.handle(),
                        &vk::DebugUtilsObjectNameInfoEXT::builder()
                            .object_handle(handle.as_raw())
                            .object_type(vk::ObjectType::DESCRIPTOR_SET)
                            .object_name(CString::new(name).unwrap().as_ref())
                            .build(),
                    )
                    .unwrap();
            }

            Self {
                handle,
                descriptor_pool,
                descriptor_set_layout,
                resources: RefCell::new(BTreeMap::new()),
            }
        }
    }

    pub fn update(&self, update_infos: &[DescriptorSetUpdateInfo]) {
        let device = self.descriptor_pool.device.clone();
        let bindings = self.descriptor_set_layout.vk_bindings.clone();

        let mut buffer_infos = Vec::new();
        let mut image_infos = Vec::new();
        let mut tlas_handles = Vec::new();
        let mut write_acceleration_structure = None;

        let descriptor_writes = update_infos
            .iter()
            .map(|info| {
                let write_builder = vk::WriteDescriptorSet::builder()
                    .dst_set(self.handle)
                    .dst_binding(info.binding)
                    .descriptor_type(
                        bindings
                            .iter()
                            .filter(|binding| binding.binding == info.binding)
                            .map(|binding| binding.descriptor_type)
                            .next()
                            .unwrap(),
                    );
                let mut write = match info.detail.borrow() {
                    DescriptorSetUpdateDetail::Buffer { buffer, offset } => {
                        self.resources
                            .try_borrow_mut()
                            .unwrap()
                            .insert(info.binding, buffer.clone());
                        buffer_infos.push(
                            vk::DescriptorBufferInfo::builder()
                                .buffer(buffer.handle)
                                .offset(*offset)
                                .range(vk::WHOLE_SIZE)
                                .build(),
                        );

                        write_builder
                            .buffer_info(&buffer_infos.as_slice()[buffer_infos.len() - 1..])
                            .build()
                    }
                    DescriptorSetUpdateDetail::Image(image_view) => {
                        self.resources
                            .try_borrow_mut()
                            .unwrap()
                            .insert(info.binding, image_view.clone());
                        image_infos.push(
                            vk::DescriptorImageInfo::builder()
                                .image_layout(image_view.image.layout())
                                .image_view(image_view.handle)
                                .build(),
                        );
                        write_builder
                            .image_info(&image_infos.as_slice()[image_infos.len() - 1..])
                            .build()
                    }
                    DescriptorSetUpdateDetail::Sampler(sampler) => {
                        self.resources
                            .try_borrow_mut()
                            .unwrap()
                            .insert(info.binding, sampler.clone());
                        image_infos.push(
                            vk::DescriptorImageInfo::builder()
                                .sampler(sampler.handle)
                                .build(),
                        );
                        write_builder
                            .image_info(&image_infos.as_slice()[image_infos.len() - 1..])
                            .build()
                    }
                    DescriptorSetUpdateDetail::AccelerationStructure(tlas) => {
                        self.resources
                            .try_borrow_mut()
                            .unwrap()
                            .insert(info.binding, tlas.clone());
                        tlas_handles.push(tlas.handle);
                        write_acceleration_structure = Some(
                            vk::WriteDescriptorSetAccelerationStructureKHR::builder()
                                .acceleration_structures(tlas_handles.as_slice())
                                .build(),
                        );
                        write_builder
                            .push_next(write_acceleration_structure.as_mut().unwrap())
                            .build()
                    }
                };

                write.descriptor_count = 1;
                write
            })
            .collect::<Vec<_>>();
        assert_eq!(descriptor_writes.len(), update_infos.len());
        unsafe {
            device
                .handle
                .update_descriptor_sets(descriptor_writes.as_slice(), &[]);
        }
    }
}

pub enum DescriptorSetUpdateDetail {
    Buffer { buffer: Arc<Buffer>, offset: u64 },
    Image(Arc<ImageView>),
    Sampler(Arc<Sampler>),
    AccelerationStructure(Arc<AccelerationStructure>),
}

pub struct DescriptorSetUpdateInfo {
    pub binding: u32,
    pub detail: DescriptorSetUpdateDetail,
}

impl Drop for DescriptorSet {
    fn drop(&mut self) {
        unsafe {
            self.descriptor_pool
                .device
                .handle
                .free_descriptor_sets(self.descriptor_pool.handle, &[self.handle])
                .unwrap();
        }
    }
}

pub struct Sampler {
    handle: vk::Sampler,
    device: Arc<Device>,
}

impl Sampler {
    pub fn new(device: Arc<Device>) -> Self {
        let info = vk::SamplerCreateInfo::builder()
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .build();
        unsafe {
            let handle = device.handle.create_sampler(&info, None).unwrap();
            Self { handle, device }
        }
    }
}

impl Drop for Sampler {
    fn drop(&mut self) {
        unsafe {
            self.device.handle.destroy_sampler(self.handle, None);
        }
    }
}

pub struct ShaderStage {
    module: Arc<ShaderModule>,
    stage: vk::ShaderStageFlags,
    entry_point: String,
    entry_point_cstr: CString,
}

impl ShaderStage {
    pub fn new(module: Arc<ShaderModule>, stage: vk::ShaderStageFlags, entry_point: &str) -> Self {
        let entry_point_cstr = CString::new(entry_point).unwrap();
        Self {
            module,
            stage,
            entry_point: entry_point.to_string(),
            entry_point_cstr,
        }
    }

    fn shader_stage_create_info(&self) -> vk::PipelineShaderStageCreateInfo {
        vk::PipelineShaderStageCreateInfo::builder()
            .module(self.module.handle)
            .stage(self.stage)
            .name(&self.entry_point_cstr)
            .build()
    }
}

pub struct AccelerationStructure {
    handle: vk::AccelerationStructureKHR,
    as_buffer: Buffer,
    device_address: u64,
    device: Arc<Device>,
}

impl AccelerationStructure {
    pub fn new(
        name: Option<&str>,
        allocator: Arc<Allocator>,
        geometries: &[vk::AccelerationStructureGeometryKHR],
        primitive_counts: &[u32],
        as_type: vk::AccelerationStructureTypeKHR,
    ) -> Self {
        assert_eq!(geometries.len(), primitive_counts.len());
        let device = &allocator.device;
        let mut queue = Queue::new(device.clone());
        let command_pool = Arc::new(CommandPool::new(device.clone()));
        unsafe {
            let size_info = allocator
                .device
                .acceleration_structure_loader
                .get_acceleration_structure_build_sizes(
                    vk::AccelerationStructureBuildTypeKHR::DEVICE,
                    &vk::AccelerationStructureBuildGeometryInfoKHR::builder()
                        .flags(vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE)
                        .ty(as_type)
                        .geometries(geometries)
                        .build(),
                    primitive_counts,
                );
            let as_buffer = Buffer::new(
                Some(&format!(
                    "{} buffer",
                    name.unwrap_or("acceleration structure")
                )),
                allocator.clone(),
                size_info.acceleration_structure_size,
                vk::BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR
                    | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
                vk_mem::MemoryUsage::GpuOnly,
            );

            let handle = allocator
                .device
                .acceleration_structure_loader
                .create_acceleration_structure(
                    &vk::AccelerationStructureCreateInfoKHR::builder()
                        .ty(as_type)
                        .buffer(as_buffer.handle)
                        .size(size_info.acceleration_structure_size)
                        .build(),
                    None,
                )
                .unwrap();

            let device = allocator.device.clone();

            if let Some(name) = name {
                device
                    .pdevice
                    .instance
                    .debug_utils_loader
                    .debug_utils_set_object_name(
                        device.handle.handle(),
                        &vk::DebugUtilsObjectNameInfoEXT::builder()
                            .object_handle(handle.as_raw())
                            .object_type(vk::ObjectType::ACCELERATION_STRUCTURE_KHR)
                            .object_name(CString::new(name).unwrap().as_ref())
                            .build(),
                    )
                    .unwrap();
            }

            let scratch_buffer = Buffer::new(
                Some(&format!(
                    "{} scratch buffer",
                    name.unwrap_or("acceleration structure")
                )),
                allocator.clone(),
                size_info.build_scratch_size,
                vk::BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR
                    | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
                vk_mem::MemoryUsage::GpuOnly,
            );

            let build_geometry_info = vk::AccelerationStructureBuildGeometryInfoKHR::builder()
                .flags(vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE)
                .ty(as_type)
                .geometries(geometries)
                .dst_acceleration_structure(handle)
                .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
                .scratch_data(vk::DeviceOrHostAddressKHR {
                    device_address: scratch_buffer.device_address(),
                })
                .build();

            let build_range_infos = primitive_counts
                .iter()
                .map(|count| {
                    vk::AccelerationStructureBuildRangeInfoKHR::builder()
                        .first_vertex(0)
                        .primitive_offset(0)
                        .transform_offset(0)
                        .primitive_count(*count)
                        .build()
                })
                .collect::<Vec<_>>();

            let device_address = device
                .acceleration_structure_loader
                .get_acceleration_structure_device_address(
                    &vk::AccelerationStructureDeviceAddressInfoKHR::builder()
                        .acceleration_structure(handle)
                        .build(),
                );
            let result = Self {
                handle,
                as_buffer,
                device_address,
                device,
            };

            let mut command_buffer = CommandBuffer::new(command_pool);
            command_buffer.encode(|recorder| {
                recorder.build_acceleration_structure_raw(
                    build_geometry_info,
                    build_range_infos.as_ref(),
                )
            });

            queue.submit_binary(command_buffer, &[], &[], &[]).wait();

            result
        }
    }

    pub fn device_address(&self) -> u64 {
        self.device_address
    }
}

impl Drop for AccelerationStructure {
    fn drop(&mut self) {
        unsafe {
            self.device
                .acceleration_structure_loader
                .destroy_acceleration_structure(self.handle, None);
        }
    }
}
