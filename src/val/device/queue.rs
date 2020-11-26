use anyhow::{anyhow, bail, Result};
use ash::version::{DeviceV1_0, EntryV1_0, InstanceV1_0};
use ash::vk;

use super::CommandBuffer;

pub struct Queue {
    queue: vk::Queue,
    device: ash::Device,
}

impl Queue {
    pub(super) fn new(device: &ash::Device, queue_family_index: u32, queue_index: u32) -> Self {
        let queue = unsafe { device.get_device_queue(queue_family_index, queue_index) };
        Self {
            queue,
            device: device.clone(),
        }
    }

    pub fn submit(&self, cmd_buf: &CommandBuffer, semaphore: vk::Semaphore) {
        let submit_info = vk::SubmitInfo::builder()
            .command_buffers(&[cmd_buf.command_buffer])
            .wait_semaphores(&[semaphore])
            .signal_semaphores(&[semaphore])
            .build();
        unsafe {
            self.device
                .queue_submit(self.queue, &[submit_info], vk::Fence::null())
        }
        .unwrap();
    }
}
