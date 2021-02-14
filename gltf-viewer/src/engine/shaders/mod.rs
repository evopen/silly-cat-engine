use rust_embed::RustEmbed;
#[derive(RustEmbed)]
#[folder = "./src/engine/shaders/bin"]
pub(super) struct Shaders;
