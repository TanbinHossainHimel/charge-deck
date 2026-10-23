// Windows only: embeds the app icon and version information (product name,
// description, copyright) into the .exe files. Shortcuts, the taskbar, Task
// Manager and the installer all show this icon. Version numbers come from
// Cargo.toml, so there is a single place to change them.

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    res.set_icon("assets/charge-deck.ico");
    res.set("ProductName", "Charge Deck");
    res.set("FileDescription", "Charge Deck - wireless battery levels in one window");
    res.set("LegalCopyright", "MIT License - Charge Deck contributors");
    if let Err(e) = res.compile() {
        // Don't break the build if the Windows resource compiler is missing:
        // the app still works, it just has no icon.
        println!("cargo:warning=could not embed icon/version info: {e}");
    }
}
