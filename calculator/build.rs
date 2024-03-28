#[cfg(not(feature = "slint-default"))]
fn main() {
    coop::import_paths();
    let config = slint_build::CompilerConfiguration::new()
        .embed_resources(slint_build::EmbedResourcesKind::EmbedForSoftwareRenderer);
    slint_build::compile_with_config("ui/app.slint", config).unwrap();
    slint_build::print_rustc_flags().unwrap();
}

#[cfg(feature = "slint-default")]
fn main() { 
    coop::import_paths();
    slint_build::compile("ui/app.slint").unwrap();
}
