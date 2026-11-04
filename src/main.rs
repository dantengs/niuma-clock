#![cfg_attr(not(test), windows_subsystem = "windows")]

mod app;
mod engine;
mod icon;
mod model;
mod pages;
mod platform;
mod quotes;
mod salary;
mod store;
mod widgets;
mod windows;

use std::sync::{Arc, Mutex};

use eframe::egui;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // 打包脚本用：把图标写进另一个 exe。
    if args.first().map(String::as_str) == Some("--stamp-icon") {
        let Some(target) = args.get(1) else {
            std::process::exit(2);
        };
        if let Err(reason) = platform::stamp_icon(std::path::Path::new(target), &icon::ico_images()) {
            platform::error_box(&reason);
            std::process::exit(1);
        }
        return;
    }

    let dir = store::data_dir();
    let log_dir = dir.clone();
    std::panic::set_hook(Box::new(move |info| {
        // 窗口程序没有控制台，崩溃信息写进文件才查得到。
        let _ = std::fs::create_dir_all(&log_dir);
        let _ = std::fs::write(log_dir.join("panic.log"), format!("{info}\n"));
    }));

    let Some(show_signal) = platform::acquire_single_instance(&dir) else {
        return;
    };

    let start_hidden = args.iter().any(|a| a == "--minimized");
    let preview_effect = args.iter().any(|a| a == "--preview-effect");
    let engine = Arc::new(Mutex::new(engine::Engine::new(
        store::load_config(&dir),
        store::load_data(&dir),
    )));

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("牛马上下班")
            .with_app_id("NiuMa.Clock")
            .with_inner_size([620.0, 800.0])
            .with_min_inner_size([560.0, 560.0])
            .with_icon(egui::IconData {
                rgba: icon::rgba(64),
                width: 64,
                height: 64,
            })
            // 悬浮窗要透明，而透明能力由根窗口的 GL 配置决定。
            .with_transparent(true)
            .with_visible(!start_hidden),
        centered: true,
        glow_options: eframe::egui_glow::GlowConfiguration {
            // 多个窗口各自等垂直同步会互相拖慢，帧率由程序自己控制。
            vsync: false,
            ..Default::default()
        },
        ..Default::default()
    };

    let result = eframe::run_native(
        "牛马上下班",
        options,
        Box::new(move |cc| Ok(Box::new(app::App::new(cc, engine, dir, start_hidden, preview_effect, show_signal)))),
    );
    if let Err(err) = result {
        platform::error_box(&format!("启动失败：{err}\n\n可能是显卡驱动不支持 OpenGL。"));
    }
}
