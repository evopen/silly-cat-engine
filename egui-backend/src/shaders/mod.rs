use rust_embed::RustEmbed;
#[derive(RustEmbed)]
#[folder = "src/shaders/bin"]
pub(super) struct Shaders;
