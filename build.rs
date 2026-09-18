fn main() {
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/brand/icons/alya-dark.ico");
        res.set("FileDescription", "Alya Compiler");
        res.set("ProductName", "Alya");
        res.set("OriginalFilename", "alya.exe");
        if let Err(e) = res.compile() {
            eprintln!("cargo:warning=Failed to compile Windows resource: {}", e);
        }
    }
}
