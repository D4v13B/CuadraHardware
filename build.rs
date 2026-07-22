fn main() {
    println!("cargo:rerun-if-changed=assets/app.ico");

    #[cfg(windows)]
    {
        let icon = std::path::Path::new("assets/app.ico");
        if icon.exists() {
            let mut resource = winresource::WindowsResource::new();
            resource.set_icon(icon.to_string_lossy().as_ref());
            resource.set("ProductName", "Cuadra POS Agent");
            resource.set("FileDescription", "Servicio local para Cuadra POS");
            resource.set("CompanyName", "Cuadra ERP");
            resource.set("LegalCopyright", "Copyright Cuadra ERP");
            resource.set("OriginalFilename", "cuadra-pos-agent.exe");
            resource
                .compile()
                .expect("no se pudo compilar el recurso de Windows");
        }
    }
}
