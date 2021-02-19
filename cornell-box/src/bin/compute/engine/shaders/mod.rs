use rust_embed::RustEmbed;
#[derive(RustEmbed)]
#[folder = "./src/bin/compute/engine/shaders/bin"]
pub(super) struct Shaders;
