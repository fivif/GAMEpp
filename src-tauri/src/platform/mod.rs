#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(target_os = "windows")]
pub use windows as current;

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "macos")]
pub use macos as current;

/// Set system-wide HTTP/HTTPS/SOCKS proxy
pub fn set_system_proxy(host: &str, port: u16) -> anyhow::Result<()> {
    current::set_system_proxy(host, port)
}

/// Disable system-wide proxy
pub fn disable_system_proxy() -> anyhow::Result<()> {
    current::disable_system_proxy()
}
