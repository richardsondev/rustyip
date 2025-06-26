use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=icon/rustyip.ico");
    println!("cargo:rerun-if-changed=Cargo.toml");
    
    // Embed Windows resources (icon, version info, etc.)
    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        
        // Set icon if available
        if Path::new("icon/rustyip.ico").exists() {
            res.set_icon("icon/rustyip.ico");
            println!("cargo:warning=✅ Embedding Windows icon: icon/rustyip.ico");
        } else {
            println!("cargo:warning=⚠️  Icon not found at icon/rustyip.ico");
            println!("cargo:warning=Run ./icon/svg-import.sh to generate icons");
        }

        // Set application metadata
        res.set("ProductName", "RustyIP Client");
        res.set("FileDescription", "RustyIP - Dynamic DNS client"); 
        res.set("LegalCopyright", "Copyright (C) Billy Richardson");
        res.set("CompanyName", "richardsondev");
        res.set("ProductVersion", env!("CARGO_PKG_VERSION"));
        res.set("FileVersion", env!("CARGO_PKG_VERSION"));
        res.set("OriginalFilename", "RustyIP.exe");
        res.set("InternalName", "RustyIP");
        res.set("Comments", "GitHub: https://github.com/richardsondev/rustyip");
        
        // Compile the resources
        if let Err(e) = res.compile() {
            println!("cargo:warning=❌ Failed to embed Windows resources: {}", e);
        } else {
            println!("cargo:warning=✅ Windows resources embedded successfully");
        }
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        println!("cargo:warning=Skipping Windows resource embedding on non-Windows platform");
    }
}
