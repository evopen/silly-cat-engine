use std::convert::TryInto;
use std::f32::consts::FRAC_PI_6;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;

use glam::{vec3, Mat4, Vec3};
use rand::{Rng, SeedableRng};
use safe_vk::{vk, MemoryUsage};

struct Geometry {
    index_type: vk::IndexType,
    index_buffer_offset: u64,
    index_buffer_address: u64,
    vertex_format: vk::Format,
    vertex_buffer_offset: u64,
    vertex_buffer_address: u64,
    vertex_stride: u64,
    triangle_count: u32,
}

struct Mesh {
    geometries: Vec<Geometry>,
    blas: safe_vk::AccelerationStructure,
}

pub struct Scene {
    doc: gltf::Document,
    buffers: Vec<Arc<safe_vk::Buffer>>,
    // images: Vec<safe_vk::Image>,
    top_level_acceleration_structure: Arc<safe_vk::AccelerationStructure>,
    instance_buffers: Vec<safe_vk::Buffer>,
    allocator: Arc<safe_vk::Allocator>,
    queue: safe_vk::Queue,
    command_pool: Arc<safe_vk::CommandPool>,
    pointer_buffer: safe_vk::Buffer,
    meshes: Vec<Mesh>,
}

impl Scene {
    pub fn from_file<I: AsRef<Path>>(allocator: Arc<safe_vk::Allocator>, path: I) -> Self {
        let mut queue = safe_vk::Queue::new(allocator.device().clone());
        let command_pool = Arc::new(safe_vk::CommandPool::new(allocator.device().clone()));
        let (doc, gltf_buffers, gltf_images) = gltf::import(path).unwrap();

        let buffers = gltf_buffers
            .iter()
            .map(|data| {
                Arc::new(safe_vk::Buffer::new_init_host(
                    Some("gltf buffer"),
                    allocator.clone(),
                    vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                        | vk::BufferUsageFlags::STORAGE_BUFFER,
                    safe_vk::MemoryUsage::CpuToGpu,
                    data.as_ref(),
                ))
            })
            .collect::<Vec<_>>();

        let images = gltf_images
            .iter()
            .map(|image| {
                match image.format {
                    gltf::image::Format::R8G8B8 => {
                        let mut rgba_data: Vec<u8> =
                            Vec::with_capacity((image.width * image.height * 4) as usize);
                        for i in 0..image.pixels.len() {
                            rgba_data.push(image.pixels[i]);
                            if (i + 1) % 3 == 0 {
                                rgba_data.push(std::u8::MAX);
                            }
                        }
                        safe_vk::Image::new_init_host(
                            Some("gltf texture"),
                            allocator.clone(),
                            vk::Format::R8G8B8A8_UNORM,
                            image.width,
                            image.height,
                            vk::ImageTiling::LINEAR,
                            vk::ImageUsageFlags::SAMPLED,
                            safe_vk::MemoryUsage::CpuToGpu,
                            &mut queue,
                            command_pool.clone(),
                            &rgba_data,
                        )
                    }
                    gltf::image::Format::R8G8B8A8 => {
                        safe_vk::Image::new_init_host(
                            Some("gltf texture"),
                            allocator.clone(),
                            vk::Format::R8G8B8A8_UNORM,
                            image.width,
                            image.height,
                            vk::ImageTiling::OPTIMAL,
                            vk::ImageUsageFlags::SAMPLED,
                            safe_vk::MemoryUsage::CpuToGpu,
                            &mut queue,
                            command_pool.clone(),
                            &image.pixels,
                        )
                    }
                    _ => {
                        unimplemented!()
                    }
                };
            })
            .collect::<Vec<_>>();

        assert_eq!(doc.scenes().len(), 1);

        let scene = doc.scenes().next().unwrap();

        let mut meshes = Vec::with_capacity(doc.meshes().count());
        for mesh in doc.meshes() {
            let mut geometries = Vec::with_capacity(mesh.primitives().count());
            for primitive in mesh.primitives() {
                let index_accessor = primitive.indices().expect("unsupported");
                let index_type = match index_accessor.data_type() {
                    gltf::accessor::DataType::U16 => vk::IndexType::UINT16,
                    gltf::accessor::DataType::U32 => vk::IndexType::UINT32,
                    _ => {
                        panic!("not supported");
                    }
                };
                let index_buffer_offset =
                    (index_accessor.offset() + index_accessor.view().unwrap().offset()) as u64;
                let index_buffer_index = index_accessor.view().unwrap().buffer().index();
                let index_buffer_address =
                    buffers.get(index_buffer_index).unwrap().device_address();
                let index_device_address = vk::DeviceOrHostAddressConstKHR {
                    device_address: index_buffer_address + index_buffer_offset,
                };
                let (_, vertex_accessor) = primitive
                    .attributes()
                    .find(|(semantic, _)| semantic.eq(&gltf::Semantic::Positions))
                    .unwrap();
                let vertex_format = match vertex_accessor.data_type() {
                    gltf::accessor::DataType::F32 => vk::Format::R32G32B32_SFLOAT,
                    _ => {
                        panic!("fuck");
                    }
                };
                let vertex_buffer_offset =
                    (vertex_accessor.offset() + vertex_accessor.view().unwrap().offset()) as u64;
                let vertex_buffer_index = vertex_accessor.view().unwrap().buffer().index();
                let vertex_buffer_address =
                    buffers.get(vertex_buffer_index).unwrap().device_address();
                let vertex_device_address = vk::DeviceOrHostAddressConstKHR {
                    device_address: vertex_buffer_address + vertex_buffer_offset,
                };
                let vertex_stride = match vertex_accessor.dimensions() {
                    gltf::accessor::Dimensions::Vec3 => std::mem::size_of::<f32>() as u64 * 3,
                    _ => {
                        panic!("fuck");
                    }
                };
                let triangle_count = index_accessor.count() as u32 / 3;

                geometries.push(Geometry {
                    index_type,
                    index_buffer_offset,
                    index_buffer_address,
                    vertex_format,
                    vertex_buffer_offset,
                    vertex_buffer_address,
                    vertex_stride,
                    triangle_count,
                });
            }
            let blas = safe_vk::AccelerationStructure::new(
                Some("bottom level - mesh"),
                allocator.clone(),
                geometries
                    .iter()
                    .map(|geometry| {
                        vk::AccelerationStructureGeometryKHR::builder()
                            .geometry_type(vk::GeometryTypeKHR::TRIANGLES)
                            .flags(
                                vk::GeometryFlagsKHR::OPAQUE
                                    | vk::GeometryFlagsKHR::NO_DUPLICATE_ANY_HIT_INVOCATION,
                            )
                            .geometry(vk::AccelerationStructureGeometryDataKHR {
                                triangles:
                                    vk::AccelerationStructureGeometryTrianglesDataKHR::builder()
                                        .index_type(geometry.index_type)
                                        .index_data(vk::DeviceOrHostAddressConstKHR {
                                            device_address: buffers[0].device_address()
                                                + geometry.index_buffer_offset,
                                        })
                                        .vertex_data(vk::DeviceOrHostAddressConstKHR {
                                            device_address: buffers[0].device_address()
                                                + geometry.vertex_buffer_offset,
                                        })
                                        .vertex_format(geometry.vertex_format)
                                        .vertex_stride(geometry.vertex_stride)
                                        .max_vertex(std::u32::MAX)
                                        .build(),
                            })
                            .build()
                    })
                    .collect::<Vec<_>>()
                    .as_slice(),
                geometries
                    .iter()
                    .map(|geometry| geometry.triangle_count)
                    .collect::<Vec<_>>()
                    .as_slice(),
                vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL,
            );
            meshes.push(Mesh { geometries, blas });
        }

        let instance_buffers: Vec<safe_vk::Buffer> = scene
            .nodes()
            .map(|node| {
                Self::process_node(
                    node,
                    meshes.as_slice(),
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

        let top_level_acceleration_structure = Arc::new(safe_vk::AccelerationStructure::new(
            Some("top level - mesh"),
            allocator.clone(),
            &[instance_geometry],
            &[instance_buffer_addresses.len() as u32],
            vk::AccelerationStructureTypeKHR::TOP_LEVEL,
        ));

        Self {
            doc,
            buffers,
            // images,
            instance_buffers,
            allocator,
            queue,
            command_pool,
            top_level_acceleration_structure,
            pointer_buffer,
            meshes,
        }
    }

    fn process_node(
        node: gltf::Node,
        meshes: &[Mesh],
        allocator: Arc<safe_vk::Allocator>,
        queue: &mut safe_vk::Queue,
        command_pool: Arc<safe_vk::CommandPool>,
    ) -> Vec<safe_vk::Buffer> {
        let orig_transform = Mat4::from_cols_array_2d(&node.transform().matrix());

        let mut rng = rand::rngs::SmallRng::from_entropy();

        let mut arr = Vec::new();

        if let Some(mesh) = node.mesh() {
            let instance = vk::AccelerationStructureInstanceKHR {
                transform: vk::TransformMatrixKHR {
                    matrix: orig_transform.transpose().as_ref()[..12]
                        .try_into()
                        .unwrap(),
                },
                instance_custom_index_and_mask: 0 | (0xFF << 24),
                instance_shader_binding_table_record_offset_and_flags: rng.gen_range(0..=4)
                    | (vk::GeometryInstanceFlagsKHR::TRIANGLE_FACING_CULL_DISABLE.as_raw() << 24),
                acceleration_structure_reference: vk::AccelerationStructureReferenceKHR {
                    device_handle: meshes[mesh.index()].blas.device_address(),
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

    pub fn tlas(&self) -> &Arc<safe_vk::AccelerationStructure> {
        &self.top_level_acceleration_structure
    }

    pub fn sole_buffer(&self) -> &Arc<safe_vk::Buffer> {
        assert_eq!(self.buffers.len(), 1);
        &self.buffers[0]
    }

    pub fn sole_geometry_index_buffer_offset(&self) -> u64 {
        self.meshes[0].geometries[0].index_buffer_offset
    }
    pub fn sole_geometry_vertex_buffer_offset(&self) -> u64 {
        self.meshes[0].geometries[0].vertex_buffer_offset
    }
}
