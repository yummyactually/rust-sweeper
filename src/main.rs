#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod app;
mod win32;

fn main() {
    if let Err(error) = app::run() {
        win32::show_error(&error);
        std::process::exit(1);
    }
}
