#![feature(negative_impls)]

use ash::version::{DeviceV1_0, DeviceV1_2, EntryV1_0, InstanceV1_0};

use anyhow::Result;
use ash::extensions;
use vk::Handle;

use std::collections::{BTreeMap, BTreeSet, HashMap, LinkedList};
use std::ffi::{CStr, CString};
use std::sync::{Arc, Mutex};

pub use ash::vk;
pub use vk_mem::MemoryUsage;

pub mod name {
    pub mod instance {
        pub mod layer {
            pub mod khronos {
                pub const VALIDATION: &str = "VK_LAYER_KHRONOS_validation";
            }
            pub mod lunarg {
                pub const MONITOR: &str = "VK_LAYER_LUNARG_monitor";
                pub const GFXRECONSTRUCT: &str = "VK_LAYER_LUNARG_gfxreconstruct";
            }
        }
        pub mod extension {
            pub mod ext {
                pub const DEBUG_UTILS: &str = "VK_EXT_debug_utils";
                pub const DEBUG_MARKER: &str = "VK_EXT_debug_marker";
            }
        }
    }
    pub mod device {
        mod layer {}
        pub mod extension {
            pub mod khr {
                pub const SWAPCHAIN: &str = "VK_KHR_swapchain";
                pub const DEFERED_HOST_OPERATION: &str = "VK_KHR_deferred_host_operations";
                pub const RAY_TRACING_PIPELINE: &str = "VK_KHR_ray_tracing_pipeline";
                pub const ACCELERATION_STRUCTURE: &str = "VK_KHR_acceleration_structure";
            }
            mod ext {}
        }
    }
}

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
    surface_loader: ash::extensions::khr::Surface,
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
        let surface_loader = ash::extensions::khr::Surface::new(&entry.handle, &handle);

        let result = Self {
            handle,
            entry,
            surface_loader,
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

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            self.instance
                .surface_loader
                .destroy_surface(self.handle, None);
        }
    }
}

pub struct Device {
    handle: ash::Device,
    pdevice: Arc<PhysicalDevice>,
    acceleration_structure_loader: ash::extensions::khr::AccelerationStructure,
    swapchain_loader: ash::extensions::khr::Swapchain,
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

            let acceleration_structure_loader =
                ash::extensions::khr::AccelerationStructure::new(&pdevice.instance.handle, &handle);

            let swapchain_loader =
                ash::extensions::khr::Swapchain::new(&pdevice.instance.handle, &handle);

            Self {
                handle,
                pdevice,
                acceleration_structure_loader,
                swapchain_loader,
            }
        }
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
    mapped: bool,
    device_address: vk::DeviceAddress,
    size: usize,
    allocation_info: vk_mem::AllocationInfo,
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

        unsafe {
            let device_address = allocator.device.handle.get_buffer_device_address(
                &vk::BufferDeviceAddressInfo::builder()
                    .buffer(handle)
                    .build(),
            );

            Self {
                handle,
                allocation,
                mapped: false,
                device_address,
                size: size.to_usize().unwrap(),
                allocator,
                allocation_info,
            }
        }
    }

    pub fn new_init<I: AsRef<[u8]>>(
        allocator: Arc<Allocator>,
        buffer_usage: vk::BufferUsageFlags,
        memory_usage: vk_mem::MemoryUsage,
        queue: &mut Queue,
        command_pool: Arc<CommandPool>,
        data: I,
    ) -> Self {
        let data = data.as_ref();
        let buffer = Arc::new(Self::new(
            allocator.clone(),
            data.len(),
            buffer_usage
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::TRANSFER_DST,
            memory_usage,
        ));
        if buffer.is_device_local() {
            let staging_buffer = Arc::new(Self::new(
                allocator.clone(),
                data.len(),
                vk::BufferUsageFlags::TRANSFER_SRC,
                vk_mem::MemoryUsage::CpuToGpu,
            ));
            staging_buffer.copy_from(data);
            let mut cmd_buf = CommandBuffer::new(command_pool);
            cmd_buf.encode(|manager| {
                manager.copy_buffer(
                    staging_buffer,
                    buffer.clone(),
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
        }
        Arc::try_unwrap(buffer).unwrap()
    }

    pub fn map(&mut self) -> *mut u8 {
        if self.is_device_local() {
            panic!("cannot map device local memory");
        }
        self.mapped = true;
        self.allocator.handle.map_memory(&self.allocation).unwrap()
    }

    pub fn unmap(&mut self) {
        if self.mapped {
            self.allocator.handle.unmap_memory(&self.allocation);
            self.mapped = false;
        }
    }

    pub fn memory_type(&self) -> u32 {
        self.allocation_info.get_memory_type()
    }

    pub fn device_address(&self) -> vk::DeviceAddress {
        self.device_address
    }

    pub fn copy_from<I: AsRef<[u8]>>(&self, data: I) {
        let data = data.as_ref();
        let mapped = self.allocator.handle.map_memory(&self.allocation).unwrap();
        let mapped_bytes = unsafe { std::slice::from_raw_parts_mut(mapped, self.size) };
        mapped_bytes.copy_from_slice(data);
        self.allocator.handle.unmap_memory(&self.allocation);
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn is_device_local(&self) -> bool {
        self.allocation_info.get_memory_type() & vk::MemoryPropertyFlags::DEVICE_LOCAL.as_raw() != 0
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        if self.mapped {
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
    command_buffers: HashMap<vk::CommandBuffer, (Arc<Mutex<bool>>, CommandBuffer)>,
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
            if let Ok(in_use_locked) = in_use.try_lock() {
                if !*in_use_locked {
                    removal_list.push(handle.clone());
                }
            }
        }
        for removal in removal_list {
            self.command_buffers.remove(&removal);
        }
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

            let submit_info = vk::SubmitInfo::builder()
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
                .build();

            let fence = Fence::new(self.device.clone(), false);
            self.device
                .handle
                .queue_submit(self.handle, &[submit_info], fence.handle)
                .unwrap();

            let in_use = Arc::new(Mutex::new(true));
            let in_use_signaler = in_use.clone();

            tokio::task::spawn(async move {
                fence.wait();
                *in_use_signaler.lock().unwrap() = false;
            });

            self.command_buffers
                .insert(command_buffer.handle, (in_use, command_buffer));
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
    pub fn new(device: Arc<Device>) -> Result<Self> {
        unsafe {
            let handle = device
                .handle
                .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)?;
            Ok(Self { handle, device })
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

pub struct CommandRecorder<'a> {
    command_buffer: &'a mut CommandBuffer,
    bind_point: Option<vk::PipelineBindPoint>,
}

impl<'a> CommandRecorder<'a> {
    pub fn copy_buffer(&mut self, src: Arc<Buffer>, dst: Arc<Buffer>, region: &[vk::BufferCopy]) {
        unsafe {
            self.device().handle.cmd_copy_buffer(
                self.command_buffer.handle,
                src.handle,
                dst.handle,
                region,
            );
            self.command_buffer.resources.push(src);
            self.command_buffer.resources.push(dst);
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
        I: FnOnce(&mut CommandRecorder, &dyn Pipeline),
    {
        unsafe {
            self.device().handle.cmd_bind_pipeline(
                self.command_buffer.handle,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.handle,
            );
            f(self, pipeline.as_ref());
        }
        // self.command_buffer.resources.push(pipeline);
    }

    pub fn bind_descriptor_sets(
        &mut self,
        mut descriptor_sets: Vec<Arc<DescriptorSet>>,
        layout: &PipelineLayout,
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
                0,
                descriptor_set_handles.as_slice(),
                &[],
            );
        }

        while !descriptor_sets.is_empty() {
            self.command_buffer
                .resources
                .push(descriptor_sets.pop().unwrap());
        }
    }

    pub fn bind_index_buffer(
        &mut self,
        buffer: Arc<Buffer>,
        offset: u64,
        index_type: vk::IndexType,
    ) {
        unsafe {
            self.device().handle.cmd_bind_index_buffer(
                self.command_buffer.handle,
                buffer.handle,
                offset,
                index_type,
            );
        }
        self.command_buffer.resources.push(buffer);
    }

    pub fn bind_vertex_buffer(&mut self, buffers: Vec<Arc<Buffer>>, offsets: &[u64]) {
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

    pub fn set_scissor(&self, scissors: &[vk::Rect2D]) {
        unsafe {
            self.device()
                .handle
                .cmd_set_scissor(self.command_buffer.handle, 0, scissors);
        }
    }

    pub fn draw_indexed(&self, index_count: u32, instance_count: u32) {
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

    fn device(&self) -> &Device {
        &self.command_buffer.pool.device
    }
}

trait Resource {}

impl Resource for Buffer {}
impl Resource for RenderPass {}
impl Resource for Framebuffer {}
impl Resource for GraphicsPipeline {}
impl Resource for DescriptorSet {}
impl Resource for PipelineLayout {}

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
    handle: vk::SwapchainKHR,
    device: Arc<Device>,
    surface: Arc<Surface>,
    extent: vk::Extent2D,
    format: vk::Format,
}

impl Swapchain {
    pub fn new(device: Arc<Device>, surface: Arc<Surface>) -> Self {
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
                .present_mode(vk::PresentModeKHR::FIFO)
                .clipped(true)
                .image_array_layers(1);
            let handle = device
                .swapchain_loader
                .create_swapchain(&swapchain_create_info, None)
                .unwrap();

            Self {
                handle,
                device,
                surface,
                extent: surface_capabilities.current_extent,
                format,
            }
        }
    }

    pub fn acquire_next_image(&self, semaphore: vk::Semaphore) -> Result<(u32, bool)> {
        unsafe {
            Ok(self.device.swapchain_loader.acquire_next_image(
                self.handle,
                0,
                semaphore,
                vk::Fence::null(),
            )?)
        }
    }

    pub fn renew(&mut self) {
        let swapchain_loader = &self.device.swapchain_loader;
        let surface_loader = &self.device.pdevice.instance.surface_loader;
        let pdevice = &self.device.pdevice;
        unsafe {
            swapchain_loader.destroy_swapchain(self.handle, None);
            let surface_capabilities = surface_loader
                .get_physical_device_surface_capabilities(pdevice.handle, self.surface.handle)
                .unwrap();

            let surface_format = surface_loader
                .get_physical_device_surface_formats(pdevice.handle, self.surface.handle)
                .unwrap()[0];

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
                .present_mode(vk::PresentModeKHR::FIFO)
                .clipped(true)
                .image_array_layers(1);
            self.handle = swapchain_loader
                .create_swapchain(&swapchain_create_info, None)
                .unwrap();
            self.extent = surface_capabilities.current_extent;
        }
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        unsafe {
            self.device
                .swapchain_loader
                .destroy_swapchain(self.handle, None)
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
    layout: vk::ImageLayout,
    format: vk::Format,
}

impl Image {
    pub fn new(
        allocator: Arc<Allocator>,
        width: u32,
        height: u32,
        image_usage: vk::ImageUsageFlags,
        memory_usage: vk_mem::MemoryUsage,
    ) -> Self {
        let format = vk::Format::B8G8R8A8_UNORM;
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
                    .tiling(vk::ImageTiling::OPTIMAL)
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

        let image_type = ImageType::Allocated {
            allocator,
            allocation,
            allocation_info,
        };

        Self {
            handle,
            width,
            height,
            layout: vk::ImageLayout::UNDEFINED,
            image_type,
            format,
        }
    }

    pub fn from_swapchain(swapchain: Arc<Swapchain>) -> Vec<Self> {
        unsafe {
            let device = swapchain.device.as_ref();
            let images = device
                .swapchain_loader
                .get_swapchain_images(swapchain.handle)
                .unwrap();

            let results = images
                .into_iter()
                .map(|handle| Self {
                    handle,
                    image_type: ImageType::Swapchain {
                        swapchain: swapchain.clone(),
                    },
                    width: swapchain.extent.width,
                    height: swapchain.extent.height,
                    layout: vk::ImageLayout::UNDEFINED,
                    format: swapchain.format,
                })
                .collect::<Vec<_>>();

            results
        }
    }

    // pub fn view(&self) -> vk::ImageView {
    //     self.view
    // }

    pub fn cmd_set_layout(
        &mut self,
        command_buffer: &CommandBuffer,
        layout: vk::ImageLayout,
        need_old_data: bool,
    ) {
        let old_layout = match need_old_data {
            true => self.layout,
            false => vk::ImageLayout::UNDEFINED,
        };
        cmd_set_image_layout(old_layout, command_buffer, self.handle, layout);
        self.layout = layout;
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

pub struct DescriptorSetLayout {
    handle: vk::DescriptorSetLayout,
    device: Arc<Device>,
}

impl DescriptorSetLayout {
    pub fn new(device: Arc<Device>, bindings: &[vk::DescriptorSetLayoutBinding]) -> Self {
        let info = vk::DescriptorSetLayoutCreateInfo::builder()
            .bindings(bindings)
            .build();
        unsafe {
            let handle = device
                .handle
                .create_descriptor_set_layout(&info, None)
                .unwrap();
            Self { handle, device }
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
    pub fn new(device: Arc<Device>, set_layouts: &[&DescriptorSetLayout]) -> Self {
        let set_layouts = set_layouts
            .iter()
            .map(|layout| layout.handle)
            .collect::<Vec<_>>();
        let info = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(set_layouts.as_slice())
            .build();
        unsafe {
            let handle = device.handle.create_pipeline_layout(&info, None).unwrap();
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

pub struct DescriptorSet {
    handle: vk::DescriptorSet,
    descriptor_pool: Arc<DescriptorPool>,
}

impl DescriptorSet {
    pub fn new(
        descriptor_pool: Arc<DescriptorPool>,
        descriptor_set_layout: &DescriptorSetLayout,
    ) -> Self {
        let device = &descriptor_pool.device;
        let info = vk::DescriptorSetAllocateInfo::builder()
            .set_layouts(&[descriptor_set_layout.handle])
            .descriptor_pool(descriptor_pool.handle)
            .build();
        unsafe {
            let handle = device
                .handle
                .allocate_descriptor_sets(&info)
                .unwrap()
                .first()
                .unwrap()
                .to_owned();
            Self {
                handle,
                descriptor_pool,
            }
        }
    }
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
    module: ShaderModule,
    stage: vk::ShaderStageFlags,
    entry_point: String,
    entry_point_cstr: CString,
}

impl ShaderStage {
    pub fn new(module: ShaderModule, stage: vk::ShaderStageFlags, entry_point: &str) -> Self {
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

// pub struct AccelerationStructure {
//     handle: vk::AccelerationStructureKHR,
//     as_buffer: Buffer,
//     device_address: u64,
// }

// impl AccelerationStructure {
//     pub fn new(
//         allocator: Arc<Allocator>,
//         geometries: &[vk::AccelerationStructureGeometryKHR],
//         as_type: vk::AccelerationStructureTypeKHR,
//         primitive_count: u32,
//         queue: &Queue,
//     ) -> Result<Self> {
//         unsafe {
//             let build_geometry_info = vk::AccelerationStructureBuildGeometryInfoKHR::builder()
//                 .flags(vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE)
//                 .ty(as_type)
//                 .geometries(geometries);
//             let size_info = allocator
//                 .device
//                 .acceleration_structure_loader
//                 .get_acceleration_structure_build_sizes(
//                     allocator.device.handle.handle(),
//                     vk::AccelerationStructureBuildTypeKHR::DEVICE,
//                     &build_geometry_info,
//                     &[1],
//                 );
//             let as_buffer = Buffer::new(
//                 allocator.clone(),
//                 size_info.acceleration_structure_size,
//                 vk::BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR
//                     | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
//                 vk_mem::MemoryUsage::GpuOnly,
//             );

//             let handle = allocator
//                 .device
//                 .acceleration_structure_loader
//                 .create_acceleration_structure(
//                     &vk::AccelerationStructureCreateInfoKHR::builder()
//                         .ty(as_type)
//                         .buffer(as_buffer.handle)
//                         .size(size_info.acceleration_structure_size)
//                         .build(),
//                     None,
//                 )?;

//             let scratch_buffer = Buffer::new(
//                 allocator.clone(),
//                 size_info.build_scratch_size,
//                 vk::BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR
//                     | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
//                 vk_mem::MemoryUsage::GpuOnly,
//             );

//             let build_geometry_info = build_geometry_info
//                 .dst_acceleration_structure(handle)
//                 .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
//                 .scratch_data(vk::DeviceOrHostAddressKHR {
//                     device_address: scratch_buffer.device_address(),
//                 });

//             let build_range_info = vk::AccelerationStructureBuildRangeInfoKHR::builder()
//                 .first_vertex(0)
//                 .primitive_offset(0)
//                 .transform_offset(0)
//                 .primitive_count(primitive_count)
//                 .build();

//             let command_buffer = CommandBuffer::new(&vulkan.device, vulkan.command_pool)?;
//             command_buffer.begin()?;
//             vulkan
//                 .acceleration_structure_loader
//                 .cmd_build_acceleration_structures(
//                     command_buffer.handle(),
//                     &[build_geometry_info.build()],
//                     &[&[build_range_info]],
//                 );
//             command_buffer.end()?;
//             queue.submit_binary(command_buffer, &[], &[], &[])?.wait()?;

//             let device_address = vulkan
//                 .acceleration_structure_loader
//                 .get_acceleration_structure_device_address(
//                     vulkan.device.handle(),
//                     &vk::AccelerationStructureDeviceAddressInfoKHR::builder()
//                         .acceleration_structure(handle)
//                         .build(),
//                 );

//             Ok(Self {
//                 handle,
//                 as_buffer,
//                 device_address,
//             })
//         }
//     }

//     pub fn device_address(&self) -> u64 {
//         self.device_address
//     }

//     pub fn handle(&self) -> vk::AccelerationStructureKHR {
//         self.handle
//     }
// }
