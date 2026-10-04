pub mod actor;
#[cfg(feature = "host")]
mod clipboard;
#[cfg(feature = "host")]
pub mod host;
#[cfg(any(feature = "host", test))]
mod preferences;

#[cfg(all(feature = "host", target_os = "macos"))]
mod macos_exit;
