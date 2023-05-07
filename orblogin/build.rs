#[cfg(not(feature = "slint-default"))]
fn main() {
    let config = slint_build::CompilerConfiguration::new()
        .embed_resources(slint_build::EmbedResourcesKind::EmbedForSoftwareRenderer);
    slint_build::compile_with_config("ui/login_window.slint", config).unwrap();
    slint_build::print_rustc_flags().unwrap();
}

#[cfg(feature = "slint-default")]
fn main() { 
    slint_build::compile("ui/login_window.slint").unwrap();
}