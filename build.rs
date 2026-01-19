extern crate embed_resource;

fn main() {
    #[cfg(target_os = "windows")]
    embed_resource::compile("icon.rc", embed_resource::NONE);
}
