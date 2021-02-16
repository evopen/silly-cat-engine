#![allow(unused)]

use std::convert::TryInto;
use std::path::Path;
use std::sync::Arc;
use std::unimplemented;

use bytemuck::cast_slice;
use glam::u32;
use safe_vk::vk;

pub struct Scene {
    doc: gltf::Document,
    buffers: Vec<safe_vk::Buffer>,
    images: Vec<safe_vk::Image>,
    bottom_level_acceleration_structures: Vec<safe_vk::AccelerationStructure>,
    top_level_acceleration_structure: safe_vk::AccelerationStructure,
    instance_buffers: Vec<safe_vk::Buffer>,
    allocator: Arc<safe_vk::Allocator>,
    queue: safe_vk::Queue,
    command_pool: Arc<safe_vk::CommandPool>,
    pointer_buffer: safe_vk::Buffer,
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

        let instance_buffers: Vec<safe_vk::Buffer> = scene
            .nodes()
            .map(|node| {
                Self::process_node(
                    node,
                    bottom_level_acceleration_structures.as_slice(),
                    allocator.clone(),
                    &mut queue,
                    command_pool.clone(),
                )
            })
            .flatten()
            .collect();

        let instance_buffer_addresses = instance_buffers
            .iter()
            .map(|buffer| buffer.device_address())
            .collect::<Vec<_>>();

        let pointer_buffer = safe_vk::Buffer::new_init_device(
            Some("pointer buffer"),
            allocator.clone(),
            vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            safe_vk::MemoryUsage::GpuOnly,
            &mut queue,
            command_pool.clone(),
            bytemuck::cast_slice(&instance_buffer_addresses),
        );

        let instance_geometry = vk::AccelerationStructureGeometryKHR::builder()
            .geometry_type(vk::GeometryTypeKHR::INSTANCES)
            .flags(vk::GeometryFlagsKHR::OPAQUE)
            .geometry(vk::AccelerationStructureGeometryDataKHR {
                instances: vk::AccelerationStructureGeometryInstancesDataKHR::builder()
                    .array_of_pointers(true)
                    .data(vk::DeviceOrHostAddressConstKHR {
                        device_address: pointer_buffer.device_address(),
                    })
                    .build(),
            })
            .build();

        let top_level_acceleration_structure = safe_vk::AccelerationStructure::new(
            Some("top level - mesh"),
            allocator.clone(),
            &[instance_geometry],
            &[instance_buffer_addresses.len() as u32],
            vk::AccelerationStructureTypeKHR::TOP_LEVEL,
        );

        Self {
            doc,
            buffers,
            images,
            bottom_level_acceleration_structures,
            instance_buffers,
            allocator,
            queue,
            command_pool,
            top_level_acceleration_structure,
            pointer_buffer,
        }
    }

    fn process_node(
        node: gltf::Node,
        bottoms: &[safe_vk::AccelerationStructure],
        allocator: Arc<safe_vk::Allocator>,
        queue: &mut safe_vk::Queue,
        command_pool: Arc<safe_vk::CommandPool>,
    ) -> Vec<safe_vk::Buffer> {
        let transform = glam::Mat4::from_cols_array_2d(&node.transform().matrix());

        let mut arr = node
            .children()
            .map(|node| {
                Self::process_node(
                    node,
                    bottoms,
                    allocator.clone(),
                    queue,
                    command_pool.clone(),
                )
            })
            .flatten()
            .collect::<Vec<_>>();

        if let Some(mesh) = node.mesh() {
            let instance = vk::AccelerationStructureInstanceKHR {
                transform: vk::TransformMatrixKHR {
                    matrix: transform.transpose().as_ref()[..12].try_into().unwrap(),
                },
                instance_custom_index_and_mask: 0 | (0xFF << 24),
                instance_shader_binding_table_record_offset_and_flags: 0 | (0x01 << 24),
                acceleration_structure_reference: vk::AccelerationStructureReferenceKHR {
                    device_handle: bottoms[mesh.index()].device_address(),
                },
            };

            let data = unsafe {
                std::slice::from_raw_parts(
                    std::mem::transmute(&instance),
                    std::mem::size_of::<vk::AccelerationStructureInstanceKHR>(),
                )
            };

            let instance_buffer = safe_vk::Buffer::new_init_device(
                Some("instance buffer"),
                allocator.clone(),
                vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                    | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
                safe_vk::MemoryUsage::GpuOnly,
                queue,
                command_pool.clone(),
                data,
            );

            arr.push(instance_buffer);
        }
        arr
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
