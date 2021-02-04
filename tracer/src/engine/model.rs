use anyhow::Result;

use core::panic;

use std::sync::Arc;

use ash::vk;

use super::buffer::Buffer;
use super::Vulkan;

struct Primitive {}

struct Mesh {}

impl Mesh {}

pub struct Model {
    buffers: Vec<Buffer>,
    geometries: Vec<vk::AccelerationStructureGeometryKHR>,
    model: gltf::Gltf,
    geometries_triangle_count: u32,
}

impl Model {
    pub fn new(model: &gltf::Gltf, vulkan: Arc<Vulkan>) -> Result<Self> {
        let mut buffers = Vec::with_capacity(model.buffers().len());
        for gltf_buffer in model.buffers() {
            let buffer = Buffer::new(
                gltf_buffer.length(),
                vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                    | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
                vk_mem::MemoryUsage::CpuToGpu,
                vulkan.clone(),
            )?;
            match gltf_buffer.source() {
                gltf::buffer::Source::Bin => {
                    let bin = model.blob.as_ref().unwrap().as_slice();
                    buffer.copy_from(bin.as_ptr())?;
                }
                gltf::buffer::Source::Uri(_) => {
                    panic!("fuck")
                }
            }
            buffers.push(buffer);
        }
        dbg!(&buffers.len());

        let geometries: Vec<Vec<vk::AccelerationStructureGeometryKHR>> = model
            .meshes()
            .map(|mesh| {
                mesh.primitives()
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
                                            .unwrap()
                                            + offset,
                                    },
                                )
                            }
                            None => (
                                vk::IndexType::NONE_KHR,
                                vk::DeviceOrHostAddressConstKHR::default(),
                            ),
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
                            device_address: buffers.get(index).unwrap().device_address().unwrap()
                                + offset,
                        };
                        let vertex_stride = match accessor.dimensions() {
                            gltf::accessor::Dimensions::Vec3 => {
                                std::mem::size_of::<f32>() as u64 * 3
                            }
                            _ => {
                                panic!("fuck");
                            }
                        };

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
                            .build()
                    })
                    .collect()
            })
            .collect();

        let geometries_triangle_count = model.meshes().fold(0, |_acc, mesh| {
            mesh.primitives().fold(0, |_acc, prim| {
                let indices = prim.indices().unwrap();
                indices.count() / 3
            })
        }) as u32;

        Ok(Self {
            buffers,
            geometries: geometries.into_iter().flatten().collect(),
            model: model.clone(),
            geometries_triangle_count,
        })
    }

    pub fn geometries(&self) -> &[vk::AccelerationStructureGeometryKHR] {
        self.geometries.as_slice()
    }

    pub fn geometry_triangle_count(&self) -> u32 {
        self.geometries_triangle_count
    }
}

fn process_node(node: &gltf::Node) {
    for node in node.children() {
        process_node(&node);
        let _transform = glam::Mat4::from_cols_array_2d(&node.transform().matrix());
        if let Some(mesh) = node.mesh() {
            for _primitive in mesh.primitives() {}
        }
    }
}
