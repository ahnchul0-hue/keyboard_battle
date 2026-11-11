// 배포판에서는 콘솔 창을 띄우지 않는다.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;

use std::sync::Arc;

use eframe::egui::{self, FontData, FontDefinitions, FontFamily};

const FONT_NAME: &str = "D2Coding";
static FONT: &[u8] = include_bytes!("../assets/fonts/D2Coding-Regular.ttf");

/// 한글이 들어 있는 글꼴을 실행 파일에 넣어 두고, 모든 글에 먼저 쓴다.
fn install_font(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts
        .font_data
        .insert(FONT_NAME.to_owned(), Arc::new(FontData::from_static(FONT)));
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .insert(0, FONT_NAME.to_owned());
    }
    ctx.set_fonts(fonts);
}

/// 콘솔이 없어 오류가 보이지 않으므로, 실행 파일 옆에 적어 둔다.
fn report_failure(message: &str) {
    eprintln!("{message}");
    if let Ok(exe) = std::env::current_exe() {
        // 적지 못해도 더 할 수 있는 일이 없다.
        let _ = std::fs::write(exe.with_file_name("keyboard_battle_error.txt"), message);
    }
}

fn main() {
    std::panic::set_hook(Box::new(|panic| report_failure(&panic.to_string())));

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("키보드 배틀")
            .with_inner_size([960.0, 640.0])
            .with_min_inner_size([720.0, 480.0]),
        ..Default::default()
    };
    let result = eframe::run_native(
        "keyboard_battle",
        options,
        Box::new(|creation| {
            install_font(&creation.egui_ctx);
            Ok(Box::new(app::CheckApp::default()))
        }),
    );
    if let Err(error) = result {
        report_failure(&format!("창을 열지 못했습니다: {error}"));
        std::process::exit(1);
    }
}
