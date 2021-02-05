use super::shader::ShaderStage;
use super::Vulkan;
use anyhow::Result;
use ash::version::DeviceV1_0;
use ash::vk;
use std::ffi::{CStr, CString};
use std::sync::Arc;

pub struct Sampler {
    handle: vk::Sampler,
    vulkan: Arc<Vulkan>,
}

impl Sampler {
    pub fn new(vulkan: Arc<Vulkan>) -> Result<Self> {
        let info = vk::SamplerCreateInfo::builder()
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .build();
        unsafe {
            let handle = vulkan.device.create_sampler(&info, None)?;
            Ok(Self { handle, vulkan })
        }
    }
}

impl Drop for Sampler {
    fn drop(&mut self) {
        unsafe {
            self.vulkan.device.destroy_sampler(self.handle, None);
        }
    }
}

pub struct DescriptorSetLayout {
    handle: vk::DescriptorSetLayout,
    vulkan: Arc<Vulkan>,
}

impl DescriptorSetLayout {
    pub fn new(vulkan: Arc<Vulkan>, bindings: &[vk::DescriptorSetLayoutBinding]) -> Result<Self> {
        let info = vk::DescriptorSetLayoutCreateInfo::builder()
            .bindings(bindings)
            .build();
        unsafe {
            let handle = vulkan.device.create_descriptor_set_layout(&info, None)?;
            Ok(Self { handle, vulkan })
        }
    }
}

impl Drop for DescriptorSetLayout {
    fn drop(&mut self) {
        unsafe {
            self.vulkan
                .device
                .destroy_descriptor_set_layout(self.handle, None);
        }
    }
}

pub struct PipelineLayout {
    handle: vk::PipelineLayout,
    vulkan: Arc<Vulkan>,
}

impl PipelineLayout {
    pub fn new(vulkan: Arc<Vulkan>, set_layouts: &[&DescriptorSetLayout]) -> Result<Self> {
        let set_layouts = set_layouts
            .iter()
            .map(|layout| layout.handle)
            .collect::<Vec<_>>();
        let info = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(set_layouts.as_slice())
            .build();
        unsafe {
            let handle = vulkan.device.create_pipeline_layout(&info, None)?;
            Ok(Self { handle, vulkan })
        }
    }
}

impl Drop for PipelineLayout {
    fn drop(&mut self) {
        unsafe {
            self.vulkan
                .device
                .destroy_pipeline_layout(self.handle, None);
        }
    }
}

pub struct GraphicsPipeline {
    handle: vk::Pipeline,
    vulkan: Arc<Vulkan>,
}

impl GraphicsPipeline {
    pub fn new(
        vulkan: Arc<Vulkan>,
        pipeline_layout: &PipelineLayout,
        stages: &[&ShaderStage],
        vertex_input_state: &vk::PipelineVertexInputStateCreateInfo,
        input_assembly_state: &vk::PipelineInputAssemblyStateCreateInfo,
        rasterization_state: &vk::PipelineRasterizationStateCreateInfo,
        multisample_state: &vk::PipelineMultisampleStateCreateInfo,
        depth_stencil_state: &vk::PipelineDepthStencilStateCreateInfo,
        color_blend_state: &vk::PipelineColorBlendStateCreateInfo,
    ) -> Result<Self> {
        let shader_stages = stages
            .iter()
            .map(|stage| stage.shader_stage_create_info().clone())
            .collect::<Vec<_>>();
        let info = vk::GraphicsPipelineCreateInfo::builder()
            .layout(pipeline_layout.handle)
            .stages(shader_stages.as_slice())
            .vertex_input_state(vertex_input_state)
            .input_assembly_state(input_assembly_state)
            .rasterization_state(rasterization_state)
            .multisample_state(multisample_state)
            .depth_stencil_state(depth_stencil_state)
            .color_blend_state(color_blend_state)
            .build();
        unsafe {
            let handle = vulkan
                .device
                .create_graphics_pipelines(vk::PipelineCache::null(), &[info], None)
                .unwrap()
                .first()
                .unwrap()
                .to_owned();
            Ok(Self { handle, vulkan })
        }
    }
}

impl Drop for GraphicsPipeline {
    fn drop(&mut self) {
        unsafe {
            self.vulkan.device.destroy_pipeline(self.handle, None);
        }
    }
}

pub struct ShaderModule {
    handle: vk::ShaderModule,
    vulkan: Arc<Vulkan>,
}

#[repr(C, align(32))]
struct AlignedSpirv {
    pub code: Vec<u8>,
}

impl ShaderModule {
    pub fn new<P>(vulkan: Arc<Vulkan>, spv: P) -> Result<Self>
    where
        P: AsRef<[u8]>,
    {
        let aligned = AlignedSpirv {
            code: spv.as_ref().to_vec(),
        };
        let info = vk::ShaderModuleCreateInfo::builder()
            .code(bytemuck::cast_slice(aligned.code.as_slice()))
            .build();
        unsafe {
            let handle = vulkan.device.create_shader_module(&info, None)?;
            Ok(Self { handle, vulkan })
        }
    }
}

impl Drop for ShaderModule {
    fn drop(&mut self) {
        unsafe {
            self.vulkan.device.destroy_shader_module(self.handle, None);
        }
    }
}

pub struct ShaderStage {
    module: ShaderModule,
    stage: vk::ShaderStageFlags,
    entry_point: CString,
    shader_stage_create_info: vk::PipelineShaderStageCreateInfo,
}

impl ShaderStage {
    pub fn new(module: ShaderModule, stage: vk::ShaderStageFlags, entry_point: &str) -> Self {
        let entry_point = CString::new(entry_point).unwrap();
        Self {
            module,
            stage,
            entry_point: CString::new(entry_point).unwrap(),
            shader_stage_create_info: vk::PipelineShaderStageCreateInfo::builder()
                .stage(stage)
                .module(module.handle)
                .name(entry_point.as_ref())
                .build(),
        }
    }

    pub fn shader_stage_create_info(&self) -> &vk::PipelineShaderStageCreateInfo {
        &self.shader_stage_create_info
    }
}

pub struct DescriptorSet {
    handle: vk::DescriptorSet,
    vulkan: Arc<Vulkan>,
}

impl DescriptorSet {
    pub fn new(
        vulkan: Arc<Vulkan>,
        descriptor_pool: &DescriptorPool,
        descriptor_set_layout: &DescriptorSetLayout,
    ) -> Result<Self> {
        let info = vk::DescriptorSetAllocateInfo::builder()
            .set_layouts(&[descriptor_set_layout.handle])
            .build();
        unsafe {
            let handle = vulkan.device.allocate_descriptor_sets(&info)?;
            Ok(Self { handle, vulkan })
        }
    }
}

impl Drop for DescriptorSet {
    fn drop(&mut self) {
        unsafe {
            self.vulkan.device.free_descriptor_sets(self.handle, None);
        }
    }
}

pub struct DescriptorPool {
    handle: vk::DescriptorPool,
    vulkan: Arc<Vulkan>,
}

impl DescriptorPool {
    pub fn new(
        vulkan: Arc<Vulkan>,
        descriptor_pool_size: &[vk::DescriptorPoolSize],
        max_sets: u32,
    ) -> Result<Self> {
        let info = vk::DescriptorPoolCreateInfo::builder()
            .pool_sizes(descriptor_pool_size)
            .max_sets(max_sets)
            .build();
        unsafe {
            let handle = vulkan.device.create_descriptor_pool(&info, None)?;
            Ok(Self { handle, vulkan })
        }
    }
}

impl Drop for DescriptorPool {
    fn drop(&mut self) {
        unsafe {
            self.vulkan
                .device
                .destroy_descriptor_pool(self.handle, None);
        }
    }
}
