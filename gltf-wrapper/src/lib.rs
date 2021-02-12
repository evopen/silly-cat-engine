use std::path::Path;
use std::sync::Arc;
use std::unimplemented;

use safe_vk::vk;

pub struct Scene {
    doc: gltf::Document,
    buffers: Vec<safe_vk::Buffer>,
    images: Vec<safe_vk::Image>,
}

impl Scene {
    pub fn from_file<I: AsRef<Path>>(allocator: Arc<safe_vk::Allocator>, path: I) -> Self {
        let mut queue = safe_vk::Queue::new(allocator.device().clone());
        let command_pool = Arc::new(safe_vk::CommandPool::new(allocator.device().clone()));
        let (doc, gltf_buffers, gltf_images) = gltf::import(path).unwrap();

        let buffers = gltf_buffers
            .iter()
            .map(|data| {
                safe_vk::Buffer::new_init_host(
                    Some("gltf buffer"),
                    allocator.clone(),
                    vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
                    safe_vk::MemoryUsage::CpuToGpu,
                    data.as_ref(),
                )
            })
            .collect::<Vec<_>>();

        let images = gltf_images
            .iter()
            .map(|image| {
                println!("fuck");
                let format = match image.format {
                    gltf::image::Format::R8 => vk::Format::R8_UNORM,
                    gltf::image::Format::R8G8 => vk::Format::R8G8_UNORM,
                    gltf::image::Format::R8G8B8 => vk::Format::R8G8B8_UNORM,
                    gltf::image::Format::R8G8B8A8 => vk::Format::R8G8B8A8_UNORM,
                    gltf::image::Format::B8G8R8 => vk::Format::B8G8R8_UNORM,
                    gltf::image::Format::B8G8R8A8 => vk::Format::B8G8R8A8_UNORM,
                    _ => {
                        unimplemented!()
                    }
                };

                safe_vk::Image::new_init_host(
                    allocator.clone(),
                    format,
                    image.width,
                    image.height,
                    vk::ImageUsageFlags::SAMPLED,
                    safe_vk::MemoryUsage::CpuToGpu,
                    &mut queue,
                    command_pool.clone(),
                    &image.pixels,
                )
            })
            .collect::<Vec<_>>();

        Self {
            doc,
            buffers,
            images,
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn test_all() {
        let entry = Arc::new(safe_vk::Entry::new().unwrap());

        let instance = Arc::new(safe_vk::Instance::new(
            entry.clone(),
            &[
                safe_vk::name::instance::layer::khronos::VALIDATION,
                safe_vk::name::instance::layer::lunarg::MONITOR,
            ],
            &[safe_vk::name::instance::extension::ext::DEBUG_UTILS],
        ));
        let pdevice = Arc::new(safe_vk::PhysicalDevice::new(instance.clone(), None));

        let device = Arc::new(safe_vk::Device::new(
            pdevice.clone(),
            &vk::PhysicalDeviceFeatures::default(),
            &[],
        ));
        let allocator = Arc::new(safe_vk::Allocator::new(device.clone()));

        dbg!(&std::env::current_dir());
        let scene = Scene::from_file(allocator.clone(), "../models/2.0/Box/glTF-Binary/Box.glb");
        let scene = Scene::from_file(allocator.clone(), "../models/2.0/Box/glTF/Box.gltf");
    }
}
