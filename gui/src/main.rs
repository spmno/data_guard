//! 桌面启动器：在本机原生后端上运行（Linux/GTK4，`cargo run`）。
//!
//! iOS / Android / Web 目标用 `nsh run --target …`（见仓库 README）。

#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[cfg(target_os = "linux")]
use nsh_platform_linux as platform;
#[cfg(target_os = "macos")]
use nsh_platform_macos as platform;
#[cfg(target_os = "windows")]
use nsh_platform_windows as platform;

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
fn main() -> impl std::process::Termination {
    env_logger::init();
    platform::run_app(
        platform::AppSource::Rust(platform::Root::of::<data_guard_gui::App>()),
        platform::AppOptions {
            title: "data_guard".to_owned(),
            #[cfg(any(target_os = "linux", target_os = "windows"))]
            app_id: "com.spmno.data_guard".to_owned(),
            size: (900, 640),
            ..Default::default()
        },
    )
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
compile_error!(
    "Run this binary on macOS, Windows or Linux; use nsh run for iOS, Android and the web."
);
