// 배포판에서는 콘솔 창을 띄우지 않는다.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod input;
mod result;
mod settings;
mod theme;
mod view;

use std::sync::Arc;

use eframe::egui::{self, FontData, FontDefinitions, FontFamily};
use keyboard_battle::texts::Deck;

/// 문장을 섞지 않고 실린 순서대로 내게 하는 표시. 시연이나 확인처럼 늘 같은 문장이 필요할 때 쓴다.
/// 창으로 열 때는 실행 인자 `--in-order`로, 브라우저에서는 주소 끝의 `#in-order`로 준다.
const IN_ORDER: &str = "in-order";

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

/// 켤 때마다 달라지는 수. 문장을 섞는 씨앗으로 쓴다.
#[cfg(not(target_arch = "wasm32"))]
fn seed() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    // 시계가 1970년보다 앞에 맞춰져 있으면 0을 쓴다. 섞이는 순서가 늘 같을 뿐 프로그램은 돈다.
    let since_epoch = SystemTime::now().duration_since(UNIX_EPOCH);
    since_epoch.map_or(0, |time| time.as_nanos() as u64)
}

/// 브라우저에서는 표준 라이브러리의 시계를 쓸 수 없어 브라우저의 난수를 쓴다.
#[cfg(target_arch = "wasm32")]
fn seed() -> u64 {
    (eframe::web_sys::js_sys::Math::random() * (1u64 << 53) as f64) as u64
}

/// 문장을 실린 순서대로 내 달라는 표시가 있는지.
fn wants_listed_order(creation: &eframe::CreationContext<'_>) -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = creation;
        std::env::args().any(|arg| arg.strip_prefix("--") == Some(IN_ORDER))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let hash = &creation.integration_info.web_info.location.hash;
        hash.strip_prefix('#') == Some(IN_ORDER)
    }
}

/// 창이나 웹 화면이 준비되면 글꼴과 색을 넣고 연습 화면을 만든다.
fn create_app(creation: &eframe::CreationContext<'_>) -> Box<dyn eframe::App> {
    install_font(&creation.egui_ctx);
    theme::install(&creation.egui_ctx);
    // 켤 때마다 다른 문장으로 시작한다.
    let deck = if wants_listed_order(creation) {
        Deck::in_order()
    } else {
        Deck::shuffled(seed())
    };
    Box::new(app::PracticeApp::new(deck))
}

/// 콘솔이 없어 오류가 보이지 않으므로, 실행 파일 옆에 적어 둔다.
#[cfg(not(target_arch = "wasm32"))]
fn report_failure(message: &str) {
    eprintln!("{message}");
    if let Ok(exe) = std::env::current_exe() {
        // 적지 못해도 더 할 수 있는 일이 없다.
        let _ = std::fs::write(exe.with_file_name("keyboard_battle_error.txt"), message);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    std::panic::set_hook(Box::new(|panic| report_failure(&panic.to_string())));

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("키보드 배틀")
            .with_inner_size([960.0, 700.0])
            .with_min_inner_size([720.0, 480.0]),
        ..Default::default()
    };
    let result = eframe::run_native(
        "keyboard_battle",
        options,
        Box::new(|creation| Ok(create_app(creation))),
    );
    if let Err(error) = result {
        report_failure(&format!("창을 열지 못했습니다: {error}"));
        std::process::exit(1);
    }
}

/// 브라우저에서는 `web/index.html`의 canvas에 화면을 그린다.
#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;
    use eframe::web_sys;

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window().and_then(|window| window.document());
        let element = |id: &str| document.as_ref().and_then(|d| d.get_element_by_id(id));
        let canvas = element("screen")
            .and_then(|canvas| canvas.dyn_into::<web_sys::HtmlCanvasElement>().ok())
            .expect("index.html에 id가 screen인 canvas가 있다");

        let started = eframe::WebRunner::new()
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(|creation| Ok(create_app(creation))),
            )
            .await;
        // 불러오는 중이라는 안내를 지우거나, 못 열었으면 그 까닭으로 바꾼다.
        if let Some(note) = element("note") {
            match started {
                Ok(()) => note.remove(),
                Err(error) => {
                    note.set_text_content(Some(&format!("화면을 열지 못했습니다: {error:?}")));
                }
            }
        }
    });
}
