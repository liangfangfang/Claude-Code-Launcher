fn main() {
    // Compile Windows resource file
    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("CompanyName", "akchth");
        res.set("FileDescription", "Claude Code Launcher");
        res.set("FileVersion", "2.2.1.0");
        res.set("InternalName", "claude-launcher");
        res.set("LegalCopyright", "Copyright (c) 2026 akchth");
        res.set("OriginalFilename", "claude-launcher.exe");
        res.set("ProductName", "Claude Code Launcher");
        res.set("ProductVersion", "2.2.1.0");
        res.compile().unwrap();
    }
}
