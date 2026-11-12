//! 과정이 끝났을 때 뜨는 결과 창.
//!
//! 가장 큰 숫자 둘은 속도와 정확도다. 그 아래에 지난 과정과의 차이, 문장별 기록,
//! 자주 틀린 글쇠를 두고, 다음 행동은 글쇠 하나로 고르게 한다.

use std::collections::BTreeSet;

use eframe::egui::text::LayoutJob;
use eframe::egui::{
    self, Align, Color32, FontId, Frame, Grid, Id, Layout, Modal, RichText, Stroke, TextFormat,
};
use keyboard_battle::measure::Tally;
use keyboard_battle::texts::{Kind, Sentence};

use crate::theme::Palette;

/// 결과 창이 떠오르는 데 걸리는 시간(초). 갑자기 덮지 않게 한다.
const APPEAR: f64 = 0.2;
/// 뒤 화면을 가리는 정도 (0 ~ 255). 방금 친 블럭이 비쳐 보이도록 옅게 둔다.
const BACKDROP: f32 = 70.0;

/// 결과 창에서 고른 다음 행동.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    /// 새 문장으로 넘어간다.
    Next,
    /// 같은 문장을 다시 친다.
    Retry,
}

/// 결과 창에 보여 줄 내용.
pub struct Summary<'a> {
    /// 과정 번호 (1부터).
    pub number: usize,
    pub lines: &'a [Sentence],
    /// 문장별 기록. `lines`와 순서가 같다.
    pub results: &'a [Tally],
    /// 바로 앞에 끝낸 과정의 기록.
    pub previous: Option<Tally>,
    /// 이번 실행에서 지금까지 가장 빨랐던 속도(타/분).
    pub best: Option<f64>,
    /// 틀렸을 때 눌러야 했던 글쇠들. 많이 틀린 것부터.
    pub misses: Vec<(char, u32)>,
    /// 문장마다, 틀리기 시작한 자리(글자 번호)들. `lines`와 순서가 같다.
    pub slips: &'a [BTreeSet<usize>],
    /// 과정을 끝낸 뒤 흐른 시간(초).
    pub age: f64,
}

/// 초를 `분:초`로 적는다.
pub fn clock(seconds: f64) -> String {
    let seconds = seconds.max(0.0) as u64;
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

/// 걸린 시간을 적는다. 문장 하나는 몇 초면 끝나므로 1분이 안 되면 소수 한 자리까지 보인다.
fn span(seconds: f64) -> String {
    if seconds < 60.0 {
        format!("{seconds:.1}초")
    } else {
        format!("{}분 {}초", seconds as u64 / 60, seconds as u64 % 60)
    }
}

/// 지난 기록과의 차이를 적는다. 반올림해서 0이면 `None`.
fn change(now: f64, before: f64, decimals: usize) -> Option<String> {
    let text = format!("{:+.decimals$}", now - before);
    text.chars()
        .any(|c| ('1'..='9').contains(&c))
        .then_some(text)
}

/// 큰 숫자 하나와 그 아래의 이름표.
fn figure(
    ui: &mut egui::Ui,
    palette: &Palette,
    (value, unit, name): (String, &str, &str),
    change: Option<String>,
) {
    ui.vertical(|ui| {
        let section = |size, color| TextFormat {
            font_id: FontId::proportional(size),
            color,
            ..Default::default()
        };
        let mut job = LayoutJob::default();
        job.append(&value, 0.0, section(44.0, palette.plain));
        job.append(unit, 6.0, section(16.0, palette.dim));
        ui.label(job);
        ui.horizontal(|ui| {
            ui.label(RichText::new(name).color(palette.dim));
            // 부호가 뜻을 전하고, 색은 거들 뿐이다.
            if let Some(change) = change {
                let color = if change.starts_with('+') {
                    palette.good
                } else {
                    palette.bad
                };
                ui.label(RichText::new(format!("지난 과정보다 {change}")).color(color));
            }
        });
    });
}

fn key_name(key: char) -> String {
    if key == ' ' {
        "사이띄개".to_owned()
    } else {
        key.to_string()
    }
}

fn lines_ui(ui: &mut egui::Ui, palette: &Palette, summary: &Summary<'_>) {
    let dim = |text: &str| RichText::new(text).color(palette.dim);
    Grid::new("result_lines")
        .num_columns(4)
        .spacing([28.0, 6.0])
        .show(ui, |ui| {
            for title in ["문장", "속도", "정확도", "시간"] {
                ui.label(dim(title));
            }
            ui.end_row();
            for (index, (line, tally)) in summary.lines.iter().zip(summary.results).enumerate() {
                ui.label(format!("{} {}", index + 1, line.kind.name()));
                let mut speed = format!("{:.0} 타/분", tally.per_minute());
                if line.kind == Kind::English {
                    speed += &format!(" · {:.0} WPM", tally.wpm());
                }
                ui.label(speed);
                ui.label(format!("{:.1}%", tally.accuracy() * 100.0));
                ui.label(span(tally.seconds));
                ui.end_row();
            }
        });
}

fn misses_ui(ui: &mut egui::Ui, palette: &Palette, misses: &[(char, u32)]) {
    if misses.is_empty() {
        ui.label(RichText::new("틀린 글쇠 없이 끝냈습니다.").color(palette.good));
        return;
    }
    let keys: Vec<String> = misses
        .iter()
        .take(5)
        .map(|&(key, count)| format!("{} {count}번", key_name(key)))
        .collect();
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("자주 틀린 글쇠").color(palette.dim));
        ui.label(keys.join(" · "));
    });
}

/// 틀리기 시작한 자리를 문장 위에 표시해 보여 준다. 틀린 데가 없는 문장은 건너뛴다.
fn slips_ui(ui: &mut egui::Ui, palette: &Palette, summary: &Summary<'_>) {
    let format = |color, marked: bool| TextFormat {
        font_id: FontId::monospace(16.0),
        color,
        underline: if marked {
            Stroke::new(2.0, color)
        } else {
            Stroke::NONE
        },
        ..Default::default()
    };
    for (index, (line, slips)) in summary.lines.iter().zip(summary.slips).enumerate() {
        if slips.is_empty() {
            continue;
        }
        let mut job = LayoutJob::default();
        job.append(&format!("{} ", index + 1), 0.0, format(palette.dim, false));
        for (at, c) in line.text.chars().enumerate() {
            // 빈칸에서 틀린 것도 보이도록 밑줄을 함께 긋는다.
            let slipped = slips.contains(&at);
            let color = if slipped { palette.bad } else { palette.text };
            job.append(c.encode_utf8(&mut [0; 4]), 0.0, format(color, slipped));
        }
        ui.label(job);
    }
}

/// 결과 창을 그린다. 단추를 눌렀으면 고른 행동을 돌려준다.
pub fn show(ctx: &egui::Context, palette: &Palette, summary: &Summary<'_>) -> Option<Choice> {
    let total: Tally = summary.results.iter().copied().sum();
    let previous = summary.previous;
    let shown = ((summary.age / APPEAR) as f32).clamp(0.0, 1.0);
    if shown < 1.0 {
        ctx.request_repaint();
    }
    let id = Id::unique("result");
    let modal = Modal::new(id)
        // 떠오르는 정도를 여기서 직접 정하므로 egui의 것은 끈다.
        .area(Modal::default_area(id).fade_in(false))
        .frame(Frame::popup(&ctx.global_style()).multiply_with_opacity(shown))
        .backdrop_color(Color32::from_black_alpha((BACKDROP * shown) as u8));
    let modal = modal.show(ctx, |ui| {
        ui.set_opacity(shown);
        ui.set_width(580.0);
        ui.horizontal(|ui| {
            let title = format!("과정 {} 결과", summary.number);
            ui.label(RichText::new(title).heading().color(palette.plain));
            if summary.best.is_some_and(|best| total.per_minute() > best) {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new("이번 실행 최고 속도").color(palette.good));
                });
            }
        });
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            let speed = total.per_minute();
            let faster = previous.and_then(|before| change(speed, before.per_minute(), 0));
            figure(
                ui,
                palette,
                (format!("{speed:.0}"), "타/분", "속도"),
                faster,
            );
            ui.add_space(48.0);
            let accuracy = total.accuracy() * 100.0;
            let cleaner =
                previous.and_then(|before| change(accuracy, before.accuracy() * 100.0, 1));
            figure(
                ui,
                palette,
                (format!("{accuracy:.1}"), "%", "정확도"),
                cleaner,
            );
        });
        ui.add_space(8.0);
        let counts = format!(
            "걸린 시간 {} · 누른 글쇠 {}개 · 틀린 글쇠 {}개",
            span(total.seconds),
            total.presses,
            total.wrong
        );
        ui.label(RichText::new(counts).color(palette.dim));

        ui.separator();
        lines_ui(ui, palette, summary);
        ui.add_space(4.0);
        misses_ui(ui, palette, &summary.misses);
        slips_ui(ui, palette, summary);
        ui.separator();

        ui.horizontal(|ui| {
            let next = RichText::new("다음 과정 (Enter)").color(palette.card);
            let next = egui::Button::new(next).fill(palette.caret);
            if ui.add(next).clicked() {
                return Some(Choice::Next);
            }
            ui.button("같은 문장 다시 (R)")
                .clicked()
                .then_some(Choice::Retry)
        })
        .inner
    });
    modal.inner
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_shows_minutes_and_seconds() {
        assert_eq!(clock(0.0), "0:00");
        assert_eq!(clock(9.9), "0:09");
        assert_eq!(clock(72.4), "1:12");
        assert_eq!(clock(-1.0), "0:00");
    }

    #[test]
    fn span_keeps_a_decimal_for_short_times() {
        assert_eq!(span(7.44), "7.4초");
        assert_eq!(span(59.94), "59.9초");
        assert_eq!(span(72.4), "1분 12초");
    }

    #[test]
    fn change_carries_its_sign_and_hides_no_change() {
        assert_eq!(change(318.4, 300.0, 0).as_deref(), Some("+18"));
        assert_eq!(change(288.0, 300.0, 0).as_deref(), Some("-12"));
        assert_eq!(change(97.4, 98.0, 1).as_deref(), Some("-0.6"));
        assert_eq!(change(300.2, 300.0, 0), None);
        assert_eq!(change(98.0, 98.0, 1), None);
    }

    #[test]
    fn space_key_gets_a_readable_name() {
        assert_eq!(key_name(' '), "사이띄개");
        assert_eq!(key_name('ㅆ'), "ㅆ");
    }
}
