//! 글 블럭과 진행 막대를 그린다.
//!
//! 블럭 하나에는 한 쪽의 글이 들어간다. 글은 창 너비에 맞춰 여러 줄로 나누고,
//! 다 칠 때까지 그 자리에 그대로 둔다. 줄마다 블럭이 바뀌면 치는 흐름이 끊기기 때문이다.

use std::cmp::Ordering;
use std::ops::Range;
use std::sync::Arc;

use eframe::egui::text::{CCursor, LayoutJob};
use eframe::egui::{
    self, Align, Color32, FontId, Galley, Layout, Pos2, Rect, RichText, Sense, Stroke, StrokeKind,
    TextFormat,
};
use keyboard_battle::overlay::{self, Cell, State};
use keyboard_battle::texts::{MAX_CELLS, Sentence};
use keyboard_battle::typed::TypedText;

use crate::settings::{Echo, Marker};
use crate::theme::Palette;

/// 블럭의 테두리와 안쪽 여백이 좌우로 차지하는 너비.
const CARD_INSET: f32 = 2.0 * (CARD_PADDING + 2.0);
const CARD_PADDING: f32 = 12.0;

/// 블럭이 놓일 자리의 너비 `width`에 맞춘 문장 글자 크기.
///
/// 가장 긴 문장과 커서가 한 줄에 들어오는 가장 큰 크기를 고른다. 글자 한 칸의 너비가
/// 정수 픽셀이 되어야 글자가 또렷하므로 짝수로 맞추고, 너무 작거나 크지 않게 한다.
pub fn typing_size(width: f32) -> f32 {
    // 글자 한 칸의 너비는 글자 크기의 절반이다.
    let fits = 2.0 * (width - CARD_INSET) / (MAX_CELLS + 1) as f32;
    ((fits / 2.0).floor() * 2.0).clamp(16.0, 40.0)
}

/// 블럭을 그리는 데 드는 것들.
pub struct Look<'a> {
    pub palette: &'a Palette,
    /// 문장 글자의 크기.
    pub size: f32,
    /// 지금 칠 낱말을 알려 주는 방식.
    pub marker: Marker,
    /// 친 글을 보여 주는 자리.
    pub echo: Echo,
}

/// 글자의 모양. `underline`이 있으면 그 색으로 밑줄을 긋는다.
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

/// 지금 칠 낱말의 글자 모양. 표시하는 방식은 설정을 따른다.
fn marked_format(color: Color32, look: &Look<'_>) -> TextFormat {
    let (palette, size) = (look.palette, look.size);
    match look.marker {
        Marker::Underline => typing_format(size, color, Some(palette.caret)),
        Marker::Block => TextFormat {
            background: palette.caret,
            ..typing_format(size, palette.card, None)
        },
        Marker::None => typing_format(size, color, None),
    }
}

/// 줄과 줄 사이의 틈(점). 친 글을 아래 줄에 보일 때는 칠 줄과 친 줄이 한 쌍이므로 쌍 사이를 더 띄운다.
const ROW_GAP: f32 = 4.0;
const PAIR_GAP: f32 = 10.0;
/// 다음에 칠 쪽을 미리 보일 때 보이는 줄 수.
const PREVIEW_ROWS: usize = 2;

/// 글자가 차지하는 칸 수. 영문·숫자·기호는 한 칸, 한글은 두 칸이다.
fn cells_of(c: char) -> usize {
    if c.is_ascii() { 1 } else { 2 }
}

/// 한 줄에 들어가는 칸 수. 줄 끝에 빈칸과 커서가 설 자리 하나는 남겨 둔다.
fn capacity(width: f32, size: f32) -> usize {
    let cells = (width / (size / 2.0)).floor() as usize;
    cells.saturating_sub(1).max(8)
}

/// 글을 한 줄에 `capacity`칸씩, 낱말이 잘리지 않게 여러 줄로 나눈다.
/// 돌려주는 것은 줄마다의 글자 번호 범위이고, 줄 끝의 빈칸은 그 줄에 붙는다.
pub fn rows(text: &str, capacity: usize) -> Vec<Range<usize>> {
    let chars: Vec<char> = text.chars().collect();
    let mut rows = Vec::new();
    let (mut start, mut used, mut at) = (0, 0, 0);
    while at < chars.len() {
        // 낱말 하나: 빈칸이 아닌 글자들과 그 뒤에 붙은 빈칸들.
        let is_space = |&index: &usize| chars[index] == ' ';
        let end = (at..chars.len()).find(is_space).unwrap_or(chars.len());
        let next = (end..chars.len())
            .find(|index| !is_space(index))
            .unwrap_or(chars.len());
        let width: usize = chars[at..end].iter().copied().map(cells_of).sum();
        if used > 0 && used + width > capacity {
            rows.push(start..at);
            (start, used) = (at, 0);
        }
        if width > capacity {
            // 한 줄보다 긴 낱말은 줄 끝에서 자른다.
            for (index, &c) in chars.iter().enumerate().take(end).skip(at) {
                let cells = cells_of(c);
                if used > 0 && used + cells > capacity {
                    rows.push(start..index);
                    (start, used) = (index, 0);
                }
                used += cells;
            }
            used += next - end;
        } else {
            used += width + (next - end);
        }
        at = next;
    }
    rows.push(start..chars.len());
    rows
}

/// 글 한 줄을 넘기지 않고 그대로 놓는다. 줄은 이미 나눠 두었다.
fn lay_out(ui: &egui::Ui, mut job: LayoutJob) -> Arc<Galley> {
    job.wrap.max_width = f32::INFINITY;
    ui.painter().layout_job(job)
}

/// 줄 하나의 자리를 잡고 글을 그린다. 그 자리와 놓인 글을 돌려준다.
fn paint_row(ui: &mut egui::Ui, job: LayoutJob, color: Color32) -> (Rect, Arc<Galley>) {
    let galley = lay_out(ui, job);
    let size = egui::vec2(ui.available_width(), galley.size().y);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter().galley(rect.min, galley.clone(), color);
    (rect, galley)
}

/// `origin`에 놓인 글의 `index`번째 글자 앞에 커서를 세운다.
fn paint_caret(ui: &egui::Ui, origin: Pos2, galley: &Galley, index: usize, palette: &Palette) {
    let spot = galley.pos_from_cursor(CCursor::new(index));
    let caret = Rect::from_min_size(origin + spot.min.to_vec2(), egui::vec2(2.0, spot.height()));
    ui.painter().rect_filled(caret, 0.0, palette.caret);
}

/// 한 줄을 한 가지 색으로 적는다. 끝낸 글이나 아직 기다리는 글에 쓴다.
fn plain_row_job(chars: &[char], row: Range<usize>, color: Color32, look: &Look<'_>) -> LayoutJob {
    let text: String = chars[row].iter().collect();
    let mut job = LayoutJob::default();
    job.append(&text, 0.0, typing_format(look.size, color, None));
    job
}

/// 칠 글의 한 줄. `word`(글자 번호 범위)에 든 글자는 설정에 따른 방식으로 표시해,
/// 눈이 따라갈 자리를 알려 준다.
fn target_row_job(
    chars: &[char],
    row: Range<usize>,
    word: &Range<usize>,
    look: &Look<'_>,
) -> LayoutJob {
    let color = look.palette.plain;
    let plain = typing_format(look.size, color, None);
    let marked = marked_format(color, look);
    let mut job = LayoutJob::default();
    for index in row {
        let format = if word.contains(&index) {
            marked.clone()
        } else {
            plain.clone()
        };
        job.append(chars[index].encode_utf8(&mut [0; 4]), 0.0, format);
    }
    job
}

/// 친 글 가운데 `row`에 놓이는 부분. 칠 글과 견주어 색을 입히고, 이 줄에 커서가 있으면
/// (`holds_cursor`) 조합 중인 글자를 밑줄과 함께 붙인다.
fn typed_row_job(
    chars: &[char],
    typed: &TypedText,
    text: &str,
    row: Range<usize>,
    holds_cursor: bool,
    look: &Look<'_>,
) -> LayoutJob {
    let (palette, size) = (look.palette, look.size);
    let mut job = LayoutJob::default();
    for (index, c) in typed.done().chars().enumerate() {
        if !row.contains(&index) {
            continue;
        }
        // 틀린 사이띄개도 보이도록 틀린 글자에는 밑줄을 긋는다.
        let format = if chars.get(index) == Some(&c) {
            typing_format(size, palette.good, None)
        } else {
            typing_format(size, palette.bad, Some(palette.bad))
        };
        job.append(c.encode_utf8(&mut [0; 4]), 0.0, format);
    }
    if holds_cursor && let Some(c) = typed.composing() {
        // 조합 중이라도 이미 글과 어긋났으면 바로 알린다.
        let color = if overlay::composing_is_wrong(text, typed) {
            palette.bad
        } else {
            palette.plain
        };
        let format = typing_format(size, color, Some(color));
        job.append(c.encode_utf8(&mut [0; 4]), 0.0, format);
    }
    // 커서가 설 자리로 빈칸을 하나 둔다.
    job.append(" ", 0.0, typing_format(size, palette.plain, None));
    job
}

/// 칠 글 위에 친 글을 겹친 한 줄.
fn overlay_row_job(
    cells: &[Cell],
    row: Range<usize>,
    word: &Range<usize>,
    look: &Look<'_>,
) -> LayoutJob {
    let (palette, size) = (look.palette, look.size);
    let mut job = LayoutJob::default();
    for index in row {
        let cell = &cells[index];
        let format = match cell.state {
            State::Right => typing_format(size, palette.good, None),
            State::Wrong => typing_format(size, palette.bad, Some(palette.bad)),
            State::Composing => typing_format(size, palette.plain, Some(palette.plain)),
            // 아직 치지 않은 글자 가운데 지금 칠 낱말에 든 것에는 표시를 붙인다.
            State::Todo if word.contains(&index) => marked_format(palette.plain, look),
            State::Todo => typing_format(size, palette.plain, None),
        };
        job.append(cell.ch.encode_utf8(&mut [0; 4]), 0.0, format);
    }
    // 줄을 다 채운 뒤에도 커서가 설 자리로 빈칸을 하나 둔다.
    job.append(" ", 0.0, typing_format(size, palette.plain, None));
    job
}

/// 글 하나가 놓인 처지.
#[derive(Clone, Copy)]
enum LineState<'a> {
    Done,
    /// 지금 치는 글과 거기에 친 것.
    Current(&'a TypedText),
    Waiting,
}

/// 커서가 있는 줄 `row`가 화면 밖에 있으면, 그 줄이 맨 위에 오도록 화면을 옮긴다.
/// 보이는 동안에는 옮기지 않는다. 줄이 바뀔 때마다 화면이 움직이면 치는 흐름이 끊기므로,
/// 한 번 옮길 때 아래로 여러 줄이 함께 보이게 하고 그 줄들을 다 칠 때까지 그대로 둔다.
fn keep_in_view(ui: &egui::Ui, row: Rect) {
    let view = ui.clip_rect();
    if row.top() < view.top() || row.bottom() > view.bottom() {
        ui.scroll_to_rect(row.expand2(egui::vec2(0.0, PAIR_GAP)), Some(Align::TOP));
    }
}

/// 지금 치는 글을 줄마다 그린다. `follow`가 있으면 커서가 있는 줄이 화면 밖에 있을 때 화면을
/// 옮긴다. 그 값은 글의 첫 줄에 커서가 있을 때 함께 보일 블럭 위쪽 끝의 높이다.
fn current_line_ui(
    ui: &mut egui::Ui,
    look: &Look<'_>,
    text: &str,
    typed: &TypedText,
    rows: &[Range<usize>],
    follow: Option<f32>,
) {
    let palette = look.palette;
    let chars: Vec<char> = text.chars().collect();
    let cells = overlay::cells(text, typed);
    // 손이 닿아 있는 글자의 번호. 조합 중인 글자가 있으면 그 글자다.
    let is_ahead = |cell: &Cell| matches!(cell.state, State::Todo | State::Composing);
    let at = match look.echo {
        Echo::Over => cells.iter().position(is_ahead).unwrap_or(cells.len()),
        Echo::Below => typed.done().chars().count(),
    };
    let composing = match look.echo {
        Echo::Over => cells
            .get(at)
            .is_some_and(|cell| cell.state == State::Composing),
        Echo::Below => typed.composing().is_some(),
    };
    let word = current_word(text, at);
    for (number, row) in rows.iter().enumerate() {
        let holds_cursor = row.contains(&at) || (number + 1 == rows.len() && at >= row.end);
        let top = ui.cursor().top();
        let (rect, galley) = match look.echo {
            Echo::Over => {
                let job = overlay_row_job(&cells, row.clone(), &word, look);
                paint_row(ui, job, palette.plain)
            }
            Echo::Below => {
                let target = target_row_job(&chars, row.clone(), &word, look);
                paint_row(ui, target, palette.plain);
                ui.add_space(ROW_GAP);
                let job = typed_row_job(&chars, typed, text, row.clone(), holds_cursor, look);
                paint_row(ui, job, palette.plain)
            }
        };
        if holds_cursor {
            // 조합 중인 글자는 밑줄이 커서 노릇을 하므로, 그 밖일 때만 커서를 세운다.
            if !composing {
                paint_caret(ui, rect.min, &galley, at.min(row.end) - row.start, palette);
            }
            if let Some(block_top) = follow {
                // 칠 줄과 친 줄이 함께 보여야 한다. 글의 첫 줄이면 블럭의 이름표까지 보인다.
                let top = if number == 0 { block_top.min(top) } else { top };
                keep_in_view(ui, rect.with_min_y(top));
            }
        }
        ui.add_space(row_gap(look));
    }
}

fn row_gap(look: &Look<'_>) -> f32 {
    match look.echo {
        Echo::Over => ROW_GAP,
        Echo::Below => PAIR_GAP,
    }
}

/// 글 하나를 줄마다 그린다. `follow`는 [`current_line_ui`]에 넘긴다.
fn line_ui(
    ui: &mut egui::Ui,
    look: &Look<'_>,
    text: &str,
    state: LineState<'_>,
    capacity: usize,
    follow: Option<f32>,
) {
    let palette = look.palette;
    let rows = rows(text, capacity);
    if let LineState::Current(typed) = state {
        current_line_ui(ui, look, text, typed, &rows, follow);
        return;
    }
    let chars: Vec<char> = text.chars().collect();
    let done = matches!(state, LineState::Done);
    for row in rows {
        match (look.echo, done) {
            // 끝낸 글은 친 그대로 초록으로 남긴다.
            (Echo::Over, true) => {
                paint_row(
                    ui,
                    plain_row_job(&chars, row, palette.good, look),
                    palette.good,
                );
            }
            (Echo::Over, false) => {
                paint_row(
                    ui,
                    plain_row_job(&chars, row, palette.dim, look),
                    palette.dim,
                );
            }
            (Echo::Below, _) => {
                let target = plain_row_job(&chars, row.clone(), palette.dim, look);
                let (rect, _) = paint_row(ui, target, palette.dim);
                ui.add_space(ROW_GAP);
                if done {
                    let echo = plain_row_job(&chars, row, palette.good, look);
                    paint_row(ui, echo, palette.good);
                } else {
                    // 친 글이 놓일 줄을 미리 비워 두어, 칠 차례가 와도 블럭 크기가 달라지지 않게 한다.
                    let size = egui::vec2(ui.available_width(), rect.height());
                    ui.allocate_exact_size(size, Sense::hover());
                }
            }
        }
        ui.add_space(row_gap(look));
    }
}

/// 블럭 맨 위의 줄: 왼쪽에 이름, 오른쪽에 덧붙이는 말.
fn header_ui(
    ui: &mut egui::Ui,
    palette: &Palette,
    title: &str,
    color: Color32,
    note: Option<&str>,
) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(title).color(color));
        if let Some(note) = note {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(RichText::new(note).color(palette.dim));
            });
        }
    });
}

/// 지금 치는 쪽.
pub struct Page<'a> {
    /// 블럭 왼쪽 위에 적는 이름.
    pub title: &'a str,
    /// 블럭 오른쪽 위에 덧붙이는 말. 방금 끝낸 쪽의 기록을 적는다.
    pub note: Option<&'a str>,
    /// 이 쪽에 든 글들.
    pub lines: &'a [Sentence],
    /// 그 가운데 끝낸 글의 수.
    pub done: usize,
    /// 지금 치는 글에 친 것. 쪽을 다 끝냈으면 없다.
    pub typed: Option<&'a TypedText>,
    /// 막 넘어온 블럭이 빛나는 정도 (0 ~ 1).
    pub glow: f32,
    /// 커서가 있는 줄이 화면 밖에 있으면 화면을 옮길지.
    pub follow: bool,
}

/// 지금 치는 쪽을 담은 블럭. 다 칠 때까지 글은 제자리에 있고 커서만 움직인다.
pub fn page(ui: &mut egui::Ui, look: &Look<'_>, page: &Page<'_>) -> egui::Response {
    let palette = look.palette;
    // 막 넘어온 블럭은 테두리가 잠깐 초록이었다가 제 색으로 돌아온다. 앞 쪽을 끝냈다는 신호다.
    let edge = palette.caret.lerp_to_gamma(palette.good, page.glow);
    let frame = egui::Frame::new()
        .fill(palette.card)
        .corner_radius(8.0)
        .stroke(Stroke::new(2.0, edge))
        .inner_margin(CARD_PADDING);
    // 블럭의 위쪽 끝. 쪽의 첫 줄로 화면을 옮길 때 블럭의 이름표까지 보이게 하는 데 쓴다.
    let block_top = ui.cursor().top();
    let shown = frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        header_ui(ui, palette, page.title, palette.plain, page.note);
        ui.add_space(2.0);
        ui.spacing_mut().item_spacing.y = 0.0;
        let capacity = capacity(ui.available_width(), look.size);
        for (index, line) in page.lines.iter().enumerate() {
            let state = match (index.cmp(&page.done), page.typed) {
                (Ordering::Less, _) => LineState::Done,
                (Ordering::Equal, Some(typed)) => LineState::Current(typed),
                _ => LineState::Waiting,
            };
            // 쪽의 첫 글이 아니면 이름표는 볼 것 없이 그 줄만 보이면 된다.
            let follow = page
                .follow
                .then_some(if index == 0 { block_top } else { f32::INFINITY });
            line_ui(ui, look, &line.text, state, capacity, follow);
        }
    });
    shown.response
}

/// 다음에 칠 쪽을 흐리게 미리 보인다. 앞의 몇 줄만 보여, 지금 치는 글에서 눈을 끌어가지 않게 한다.
pub fn preview(
    ui: &mut egui::Ui,
    look: &Look<'_>,
    title: &str,
    lines: &[Sentence],
) -> egui::Response {
    let palette = look.palette;
    let frame = egui::Frame::new()
        .fill(palette.card)
        .corner_radius(8.0)
        .stroke(Stroke::new(1.0, palette.border))
        .outer_margin(1.0)
        .inner_margin(CARD_PADDING);
    let shown = frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        header_ui(ui, palette, title, palette.dim, None);
        ui.add_space(2.0);
        ui.spacing_mut().item_spacing.y = 0.0;
        let capacity = capacity(ui.available_width(), look.size);
        let mut left = PREVIEW_ROWS;
        for line in lines {
            let chars: Vec<char> = line.text.chars().collect();
            for row in rows(&line.text, capacity).into_iter().take(left) {
                paint_row(
                    ui,
                    plain_row_job(&chars, row, palette.dim, look),
                    palette.dim,
                );
                ui.add_space(ROW_GAP);
                left -= 1;
            }
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

    /// 창이 넓어지면 글자도 커지고, 어느 너비에서든 가장 긴 문장이 한 줄에 들어온다.
    #[test]
    fn typing_size_grows_with_the_window_and_always_fits() {
        // 기본 창(960)과 가장 좁은 창(720)에서 블럭이 놓이는 너비.
        assert_eq!(typing_size(944.0), 24.0);
        assert_eq!(typing_size(704.0), 18.0);
        assert_eq!(typing_size(1264.0), 34.0);
        assert_eq!(typing_size(3000.0), 40.0, "한없이 커지지는 않는다");
        assert_eq!(
            typing_size(300.0),
            16.0,
            "읽을 수 없을 만큼 작아지지는 않는다"
        );

        let mut last = 0.0;
        for width in (704..2000).step_by(7) {
            let width = width as f32;
            let size = typing_size(width);
            assert!(size >= last && size % 2.0 == 0.0);
            let line = (MAX_CELLS + 1) as f32 * size / 2.0;
            assert!(line <= width - CARD_INSET, "{width}: {size}");
            // 러스트 코드 한 줄은 넘기지 않고 한 줄에 들어온다.
            assert!(capacity(width - CARD_INSET, size) >= MAX_CELLS);
            last = size;
        }
    }

    fn texts(text: &str, capacity: usize) -> Vec<String> {
        let chars: Vec<char> = text.chars().collect();
        let row = |row: Range<usize>| chars[row].iter().collect();
        rows(text, capacity).into_iter().map(row).collect()
    }

    /// 낱말은 잘리지 않고, 줄 끝의 빈칸은 그 줄에 붙는다. 한글은 두 칸씩 센다.
    #[test]
    fn rows_break_between_words() {
        assert_eq!(texts("the quick brown fox", 9), ["the quick ", "brown fox"]);
        assert_eq!(
            texts("the quick brown fox", 10),
            ["the quick ", "brown fox"]
        );
        assert_eq!(
            texts("the quick brown fox", 15),
            ["the quick brown ", "fox"]
        );
        assert_eq!(texts("the quick brown fox", 40), ["the quick brown fox"]);
        // '가는 말이'는 아홉 칸이다.
        assert_eq!(texts("가는 말이 곱다.", 9), ["가는 말이 ", "곱다."]);
        assert_eq!(texts("가는 말이 곱다.", 8), ["가는 ", "말이 ", "곱다."]);
        assert_eq!(texts("", 10), [""]);
    }

    /// 줄을 다시 이으면 본디 글이 되고, 어느 줄도 넘치지 않는다.
    #[test]
    fn rows_keep_every_character_and_fit() {
        let text = "let total: u64 = nums.iter().map(|n| n * n).sum(); 값싼 떡볶이와 닭꼬치";
        for capacity in 8..80 {
            let rows = texts(text, capacity);
            assert_eq!(rows.concat(), text, "{capacity}칸");
            for row in &rows {
                let cells: usize = row.trim_end().chars().map(cells_of).sum();
                assert!(cells <= capacity, "{capacity}칸: {row:?}");
            }
        }
    }

    /// 한 줄보다 긴 낱말은 줄 끝에서 자른다.
    #[test]
    fn a_word_longer_than_a_row_is_cut() {
        assert_eq!(texts("abcdefghij kl", 4), ["abcd", "efgh", "ij ", "kl"]);
        assert_eq!(texts("가나다라마", 4), ["가나", "다라", "마"]);
    }

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

    fn look(palette: &Palette, marker: Marker) -> Look<'_> {
        Look {
            palette,
            size: 24.0,
            marker,
            echo: Echo::Below,
        }
    }

    /// `index`번째 글자가 든 구획의 글자 모양.
    fn format_at(job: &LayoutJob, index: usize) -> TextFormat {
        let byte = job.text.char_indices().nth(index).expect("줄 안의 글자").0;
        let holds = |section: &&egui::text::LayoutSection| {
            (section.byte_range.start.0..section.byte_range.end.0).contains(&byte)
        };
        let section = job.sections.iter().find(holds).expect("줄 안의 자리");
        section.format.clone()
    }

    /// 표시는 지금 칠 낱말에만 붙고, 방식은 설정을 따른다. 줄을 나눠도 글자는 빠지지 않는다.
    #[test]
    fn the_current_word_is_marked_the_chosen_way() {
        let palette = Palette::of(egui::Theme::Dark);
        let chars: Vec<char> = "가는 말이 곱다.".chars().collect();
        // 둘째 줄 "말이 곱다."에서 '말이'가 지금 칠 낱말이다.
        let row = |marker| target_row_job(&chars, 3..9, &(3..5), &look(&palette, marker));
        for marker in [Marker::Underline, Marker::Block, Marker::None] {
            assert_eq!(row(marker).text, "말이 곱다.");
            let rest = format_at(&row(marker), 3);
            assert_eq!(rest.underline, Stroke::NONE);
            assert_eq!(rest.background, Color32::TRANSPARENT);
        }

        let underlined = format_at(&row(Marker::Underline), 0);
        assert_eq!(underlined.underline.color, palette.caret);
        assert_eq!(underlined.background, Color32::TRANSPARENT);

        let block = format_at(&row(Marker::Block), 0);
        assert_eq!(
            (block.background, block.color),
            (palette.caret, palette.card)
        );

        let none = format_at(&row(Marker::None), 0);
        assert_eq!(none.underline, Stroke::NONE);
        assert_eq!(none.background, Color32::TRANSPARENT);
    }

    /// 친 글은 줄마다 제 줄에 놓이고, 틀린 글자는 빨갛게, 조합 중인 글자는 커서가 있는 줄에만 붙는다.
    #[test]
    fn typed_text_lands_on_its_own_row() {
        use keyboard_battle::keymap::keystroke_for;
        use keyboard_battle::typed::Script;

        let palette = Palette::of(egui::Theme::Dark);
        let look = look(&palette, Marker::Underline);
        let text = "가는 말이 곱다.";
        let chars: Vec<char> = text.chars().collect();
        let mut typed = TypedText::default();
        // "가는 발"까지 친다: '말'의 첫소리를 틀렸고, '발'은 아직 조합 중이다.
        for unit in "ㄱㅏㄴㅡㄴ ㅂㅏㄹ".chars() {
            let (pos, shift) = keystroke_for(unit).expect("글쇠 하나로 치는 문자");
            typed.press(pos, shift, Script::Hangul);
        }
        assert_eq!((typed.done(), typed.composing()), ("가는 ", Some('발')));

        // 첫 줄 "가는 "에는 다 친 글자만 놓인다. 끝의 한 칸은 커서가 설 자리다.
        let first = typed_row_job(&chars, &typed, text, 0..3, false, &look);
        assert_eq!(first.text, "가는  ");
        assert_eq!(format_at(&first, 0).color, palette.good);

        // 둘째 줄에는 조합 중인 글자가 붙고, 이미 어긋났으므로 빨갛다.
        let second = typed_row_job(&chars, &typed, text, 3..9, true, &look);
        assert_eq!(second.text, "발 ");
        assert_eq!(format_at(&second, 0).color, palette.bad);
    }
}
