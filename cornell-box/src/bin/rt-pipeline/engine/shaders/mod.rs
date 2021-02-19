use rust_embed::RustEmbed;
#[derive(RustEmbed)]
#[folder = "./src/bin/rt-pipeline/engine/shaders/bin"]
pub(super) struct Shaders;
