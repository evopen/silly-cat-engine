use rust_embed::RustEmbed;
#[derive(RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR/shaders/bin"]
pub(super) struct Shaders;
