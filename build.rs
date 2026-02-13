fn main() {
    // Only compile Windows resources on Windows target
    #[cfg(target_os = "windows")]
    {
        // Embed the Windows resource file (contains app icon and tray icon)
        let _ = embed_resource::compile("resources/windows/resources.rc", embed_resource::NONE);
    }
}
