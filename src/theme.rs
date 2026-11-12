//! 화면 밝기(어두운 화면 · 밝은 화면)와 색, 글자 크기.
//!
//! egui 기본 색은 덜 중요한 글이 바탕에 묻혀 흐릿하게 보인다.
//! 어느 화면에서든 모든 글이 또렷이 읽히도록 색을 직접 정하고, 그 명암비를 테스트로 지킨다.

use eframe::egui::{self, Color32, FontId, Stroke, SystemTheme, TextStyle, Theme, ThemePreference};

/// 화면에 쓰는 색.
pub struct Palette {
    pub background: Color32,
    /// 문장 블럭과 결과 창의 바탕.
    pub card: Color32,
    /// 보통 글.
    pub text: Color32,
    /// 지금 칠 문장처럼 가장 눈에 띄어야 하는 글.
    pub plain: Color32,
    /// 맞게 친 글자, 통과 표시.
    pub good: Color32,
    /// 틀린 글자.
    pub bad: Color32,
    /// 기다리는 문장, 도움말처럼 덜 중요한 글.
    pub dim: Color32,
    pub caret: Color32,
    /// 단추와 체크 상자의 테두리.
    pub border: Color32,
}

impl Palette {
    pub const fn of(theme: Theme) -> Self {
        match theme {
            Theme::Dark => Self {
                background: Color32::from_gray(27),
                card: Color32::from_gray(38),
                text: Color32::from_gray(215),
                plain: Color32::from_gray(240),
                good: Color32::from_rgb(120, 205, 140),
                bad: Color32::from_rgb(255, 120, 120),
                dim: Color32::from_gray(160),
                caret: Color32::from_rgb(90, 170, 255),
                border: Color32::from_gray(120),
            },
            Theme::Light => Self {
                background: Color32::from_gray(248),
                card: Color32::WHITE,
                text: Color32::from_gray(50),
                plain: Color32::from_gray(20),
                good: Color32::from_rgb(15, 115, 60),
                bad: Color32::from_rgb(200, 30, 30),
                dim: Color32::from_gray(95),
                caret: Color32::from_rgb(0, 100, 200),
                border: Color32::from_gray(130),
            },
        }
    }
}

/// 글자 크기와 두 화면의 색을 egui에 알린다.
pub fn install(ctx: &egui::Context) {
    ctx.all_styles_mut(|style| {
        style.text_styles = [
            (TextStyle::Heading, FontId::proportional(24.0)),
            (TextStyle::Body, FontId::proportional(16.0)),
            (TextStyle::Button, FontId::proportional(16.0)),
            (TextStyle::Small, FontId::proportional(13.0)),
            (TextStyle::Monospace, FontId::monospace(16.0)),
        ]
        .into();
    });
    for theme in [Theme::Dark, Theme::Light] {
        let palette = Palette::of(theme);
        ctx.style_mut_of(theme, |style| {
            style.visuals.panel_fill = palette.background;
            style.visuals.window_fill = palette.card;
            style.visuals.window_stroke = Stroke::new(1.0, palette.border);
            style.visuals.widgets.noninteractive.fg_stroke.color = palette.text;
            style.visuals.weak_text_color = Some(palette.dim);
            // 기본값은 테두리가 없어, 밝은 화면에서 체크 상자가 바탕에 묻힌다.
            style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, palette.border);
        });
    }
}

/// 화면 밝기를 고르는 단추. 오른쪽에서 왼쪽으로 놓이는 자리에 두므로 거꾸로 나열한다.
pub fn switcher(ui: &mut egui::Ui) {
    let before = ui.ctx().options(|options| options.theme_preference);
    let mut chosen = before;
    ui.selectable_value(&mut chosen, ThemePreference::Dark, "어둡게");
    ui.selectable_value(&mut chosen, ThemePreference::Light, "밝게");
    ui.selectable_value(&mut chosen, ThemePreference::System, "시스템");
    if chosen != before {
        ui.ctx().set_theme(chosen);
        // 창 테두리(제목 표시줄)도 같은 밝기로 맞춘다.
        let window_theme = match chosen {
            ThemePreference::Dark => SystemTheme::Dark,
            ThemePreference::Light => SystemTheme::Light,
            ThemePreference::System => SystemTheme::SystemDefault,
        };
        ui.ctx()
            .send_viewport_cmd(egui::ViewportCommand::SetTheme(window_theme));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// WCAG 2.x의 명암비 (1 ~ 21).
    fn contrast(a: Color32, b: Color32) -> f32 {
        let luminance = |color: Color32| {
            let linear = |channel: u8| {
                let c = f32::from(channel) / 255.0;
                if c <= 0.04045 {
                    c / 12.92
                } else {
                    ((c + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * linear(color.r()) + 0.7152 * linear(color.g()) + 0.0722 * linear(color.b())
        };
        let (a, b) = (luminance(a), luminance(b));
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    #[test]
    fn contrast_matches_known_values() {
        assert!((contrast(Color32::BLACK, Color32::WHITE) - 21.0).abs() < 0.01);
        assert!((contrast(Color32::WHITE, Color32::WHITE) - 1.0).abs() < 0.01);
    }

    /// 작은 글도 읽을 수 있는 기준(WCAG AA, 4.5:1)을 모든 색이 두 화면에서 넘는다.
    /// 글은 바탕 위에도, 문장 블럭 위에도 놓인다.
    #[test]
    fn every_colour_is_readable_in_both_themes() {
        for theme in [Theme::Dark, Theme::Light] {
            let palette = Palette::of(theme);
            let colours = [
                ("text", palette.text),
                ("plain", palette.plain),
                ("good", palette.good),
                ("bad", palette.bad),
                ("dim", palette.dim),
                ("caret", palette.caret),
            ];
            for (under, surface) in [("바탕", palette.background), ("블럭", palette.card)] {
                for (name, colour) in colours {
                    let ratio = contrast(colour, surface);
                    assert!(ratio >= 4.5, "{theme:?} {under} 위 {name}: {ratio:.2}");
                }
                // 글이 아닌 테두리는 3:1이 기준이다.
                let ratio = contrast(palette.border, surface);
                assert!(ratio >= 3.0, "{theme:?} {under} 위 border: {ratio:.2}");
            }
        }
    }

    /// egui에 넘긴 색이 실제 화면 설정에 들어갔는지 본다.
    #[test]
    fn install_applies_the_palette_to_both_themes() {
        let ctx = egui::Context::default();
        install(&ctx);
        for theme in [Theme::Dark, Theme::Light] {
            let (palette, style) = (Palette::of(theme), ctx.style_of(theme));
            assert_eq!(style.visuals.panel_fill, palette.background);
            assert_eq!(style.visuals.window_fill, palette.card);
            assert_eq!(style.visuals.text_color(), palette.text);
            assert_eq!(style.visuals.weak_text_color(), palette.dim);
        }
    }
}
