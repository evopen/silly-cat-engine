use std::path::Path;
use std::sync::Arc;
use std::unimplemented;

use glam::u32;
use safe_vk::vk;

pub struct Scene {
    doc: gltf::Document,
    buffers: Vec<safe_vk::Buffer>,
    images: Vec<safe_vk::Image>,
    bottom_level_acceleration_structures: Vec<safe_vk::AccelerationStructure>,
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

        assert_eq!(doc.scenes().len(), 1);
        let scene = doc.scenes().next().unwrap();
        for node in scene.nodes() {
            let _transform = glam::Mat4::from_cols_array_2d(&node.transform().matrix());
        }

        let bottom_level_acceleration_structures = doc
            .meshes()
            .map(|mesh| {
                let (geometries, triangle_count): (Vec<_>, Vec<_>) = mesh
                    .primitives()
                    .map(|primitive| {
                        let (index_type, index_data) = match primitive.indices() {
                            Some(accessor) => {
                                let index_type = match accessor.data_type() {
                                    gltf::accessor::DataType::U16 => vk::IndexType::UINT16,
                                    gltf::accessor::DataType::U32 => vk::IndexType::UINT32,
                                    _ => {
                                        panic!("not supported");
                                    }
                                };
                                let offset =
                                    (accessor.offset() + accessor.view().unwrap().offset()) as u64;
                                let index = accessor.view().unwrap().buffer().index();
                                accessor.view().unwrap().offset();
                                (
                                    index_type,
                                    vk::DeviceOrHostAddressConstKHR {
                                        device_address: buffers
                                            .get(index)
                                            .unwrap()
                                            .device_address()
                                            + offset,
                                    },
                                )
                            }
                            None => {
                                (
                                    vk::IndexType::NONE_KHR,
                                    vk::DeviceOrHostAddressConstKHR::default(),
                                )
                            }
                        };

                        let (_, accessor) = primitive
                            .attributes()
                            .find(|(semantic, _)| semantic.eq(&gltf::Semantic::Positions))
                            .unwrap();
                        let vertex_format = match accessor.data_type() {
                            gltf::accessor::DataType::F32 => vk::Format::R32G32B32_SFLOAT,
                            _ => {
                                panic!("fuck");
                            }
                        };
                        let offset = (accessor.offset() + accessor.view().unwrap().offset()) as u64;
                        let index = accessor.view().unwrap().buffer().index();
                        let vertex_data = vk::DeviceOrHostAddressConstKHR {
                            device_address: buffers.get(index).unwrap().device_address() + offset,
                        };
                        let vertex_stride = match accessor.dimensions() {
                            gltf::accessor::Dimensions::Vec3 => {
                                std::mem::size_of::<f32>() as u64 * 3
                            }
                            _ => {
                                panic!("fuck");
                            }
                        };

                        (
                            vk::AccelerationStructureGeometryKHR::builder()
                                .geometry_type(vk::GeometryTypeKHR::TRIANGLES)
                                .flags(vk::GeometryFlagsKHR::OPAQUE)
                                .geometry(vk::AccelerationStructureGeometryDataKHR {
                                    triangles:
                                        vk::AccelerationStructureGeometryTrianglesDataKHR::builder()
                                            .index_type(index_type)
                                            .index_data(index_data)
                                            .vertex_data(vertex_data)
                                            .vertex_format(vertex_format)
                                            .vertex_stride(vertex_stride)
                                            .max_vertex(std::u32::MAX)
                                            .build(),
                                })
                                .build(),
                            (primitive.indices().unwrap().count() / 3) as u32,
                        )
                    })
                    .collect::<Vec<_>>()
                    .into_iter()
                    .unzip();
                safe_vk::AccelerationStructure::new(
                    Some("bottom level - mesh"),
                    allocator.clone(),
                    geometries.as_ref(),
                    triangle_count.as_ref(),
                    vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL,
                )
            })
            .collect::<Vec<_>>();

        Self {
            doc,
            buffers,
            images,
            bottom_level_acceleration_structures,
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
