use anyhow::{anyhow, bail, Result};
use ash::version::{DeviceV1_0, EntryV1_0, InstanceV1_0};
use ash::vk;

pub struct Queue {
    queue: vk::Queue,
}

impl Queue {
    pub(super) fn new(device: &ash::Device, queue_family_index: u32, queue_index: u32) -> Self {
        let queue = unsafe { device.get_device_queue(queue_family_index, queue_index) };
        Self { queue }
    }
}
