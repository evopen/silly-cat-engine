use std::borrow::Cow;
use std::ffi::{CStr, CString};

use anyhow::{anyhow, bail, Result};
use ash::version::{DeviceV1_0, EntryV1_0, InstanceV1_0};
use ash::vk;

pub struct SwapchainDescription {
    image_count: u32,
}

#[repr(u64)]
enum SemaphoreState {
    Initial,
    Wait,
    Finish,
}

#[derive(Clone)]
pub struct Swapchain {
    pub(super) swapchain: vk::SwapchainKHR,
    pub(super) swapchain_loader: ash::extensions::khr::Swapchain,
    pub(super) image_views: Vec<vk::ImageView>,
    fence: vk::Fence,
    device: ash::Device,
    current_image_used: bool,
    current_image_index: u32,
}

impl Swapchain {
    pub(super) fn new(
        swapchain_loader: &ash::extensions::khr::Swapchain,
        swapchain_info: &vk::SwapchainCreateInfoKHR,
        device: &ash::Device,
    ) -> Self {
        let swapchain = unsafe { swapchain_loader.create_swapchain(&swapchain_info, None) }
            .expect(format!("{:?}", swapchain_info).as_str());

        let images = unsafe { swapchain_loader.get_swapchain_images(swapchain) }.unwrap();
        let image_views: Vec<vk::ImageView> = images
            .iter()
            .map(|&image| {
                let create_view_info = vk::ImageViewCreateInfo::builder()
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(swapchain_info.image_format)
                    .components(vk::ComponentMapping {
                        r: vk::ComponentSwizzle::R,
                        g: vk::ComponentSwizzle::G,
                        b: vk::ComponentSwizzle::B,
                        a: vk::ComponentSwizzle::A,
                    })
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    })
                    .image(image);
                unsafe { device.create_image_view(&create_view_info, None) }.unwrap()
            })
            .collect();

        let fence_info = vk::FenceCreateInfo::default();
        let fence = unsafe { device.create_fence(&fence_info, None) }.unwrap();

        Self {
            swapchain,
            swapchain_loader: swapchain_loader.clone(),
            image_views,
            fence,
            device: device.clone(),
            current_image_used: true,
            current_image_index: 0,
        }
    }

    pub fn get_current_frame(&mut self) -> vk::ImageView {
        unsafe {
            if self.current_image_used {
                let (image_index, sub_optimal) = self
                    .swapchain_loader
                    .acquire_next_image(
                        self.swapchain,
                        std::u64::MAX,
                        vk::Semaphore::null(),
                        self.fence,
                    )
                    .unwrap();
                self.device
                    .wait_for_fences(&[self.fence], true, std::u64::MAX);
                self.device.reset_fences(&[self.fence]).unwrap();
                self.current_image_index = image_index;
                self.current_image_used = false;
                self.image_views[image_index as usize]
            } else {
                self.image_views[self.current_image_index as usize]
            }
        }
    }
}
