use std::borrow::Cow;
use std::ffi::{CStr, CString};

use anyhow::{anyhow, bail, Result};
use ash::version::{DeviceV1_0, EntryV1_0, InstanceV1_0};
use ash::vk;

pub struct CommandBuffer {
    pub(super) command_buffer: vk::CommandBuffer,
    device: ash::Device,
}

impl CommandBuffer {
    pub(super) fn new(command_pool: vk::CommandPool, device: &ash::Device) -> Self {
        let command_buf_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(command_pool)
            .command_buffer_count(1)
            .level(vk::CommandBufferLevel::PRIMARY);
        let command_buffer =
            unsafe { device.allocate_command_buffers(&command_buf_info) }.unwrap()[0];
        Self {
            command_buffer,
            device: device.clone(),
        }
    }
}
