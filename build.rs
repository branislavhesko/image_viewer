fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
    
    #[cfg(target_os = "linux")]
    {
        println!("cargo:rerun-if-changed=assets/icon.png");
        
        // Embed the icon into the executable
        let icon_path = "assets/icon.png";
        if std::path::Path::new(icon_path).exists() {
            println!("cargo:rustc-env=ICON_PATH={}", icon_path);
        }
    }
    
    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.png");
        res.compile().unwrap();
    }
}
