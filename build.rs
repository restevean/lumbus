fn main() {
    // Only compile Windows resources on Windows target
    #[cfg(target_os = "windows")]
    {
        // Embed the Windows resource file (contains tray icon)
        embed_resource::compile("resources/windows/resources.rc", embed_resource::NONE);
    }
}
