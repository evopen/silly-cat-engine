use rust_embed::RustEmbed;
#[derive(RustEmbed)]
#[folder = "../target/spirv"]
pub(super) struct Shaders;

#[repr(C, align(32))]
pub(super) struct AlignedSpirv {
    pub code: Vec<u8>,
}
