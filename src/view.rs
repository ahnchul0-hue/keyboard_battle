//! 문장 블럭과 진행 막대를 그린다.

use std::ops::Range;
use std::sync::Arc;

use eframe::egui::text::{CCursor, LayoutJob};
use eframe::egui::{
    self, Align, Color32, FontId, Galley, Layout, Rect, RichText, Sense, Stroke, StrokeKind,
    TextFormat,
};
use keyboard_battle::measure::Tally;
use keyboard_battle::texts::Sentence;
use keyboard_battle::typed::TypedText;

use crate::theme::Palette;

/// 블럭을 그리는 데 드는 것들.
pub struct Look<'a> {
    pub palette: &'a Palette,
    /// 문장 글자의 크기.
    pub size: f32,
}

/// 블럭 하나가 놓인 처지.
#[derive(Clone, Copy)]
pub enum Stage<'a> {
    /// 끝낸 문장과 그 기록. `glow`는 막 끝낸 블럭이 빛나는 정도(0 ~ 1)다.
    Done {
        tally: &'a Tally,
        glow: f32,
    },
    /// 지금 칠 문장과 거기에 친 글.
    Current(&'a TypedText),
    Waiting,
}

/// 문장 글자의 모양. `underline`이 있으면 그 색으로 밑줄을 긋는다.
fn typing_format(size: f32, color: Color32, underline: Option<Color32>) -> TextFormat {
    TextFormat {
        font_id: FontId::monospace(size),
        color,
        underline: underline.map_or(Stroke::NONE, |color| Stroke::new(2.0, color)),
        ..Default::default()
    }
}

/// 지금 칠 낱말의 범위(글자 번호). `typed`는 지금까지 친 글자 수다.
pub fn current_word(text: &str, typed: usize) -> Range<usize> {
    let chars: Vec<char> = text.chars().collect();
    let mut start = typed.min(chars.len());
    // 낱말 사이의 빈칸을 칠 차례면 그다음 낱말을 가리킨다.
    while chars.get(start) == Some(&' ') {
        start += 1;
    }
    // 낱말 한가운데면 그 낱말의 처음으로 돌아간다.
    while start > 0 && chars[start - 1] != ' ' {
        start -= 1;
    }
    let length = chars[start..].iter().take_while(|&&c| c != ' ').count();
    start..start + length
}

/// 칠 문장. `word`(글자 번호 범위)에는 밑줄을 그어, 눈이 따라갈 자리를 알려 준다.
fn target_job(
    text: &str,
    word: Range<usize>,
    size: f32,
    color: Color32,
    mark: Color32,
) -> LayoutJob {
    let byte = |index: usize| {
        let found = text.char_indices().nth(index);
        found.map_or(text.len(), |(at, _)| at)
    };
    let (start, end) = (byte(word.start), byte(word.end));
    let mut job = LayoutJob::default();
    job.append(&text[..start], 0.0, typing_format(size, color, None));
    job.append(
        &text[start..end],
        0.0,
        typing_format(size, color, Some(mark)),
    );
    job.append(&text[end..], 0.0, typing_format(size, color, None));
    job
}

/// 친 글을 칠 문장과 견주어 색을 입힌다. 조합 중인 글자에는 밑줄을 긋는다.
fn typed_job(typed: &TypedText, target: &str, look: &Look<'_>) -> LayoutJob {
    let (palette, size) = (look.palette, look.size);
    let mut job = LayoutJob::default();
    let mut expected = target.chars();
    for c in typed.done().chars() {
        // 틀린 사이띄개도 보이도록 틀린 글자에는 밑줄을 긋는다.
        let format = if expected.next() == Some(c) {
            typing_format(size, palette.good, None)
        } else {
            typing_format(size, palette.bad, Some(palette.bad))
        };
        job.append(c.encode_utf8(&mut [0; 4]), 0.0, format);
    }
    if let Some(c) = typed.composing() {
        let format = typing_format(size, palette.plain, Some(palette.plain));
        job.append(c.encode_utf8(&mut [0; 4]), 0.0, format);
    }
    // 커서가 설 자리로 빈칸을 하나 둔다.
    job.append(" ", 0.0, typing_format(size, palette.plain, None));
    job
}

fn layout(ui: &egui::Ui, mut job: LayoutJob, width: f32) -> Arc<Galley> {
    job.wrap.max_width = width;
    ui.painter().layout_job(job)
}

/// 친 글을 `row` 자리에 그린다. 조합 중인 글자는 밑줄이 커서 노릇을 하므로,
/// 그 밖일 때만 글 끝에 커서를 세운다.
fn paint_typed(ui: &egui::Ui, row: Rect, typed: &TypedText, target: &str, look: &Look<'_>) {
    let galley = layout(ui, typed_job(typed, target, look), row.width());
    if typed.composing().is_none() {
        let end = galley.pos_from_cursor(CCursor::new(typed.done().chars().count()));
        let top = row.min + end.min.to_vec2();
        let caret = Rect::from_min_size(top, egui::vec2(2.0, end.height()));
        ui.painter().rect_filled(caret, 0.0, look.palette.caret);
    }
    ui.painter().galley(row.min, galley, look.palette.plain);
}

/// 문장 하나를 담은 블럭.
pub fn card(
    ui: &mut egui::Ui,
    look: &Look<'_>,
    line: &Sentence,
    stage: Stage<'_>,
) -> egui::Response {
    let palette = look.palette;
    let (status, status_color) = match stage {
        Stage::Done { .. } => ("통과", palette.good),
        Stage::Current(_) => ("지금", palette.plain),
        Stage::Waiting => ("대기", palette.dim),
    };
    let frame = egui::Frame::new()
        .fill(palette.card)
        .corner_radius(8.0)
        .inner_margin(12.0);
    // 지금 칠 블럭은 테두리를 굵게 한다. 나머지는 그만큼 바깥을 띄워 블럭 크기가 달라지지 않게 한다.
    let frame = match stage {
        Stage::Current(_) => frame.stroke(Stroke::new(2.0, palette.caret)),
        // 막 끝낸 블럭은 테두리가 잠깐 초록으로 굵어졌다가 돌아온다.
        Stage::Done { glow, .. } => frame
            .stroke(Stroke::new(
                1.0 + glow,
                palette.border.lerp_to_gamma(palette.good, glow),
            ))
            .outer_margin(1.0 - glow),
        Stage::Waiting => frame
            .stroke(Stroke::new(1.0, palette.border))
            .outer_margin(1.0),
    };
    let shown = frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            let label = format!("{status} · {}", line.kind.name());
            ui.label(RichText::new(label).color(status_color));
            if let Stage::Done { tally, .. } = stage {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let record = format!(
                        "{:.0} 타/분 · 정확도 {:.0}%",
                        tally.per_minute(),
                        (tally.accuracy() * 100.0).floor()
                    );
                    ui.label(RichText::new(record).color(palette.dim));
                });
            }
        });

        let width = ui.available_width();
        let (word, color, mark) = match stage {
            Stage::Current(typed) => {
                let word = current_word(line.text, typed.done().chars().count());
                (word, palette.plain, palette.caret)
            }
            _ => (0..0, palette.dim, palette.dim),
        };
        let target = layout(
            ui,
            target_job(line.text, word, look.size, color, mark),
            width,
        );
        let (spot, _) = ui.allocate_exact_size(target.size(), Sense::hover());
        let height = spot.height();
        ui.painter().galley(spot.min, target, color);

        // 친 글이 놓일 줄. 지금 칠 블럭이 아니어도 칠 문장과 같은 높이를 비워 두어,
        // 문장이 넘어갈 때 블럭이 위아래로 움직이지 않게 한다.
        let (row, _) = ui.allocate_exact_size(egui::vec2(width, height), Sense::hover());
        if let Stage::Current(typed) = stage {
            paint_typed(ui, row, typed, line.text, look);
        }
    });
    shown.response
}

/// 과정 전체에서 얼마나 쳤는지 보여 주는 막대.
pub fn progress_bar(ui: &mut egui::Ui, fraction: f32, palette: &Palette) {
    let size = egui::vec2(ui.available_width(), 10.0);
    let (track, _) = ui.allocate_exact_size(size, Sense::hover());
    let radius = track.height() / 2.0;
    let mut filled = track;
    filled.set_width(track.width() * fraction.clamp(0.0, 1.0));
    let painter = ui.painter();
    painter.rect_filled(track, radius, palette.card);
    painter.rect_filled(filled, radius, palette.caret);
    let outline = Stroke::new(1.0, palette.border);
    painter.rect_stroke(track, radius, outline, StrokeKind::Inside);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_word_follows_the_typing_position() {
        let text = "가는 말이 곱다.";
        assert_eq!(current_word(text, 0), 0..2);
        assert_eq!(current_word(text, 1), 0..2);
        // 낱말을 다 치고 빈칸을 칠 차례면 다음 낱말을 가리킨다.
        assert_eq!(current_word(text, 2), 3..5);
        assert_eq!(current_word(text, 4), 3..5);
        assert_eq!(current_word(text, 6), 6..9);
        // 문장보다 길게 쳤으면 마지막 낱말에 머문다.
        assert_eq!(current_word(text, 20), 6..9);
        assert_eq!(current_word("", 0), 0..0);
    }

    #[test]
    fn target_job_keeps_the_whole_sentence() {
        for word in [0..0, 0..2, 3..5, 6..9] {
            let job = target_job("가는 말이 곱다.", word, 24.0, Color32::WHITE, Color32::RED);
            assert_eq!(job.text, "가는 말이 곱다.");
        }
    }
}
