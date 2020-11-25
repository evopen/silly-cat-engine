use std::borrow::Cow;
use std::ffi::{CStr, CString};

use anyhow::{anyhow, bail, Result};
use ash::version::{DeviceV1_0, EntryV1_0, InstanceV1_0};
use ash::vk;

pub struct SwapchainDescription {
    image_count: u32,
}

#[derive(Clone)]
pub struct Swapchain {
    pub(super) swapchain: vk::SwapchainKHR,
    pub(super) swapchain_loader: ash::extensions::khr::Swapchain,
}

impl Swapchain {
    pub(super) fn new(
        swapchain_loader: &ash::extensions::khr::Swapchain,
        swapchain_info: &vk::SwapchainCreateInfoKHR,
    ) -> Self {
        let swapchain = unsafe { swapchain_loader.create_swapchain(&swapchain_info, None) }
            .expect(format!("{:?}", swapchain_info).as_str());

        Self {
            swapchain,
            swapchain_loader: swapchain_loader.clone(),
        }
    }
}

// impl Drop for Swapchain {
//     fn drop(&mut self) {
//         unsafe {
//             self.swapchain_loader
//                 .destroy_swapchain(self.swapchain, None);
//         }
//     }
// }
