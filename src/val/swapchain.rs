use std::borrow::Cow;
use std::ffi::{CStr, CString};

use anyhow::{anyhow, bail, Result};
use ash::version::{DeviceV1_0, EntryV1_0, InstanceV1_0};
use ash::vk;

pub struct SwapchainDescription {
    image_count: u32,
}

pub struct Swapchain {
    swapchain: vk::SwapchainKHR,
}

impl Swapchain {
    pub(super) fn new(
        instance: &ash::Instance,
        device: &ash::Device,
        swapchain_info: &vk::SwapchainCreateInfoKHR,
    ) -> Self {
        let swapchain_loader = ash::extensions::khr::Swapchain::new(instance, device);
        let swapchain =
            unsafe { swapchain_loader.create_swapchain(&swapchain_info, None) }.unwrap();
        Self { swapchain }
    }
}
