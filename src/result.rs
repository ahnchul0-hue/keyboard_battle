//! 연습이 끝났을 때 뜨는 결과 창.
//!
//! 가장 큰 숫자 둘은 속도와 정확도다. 그 아래에 다음에 바꿔 볼 한 가지, 종류별 기록,
//! 자주 틀린 글쇠, 틀렸던 자리를 두고, 다음 행동은 글쇠 하나로 고르게 한다.

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
/// 결과 창과 화면 끝 사이에 남기는 틈.
const EDGE: f32 = 12.0;
/// 창 맨 아래, 단추 줄과 그 위 가름줄이 차지하는 높이.
const BUTTON_ROW: f32 = 44.0;
/// 틀린 자리를 보여 주는 문장 수. 긴 연습에서는 틀린 문장이 많아서, 다 늘어놓으면 읽지 않게 된다.
const SLIPS_SHOWN: usize = 6;

/// 결과 창에서 고른 다음 행동.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    /// 같은 시간으로, 새 문장으로 한 번 더 한다.
    Again,
    /// 같은 시간으로, 같은 문장부터 다시 친다.
    Retry,
    /// 방금 친 것을 되감아 본다.
    Replay,
    /// 연습 시간을 다시 고른다.
    ChangeTime,
}

/// 결과 창에 보여 줄 내용.
pub struct Summary<'a> {
    /// 정한 연습 시간(분).
    pub minutes: u32,
    /// 시간을 다 채우지 않고 그만두었으면, 그때까지 흐른 시간(초).
    pub stopped_after: Option<f64>,
    /// 연습 전체의 기록. 시간이 다 되어 치다 만 문장도 친 데까지 들어 있다.
    pub total: Tally,
    /// 끝낸 문장 수.
    pub sentences: usize,
    /// 종류마다 끝낸 문장 수와 기록. 친 것이 있는 종류만 든다.
    pub kinds: Vec<(Kind, usize, Tally)>,
    /// 바로 앞에 한 연습의 기록.
    pub previous: Option<Tally>,
    /// 이번 실행에서 지금까지 가장 빨랐던 속도(타/분).
    pub best: Option<f64>,
    /// 틀렸을 때 눌러야 했던 글쇠들. 많이 틀린 것부터.
    pub misses: Vec<(char, u32)>,
    /// 틀린 데가 있는 문장과, 틀리기 시작한 자리(글자 번호)들. 친 순서대로다.
    pub slips: Vec<(Sentence, &'a BTreeSet<usize>)>,
    /// 틀린 글쇠를 누르고, 지우고, 다시 치는 데 쓴 시간(초).
    pub repair_seconds: f64,
    /// 다음 연습에서 바꿔 볼 한 가지와 그 근거.
    pub counsel: Option<(String, String)>,
    /// 연습을 끝낸 뒤 흐른 시간(초).
    pub age: f64,
}

/// 초를 `분:초`로 적는다.
pub fn clock(seconds: f64) -> String {
    let seconds = seconds.max(0.0) as u64;
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

/// 걸린 시간을 적는다. 1분이 안 되면 소수 한 자리까지 보인다.
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
                ui.label(RichText::new(format!("지난 연습보다 {change}")).color(color));
            }
        });
    });
}

/// 다음 연습에서 바꿔 볼 한 가지. 숫자 바로 아래에 두어, 결과를 본 눈이 곧바로 닿게 한다.
fn counsel_ui(ui: &mut egui::Ui, palette: &Palette, advice: &str, reason: &str) {
    ui.add_space(6.0);
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("다음엔 이렇게").color(palette.caret));
        ui.label(RichText::new(advice).color(palette.plain));
    });
    ui.label(RichText::new(reason).color(palette.dim));
}

fn key_name(key: char) -> String {
    if key == ' ' {
        "사이띄개".to_owned()
    } else {
        key.to_string()
    }
}

/// 한글, 영문, 러스트를 따로 본 기록. 몇 분을 치면 문장이 수십 개라, 문장마다 늘어놓지 않는다.
fn kinds_ui(ui: &mut egui::Ui, palette: &Palette, summary: &Summary<'_>) {
    let dim = |text: &str| RichText::new(text).color(palette.dim);
    Grid::new("result_kinds")
        .num_columns(4)
        .spacing([28.0, 6.0])
        .show(ui, |ui| {
            for title in ["종류", "끝낸 문장", "속도", "정확도"] {
                ui.label(dim(title));
            }
            ui.end_row();
            for &(kind, sentences, tally) in &summary.kinds {
                ui.label(kind.name());
                ui.label(format!("{sentences}개"));
                // 글쇠 한둘만 친 종류는 속도를 낼 수 없다.
                let mut speed = if tally.timed == 0 {
                    "–".to_owned()
                } else {
                    format!("{:.0} 타/분", tally.per_minute())
                };
                if kind == Kind::English && tally.timed > 0 {
                    speed += &format!(" · {:.0} WPM", tally.wpm());
                }
                ui.label(speed);
                ui.label(format!("{:.1}%", tally.accuracy() * 100.0));
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

/// 틀리기 시작한 자리를 문장 위에 표시해 보여 준다.
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
    for (line, slips) in summary.slips.iter().take(SLIPS_SHOWN) {
        let mut job = LayoutJob::default();
        for (at, c) in line.text.chars().enumerate() {
            // 빈칸에서 틀린 것도 보이도록 밑줄을 함께 긋는다.
            let slipped = slips.contains(&at);
            let color = if slipped { palette.bad } else { palette.text };
            job.append(c.encode_utf8(&mut [0; 4]), 0.0, format(color, slipped));
        }
        ui.label(job);
    }
    if let Some(more) = summary.slips.len().checked_sub(SLIPS_SHOWN + 1) {
        let note = format!("틀린 문장이 {}개 더 있습니다.", more + 1);
        ui.label(RichText::new(note).color(palette.dim));
    }
}

/// 다음 행동을 고르는 단추들. 누른 것이 있으면 돌려준다.
fn choices_ui(ui: &mut egui::Ui, palette: &Palette) -> Option<Choice> {
    let mut picked = None;
    ui.horizontal(|ui| {
        let again = RichText::new("한 번 더 (Enter)").color(palette.card);
        if ui
            .add(egui::Button::new(again).fill(palette.caret))
            .clicked()
        {
            picked = Some(Choice::Again);
        }
        let others = [
            ("같은 문장 다시 (R)", Choice::Retry),
            ("다시 보기 (V)", Choice::Replay),
            ("시간 바꾸기 (T)", Choice::ChangeTime),
        ];
        for (label, choice) in others {
            if ui.button(label).clicked() {
                picked = Some(choice);
            }
        }
    });
    picked
}

/// 결과 창을 그린다. 단추를 눌렀으면 고른 행동을 돌려준다.
pub fn show(ctx: &egui::Context, palette: &Palette, summary: &Summary<'_>) -> Option<Choice> {
    let total = summary.total;
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
            let title = format!("{}분 연습 결과", summary.minutes);
            ui.label(RichText::new(title).heading().color(palette.plain));
            if let Some(after) = summary.stopped_after {
                let note = format!("{} 만에 그만두었습니다", span(after));
                ui.label(RichText::new(note).color(palette.dim));
            }
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
        let mut counts = format!(
            "끝낸 문장 {}개 · 누른 글쇠 {}개 · 틀린 글쇠 {}개",
            summary.sentences, total.presses, total.wrong
        );
        if total.wrong > 0 {
            counts += &format!(" · 고치는 데 {}", span(summary.repair_seconds));
        }
        ui.label(RichText::new(counts).color(palette.dim));
        if let Some((advice, reason)) = &summary.counsel {
            counsel_ui(ui, palette, advice, reason);
        }

        ui.separator();
        // 창이 작으면 종류별 기록과 틀린 자리가 화면을 넘는다. 이 부분만 내려 볼 수 있게 해서
        // 큰 숫자와 조언, 단추는 늘 보이게 한다.
        let above = ui.cursor().top() - ui.min_rect().top();
        let room = ctx.content_rect().height() - 2.0 * EDGE - above - BUTTON_ROW;
        egui::ScrollArea::vertical()
            .auto_shrink([false, true])
            .max_height(room.max(BUTTON_ROW))
            .show(ui, |ui| {
                kinds_ui(ui, palette, summary);
                ui.add_space(4.0);
                misses_ui(ui, palette, &summary.misses);
                slips_ui(ui, palette, summary);
            });
        ui.separator();
        choices_ui(ui, palette)
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
        assert_eq!(clock(180.0), "3:00");
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
