//! 화면을 어떻게 보여 줄지에 대한 선택.
//!
//! 아직 저장하지 않으므로, 프로그램을 다시 켜면 처음 값으로 돌아간다.

use eframe::egui::{self, RichText};

use crate::theme::Palette;

/// 지금 칠 낱말을 알려 주는 방식.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Marker {
    /// 낱말 아래에 줄을 긋는다.
    #[default]
    Underline,
    /// 낱말의 바탕을 칠한다.
    Block,
    /// 표시하지 않는다.
    None,
}

/// 친 글을 보여 주는 자리.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Echo {
    /// 칠 문장 아래 줄에 따로 보인다.
    #[default]
    Below,
    /// 칠 문장 위에 겹쳐, 친 글자에 색을 입힌다.
    Over,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub marker: Marker,
    pub echo: Echo,
    /// 치는 동안 제목·단추·도움말을 흐리게 물릴지.
    pub focus: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            marker: Marker::default(),
            echo: Echo::default(),
            focus: true,
        }
    }
}

/// 선택지 한 줄: 이름과, 그 가운데 하나를 고르는 단추들.
fn choice<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    palette: &Palette,
    name: &str,
    value: &mut T,
    options: &[(T, &str)],
) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(name).color(palette.dim));
        for &(option, label) in options {
            ui.selectable_value(value, option, label);
        }
    });
}

/// 설정을 고르는 곳.
pub fn ui(ui: &mut egui::Ui, palette: &Palette, settings: &mut Settings) {
    choice(
        ui,
        palette,
        "지금 칠 낱말",
        &mut settings.marker,
        &[
            (Marker::Underline, "밑줄"),
            (Marker::Block, "바탕"),
            (Marker::None, "표시 없음"),
        ],
    );
    choice(
        ui,
        palette,
        "친 글",
        &mut settings.echo,
        &[(Echo::Below, "아래 줄에"), (Echo::Over, "문장 위에 겹쳐")],
    );
    choice(
        ui,
        palette,
        "집중 화면",
        &mut settings.focus,
        &[(true, "치는 동안 주변을 흐리게"), (false, "끔")],
    );
}
