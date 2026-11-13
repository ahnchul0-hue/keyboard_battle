//! 연습이 끝났을 때 뜨는 결과 창.
//!
//! 내용을 구획으로 나눠 담는다. 맨 위에 큰 숫자 셋(속도, 정확도, 친 글쇠), 그 아래에 다음에
//! 바꿔 볼 한 가지, 지난 기록과 견준 것, 종류별 기록, 틀린 곳의 차례다. 큰 숫자와 조언, 단추는
//! 창이 작아도 늘 보이고, 그 아래 구획들만 내려 볼 수 있다.

use eframe::egui::text::LayoutJob;
use eframe::egui::{
    self, Align, Color32, FontId, Frame, Grid, Id, Layout, Modal, Pos2, Rect, RichText, Sense,
    Stroke, TextFormat,
};
use keyboard_battle::measure::Tally;
use keyboard_battle::records::Standing;
use keyboard_battle::texts::Kind;

use crate::theme::Palette;

/// 결과 창이 떠오르는 데 걸리는 시간(초). 갑자기 덮지 않게 한다.
const APPEAR: f64 = 0.2;
/// 뒤 화면을 가리는 정도 (0 ~ 255). 방금 친 블럭이 비쳐 보이도록 옅게 둔다.
const BACKDROP: f32 = 70.0;
/// 결과 창과 화면 끝 사이에 남기는 틈.
const EDGE: f32 = 12.0;
/// 결과 창 안쪽의 가장 넓은 너비.
const WIDEST: f32 = 880.0;
/// 안쪽이 이보다 넓으면 견주기와 종류별 기록을 나란히 놓는다.
const SIDE_BY_SIDE: f32 = 800.0;
/// 화면이 이보다 높으면 조언을 늘 보이는 자리에 둔다.
const ROOMY: f32 = 560.0;
/// 단추가 한 줄에 다 들어가는 가장 좁은 너비.
const BUTTONS_FIT: f32 = 680.0;
/// 창 맨 아래, 단추 한 줄이 차지하는 높이.
const BUTTON_ROW: f32 = 36.0;
/// 구획과 구획 사이의 틈.
const GAP: f32 = 8.0;
/// 틀린 자리를 보여 주는 낱말 수. 긴 연습에서는 틀린 낱말이 많아서, 다 늘어놓으면 읽지 않게 된다.
const SLIPS_SHOWN: usize = 12;
/// 최근 흐름을 그리는 자리의 높이와 가장 넓은 너비.
const TREND_HEIGHT: f32 = 26.0;
const TREND_WIDTH: f32 = 150.0;
/// 흐름의 끝(이번)에 찍는 점의 반지름.
const DOT: f32 = 4.0;

/// 더 클 때와 더 작을 때 붙이는 말.
const FASTER: (&str, &str) = ("빠름", "느림");
const CLEANER: (&str, &str) = ("높음", "낮음");

/// 결과 창에서 고른 다음 행동.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    /// 같은 시간으로, 새 글로 한 번 더 한다.
    Again,
    /// 같은 시간으로, 같은 글부터 다시 친다.
    Retry,
    /// 방금 친 것을 되감아 본다.
    Replay,
    /// 연습 시간을 다시 고른다.
    ChangeTime,
    /// 프로그램을 끝낸다.
    Quit,
}

/// 결과 창에 보여 줄 내용.
pub struct Summary<'a> {
    /// 정한 연습 시간(분).
    pub minutes: u32,
    /// 시간을 다 채우지 않고 그만두었으면, 그때까지 흐른 시간(초). 그런 연습은 기록에 남기지 않는다.
    pub stopped_after: Option<f64>,
    /// 연습 전체의 기록. 시간이 다 되어 치다 만 글도 친 데까지 들어 있다.
    pub total: Tally,
    /// 끝까지 친 문단 수.
    pub pages: usize,
    /// 종류마다의 기록. 친 것이 있는 종류만 든다.
    pub kinds: Vec<(Kind, Tally)>,
    /// 이 연습을 하기 전까지의 기록에서 뽑은, 견줄 것들.
    pub standing: &'a Standing,
    /// 지금 시각(1970년부터 센 초). 지난 기록이 얼마나 오래됐는지 적는 데 쓴다.
    pub today: u64,
    /// 기록을 읽거나 적지 못했으면 그 까닭.
    pub trouble: Option<&'a str>,
    /// 틀렸을 때 눌러야 했던 글쇠들. 많이 틀린 것부터.
    pub misses: Vec<(char, u32)>,
    /// 틀린 데가 있는 낱말과, 그 안에서 틀리기 시작한 자리(글자 번호)들. 친 순서대로다.
    pub slips: Vec<(String, Vec<usize>)>,
    /// 틀린 글쇠를 누르고, 지우고, 다시 치는 데 쓴 시간(초).
    pub repair_seconds: f64,
    /// 다음 연습에서 바꿔 볼 한 가지와 그 근거.
    pub counsel: Option<(String, String)>,
    /// 연습을 끝낸 뒤 흐른 시간(초).
    pub age: f64,
}

impl Summary<'_> {
    /// 이 연습을 기록에 남겼는지. 정한 시간을 다 채운 연습만 남긴다.
    fn saved(&self) -> bool {
        self.stopped_after.is_none()
    }
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

/// 얼마나 오래됐는지를 적는다. 날짜로 적지 않는 것은, 시간대를 몰라도 틀리지 않기 때문이다.
fn ago(seconds: u64) -> String {
    match seconds {
        0..60 => "방금".to_owned(),
        60..3600 => format!("{}분 전", seconds / 60),
        3600..86_400 => format!("{}시간 전", seconds / 3600),
        _ => format!("{}일 전", seconds / 86_400),
    }
}

/// 분으로 센 시간을 시간과 분으로 적는다.
fn hours_minutes(minutes: u64) -> String {
    match (minutes / 60, minutes % 60) {
        (0, minutes) => format!("{minutes}분"),
        (hours, 0) => format!("{hours}시간"),
        (hours, minutes) => format!("{hours}시간 {minutes}분"),
    }
}

/// 견준 결과의 좋고 나쁨.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tone {
    Better,
    Worse,
    Same,
}

/// 이번 값 `now`가 견줄 값 `before`와 얼마나 다른지를 말로 적는다.
/// `words`는 더 클 때와 더 작을 때 붙이는 말이다. 반올림해서 0이면 같다고 적는다.
/// 좋고 나쁨은 말이 전하고, 색은 거들 뿐이다.
fn gap(now: f64, before: f64, decimals: usize, words: (&str, &str)) -> (String, Tone) {
    let size = format!("{:.decimals$}", (now - before).abs());
    if !size.chars().any(|c| ('1'..='9').contains(&c)) {
        ("같음".to_owned(), Tone::Same)
    } else if now > before {
        (format!("{size} {}", words.0), Tone::Better)
    } else {
        (format!("{size} {}", words.1), Tone::Worse)
    }
}

fn tone_color(palette: &Palette, tone: Tone) -> Color32 {
    match tone {
        Tone::Better => palette.good,
        Tone::Worse => palette.bad,
        Tone::Same => palette.dim,
    }
}

fn gap_label(ui: &mut egui::Ui, palette: &Palette, (text, tone): (String, Tone)) {
    ui.label(RichText::new(text).color(tone_color(palette, tone)));
}

fn key_name(key: char) -> String {
    if key == ' ' {
        "사이띄개".to_owned()
    } else {
        key.to_string()
    }
}

/// 구획 하나. 창과 다른 바탕에 테두리를 둘러, 어디까지가 한 묶음인지 보이게 한다.
/// 내용이 차지한 높이를 돌려주고, 그 높이가 `min_height`보다 낮으면 구획을 그만큼 늘린다.
fn section(
    ui: &mut egui::Ui,
    palette: &Palette,
    min_height: f32,
    body: impl FnOnce(&mut egui::Ui),
) -> f32 {
    // 구획의 테두리는 단추의 테두리보다 옅게 둔다. 누르는 것이 아니기 때문이다.
    let edge = palette.border.lerp_to_gamma(palette.background, 0.55);
    let frame = Frame::new()
        .fill(palette.background)
        .stroke(Stroke::new(1.0, edge))
        .corner_radius(8.0)
        .inner_margin(10.0);
    let shown = frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        // 높이는 내용을 넣기 전에 정한다. 넣은 뒤에 정하면 그만큼이 내용 아래에 더 붙는다.
        ui.set_min_height(min_height);
        // 내용의 높이는 그 앞뒤로 다음 것이 놓일 자리가 얼마나 내려갔는지로 잰다.
        // 구획의 크기로 재면, 방금 늘려 둔 높이가 섞여 들어가 다시는 줄어들지 않는다.
        let top = ui.cursor().top();
        body(ui);
        (ui.cursor().top() - top - ui.spacing().item_spacing.y).max(0.0)
    });
    shown.inner
}

/// 구획들을 같은 너비, 같은 높이로 나란히 놓는다.
/// 높이는 지난 화면에서 가장 높았던 구획에 맞춘다.
fn side_by_side(
    ui: &mut egui::Ui,
    palette: &Palette,
    row: &str,
    bodies: &mut [&mut dyn FnMut(&mut egui::Ui)],
) {
    let id = Id::unique(("result_row", row));
    let level: f32 = ui.ctx().data(|data| data.get_temp(id)).unwrap_or(0.0);
    let mut tallest: f32 = 0.0;
    ui.columns(bodies.len(), |columns| {
        for (column, body) in columns.iter_mut().zip(bodies.iter_mut()) {
            let height = section(column, palette, level, |ui| body(ui));
            tallest = tallest.max(height);
        }
    });
    ui.ctx().data_mut(|data| data.insert_temp(id, tallest));
}

/// 구획의 이름표.
fn title(ui: &mut egui::Ui, text: &str, color: Color32) {
    ui.label(RichText::new(text).small().color(color));
    ui.add_space(2.0);
}

/// 값들을 `area` 안의 점으로 옮긴다. 왼쪽 끝이 가장 오래된 값, 오른쪽 끝이 마지막 값이고,
/// 가장 작은 값이 바닥에, 가장 큰 값이 꼭대기에 온다. 값이 모두 같으면 가운데 높이에 놓는다.
fn trend_points(values: &[f64], area: Rect) -> Vec<Pos2> {
    let lowest = values.iter().copied().fold(f64::INFINITY, f64::min);
    let highest = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let steps = values.len().saturating_sub(1).max(1) as f32;
    let point = |(index, &value): (usize, &f64)| {
        let height = if highest > lowest {
            ((value - lowest) / (highest - lowest)) as f32
        } else {
            0.5
        };
        let x = area.left() + area.width() * index as f32 / steps;
        Pos2::new(x, area.bottom() - area.height() * height)
    };
    values.iter().enumerate().map(point).collect()
}

/// 최근 기록의 흐름을 작은 꺾은선으로 그린다. 선은 옅게 두고 마지막 값(이번)에만 점을 찍어,
/// 눈이 이번 기록으로 가게 한다. 축과 눈금은 두지 않는다. 값은 옆의 큰 숫자와 견주기 구획에서 읽는다.
fn trend_ui(ui: &mut egui::Ui, palette: &Palette, values: &[f64], hover: &str) {
    let size = egui::vec2(ui.available_width().min(TREND_WIDTH), TREND_HEIGHT);
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    let points = trend_points(values, rect.shrink(DOT + 2.0));
    let painter = ui.painter();
    if let Some(&last) = points.last() {
        painter.line(points, Stroke::new(2.0, palette.dim));
        // 점 둘레를 바탕색으로 비워, 선과 겹쳐도 점이 또렷하게 한다.
        painter.circle_filled(last, DOT + 2.0, palette.background);
        painter.circle_filled(last, DOT, palette.caret);
    }
    response.on_hover_text(hover);
}

/// 큰 숫자 칸의 맨 아랫줄.
enum Foot {
    /// 최근 기록의 흐름: 이름, 값들(오래된 것부터, 마지막이 이번), 마우스를 올렸을 때의 설명.
    Trend(String, Vec<f64>, String),
    Words(String),
}

/// 큰 숫자 하나를 담은 칸.
struct Tile<'a> {
    name: &'a str,
    value: String,
    unit: &'a str,
    /// 숫자 아래에 적는 한 줄과 그 색.
    note: (String, Color32),
    foot: Foot,
}

fn tile_ui(ui: &mut egui::Ui, palette: &Palette, tile: &Tile<'_>) {
    title(ui, tile.name, palette.dim);
    let part = |size, color| TextFormat {
        font_id: FontId::proportional(size),
        color,
        ..Default::default()
    };
    let mut job = LayoutJob::default();
    job.append(&tile.value, 0.0, part(36.0, palette.plain));
    job.append(tile.unit, 6.0, part(16.0, palette.dim));
    ui.label(job);
    ui.label(RichText::new(&tile.note.0).color(tile.note.1));
    ui.add_space(2.0);
    // 흐름이 없는 칸도 같은 높이를 차지해, 칸들의 줄이 맞는다.
    let row = egui::vec2(ui.available_width(), TREND_HEIGHT);
    ui.allocate_ui_with_layout(
        row,
        Layout::left_to_right(Align::Center),
        |ui| match &tile.foot {
            Foot::Trend(name, values, hover) => {
                ui.label(RichText::new(name).small().color(palette.dim));
                trend_ui(ui, palette, values, hover);
            }
            Foot::Words(words) => {
                ui.label(RichText::new(words).small().color(palette.dim));
            }
        },
    );
}

/// 맨 위의 큰 숫자 셋: 속도, 정확도, 친 글쇠.
fn tiles_ui(ui: &mut egui::Ui, palette: &Palette, summary: &Summary<'_>) {
    let (total, standing) = (summary.total, summary.standing);
    let (speed, accuracy) = (total.per_minute(), total.accuracy() * 100.0);
    let versus = |gap: Option<(String, Tone)>| match gap {
        Some((_, Tone::Same)) => ("지난번과 같음".to_owned(), palette.dim),
        Some((text, tone)) => (format!("지난번보다 {text}"), tone_color(palette, tone)),
        None => ("견줄 지난 기록 없음".to_owned(), palette.dim),
    };
    // 지난 기록들의 값 뒤에 이번 값을 이어, 흐름의 끝이 이번이 되게 한다.
    let trend = |past: Vec<f64>, now: f64, unit: &str, decimals: usize| {
        if past.is_empty() {
            return Foot::Words(String::new());
        }
        let lowest = past.iter().copied().fold(now, f64::min);
        let highest = past.iter().copied().fold(now, f64::max);
        let count = past.len();
        let hover = format!(
            "지난 {count}번과 이번: {lowest:.decimals$} ~ {highest:.decimals$}{unit}. \
             오른쪽 끝의 점이 이번입니다."
        );
        let values = past.into_iter().chain([now]).collect();
        Foot::Trend(format!("지난 {count}번과 이번"), values, hover)
    };
    let previous = standing.previous;
    let speeds = standing.trail.iter().map(|&(speed, _)| speed).collect();
    let accuracies = standing.trail.iter().map(|&(_, accuracy)| accuracy * 100.0);
    let repair = if total.wrong > 0 && total.seconds > 0.0 {
        let share = summary.repair_seconds / total.seconds * 100.0;
        format!("고치는 데 {} ({share:.0}%)", span(summary.repair_seconds))
    } else {
        String::new()
    };
    let tiles = [
        Tile {
            name: "속도",
            value: format!("{speed:.0}"),
            unit: "타/분",
            note: versus(previous.map(|before| gap(speed, before.speed(), 0, FASTER))),
            foot: trend(speeds, speed, " 타/분", 0),
        },
        Tile {
            name: "정확도",
            value: format!("{accuracy:.1}"),
            unit: "%",
            note: versus(
                previous.map(|before| gap(accuracy, before.accuracy() * 100.0, 1, CLEANER)),
            ),
            foot: trend(accuracies.collect(), accuracy, "%", 1),
        },
        Tile {
            name: "친 글쇠",
            value: total.needed.to_string(),
            unit: "타",
            note: if total.wrong == 0 {
                ("틀린 글쇠 없음".to_owned(), palette.good)
            } else {
                (format!("틀린 글쇠 {}개", total.wrong), palette.text)
            },
            foot: Foot::Words(repair),
        },
    ];
    let [mut first, mut second, mut third] = tiles
        .each_ref()
        .map(|tile| move |ui: &mut egui::Ui| tile_ui(ui, palette, tile));
    side_by_side(
        ui,
        palette,
        "tiles",
        &mut [&mut first, &mut second, &mut third],
    );
}

/// 다음 연습에서 바꿔 볼 한 가지. 큰 숫자 바로 아래에 두어, 결과를 본 눈이 곧바로 닿게 한다.
fn counsel_ui(ui: &mut egui::Ui, palette: &Palette, advice: &str, reason: &str) {
    title(ui, "다음엔 이렇게", palette.caret);
    ui.label(RichText::new(advice).color(palette.plain));
    ui.label(RichText::new(reason).color(palette.dim));
}

/// 이번 결과를 지난번, 최근 평균, 최고 기록과 견준다.
fn compare_ui(ui: &mut egui::Ui, palette: &Palette, summary: &Summary<'_>) {
    title(ui, "지난 기록과 견주기", palette.dim);
    let dim = |text: String| RichText::new(text).color(palette.dim);
    let small = |text: String| RichText::new(text).small().color(palette.dim);
    let standing = summary.standing;
    let Some(previous) = standing.previous else {
        let words = if summary.saved() {
            "첫 기록을 남겼습니다. 다음 연습부터 이 기록과 견줘 보여 줍니다."
        } else {
            "아직 남은 기록이 없습니다. 정한 시간을 끝까지 채운 연습만 기록에 남습니다."
        };
        ui.label(dim(words.to_owned()));
        return;
    };
    let total = summary.total;
    let (speed, accuracy) = (total.per_minute(), total.accuracy() * 100.0);
    let mut rows = vec![("지난번", previous.speed(), previous.accuracy())];
    // 기록이 하나뿐이면 평균도 최고도 그 기록이라 따로 적을 것이 없다.
    if standing.count >= 2 {
        if let Some(recent) = standing.recent {
            rows.push(("최근 평균", recent.speed, recent.accuracy));
        }
        if let (Some(fastest), Some(cleanest)) = (standing.fastest, standing.cleanest) {
            rows.push(("최고 기록", fastest.speed(), cleanest.accuracy()));
        }
    }
    Grid::new("result_compare")
        .num_columns(5)
        .spacing([14.0, 6.0])
        .show(ui, |ui| {
            for name in ["", "속도", "이번이", "정확도", "이번이"] {
                ui.label(dim(name.to_owned()));
            }
            ui.end_row();
            let strong = |text: String| RichText::new(text).color(palette.plain);
            ui.label(strong("이번".to_owned()));
            ui.label(strong(format!("{speed:.0} 타/분")));
            ui.label("");
            ui.label(strong(format!("{accuracy:.1}%")));
            ui.end_row();
            for (name, before_speed, before_accuracy) in rows {
                let before_accuracy = before_accuracy * 100.0;
                ui.label(dim(name.to_owned()));
                ui.label(format!("{before_speed:.0} 타/분"));
                gap_label(ui, palette, gap(speed, before_speed, 0, FASTER));
                ui.label(format!("{before_accuracy:.1}%"));
                gap_label(ui, palette, gap(accuracy, before_accuracy, 1, CLEANER));
                ui.end_row();
            }
        });
    ui.add_space(4.0);
    let mut notes = format!(
        "지난번: {} {}분 연습",
        ago(summary.today.saturating_sub(previous.at)),
        previous.minutes
    );
    if let Some(recent) = standing.recent.filter(|_| standing.count >= 2) {
        notes += &format!(" · 최근 평균: 지난 {}번", recent.count);
    }
    ui.label(small(notes));
    let (count, minutes) = if summary.saved() {
        let minutes = standing.minutes + u64::from(summary.minutes);
        (standing.count + 1, minutes)
    } else {
        (standing.count, standing.minutes)
    };
    ui.label(small(format!(
        "이번까지 {count}번, 모두 {} 연습했습니다.",
        hours_minutes(minutes)
    )));
}

/// 한글, 영문, 러스트를 따로 본 기록. 몇 분을 치면 문단이 여럿이라, 문단마다 늘어놓지 않는다.
fn kinds_ui(ui: &mut egui::Ui, palette: &Palette, summary: &Summary<'_>) {
    title(ui, "종류별", palette.dim);
    let dim = |text: &str| RichText::new(text).color(palette.dim);
    let usual = |kind: Kind| summary.standing.recent_kinds[kind as usize];
    Grid::new("result_kinds")
        .num_columns(5)
        .spacing([14.0, 6.0])
        .show(ui, |ui| {
            for name in ["종류", "친 글쇠", "속도", "정확도", "평소보다"] {
                ui.label(dim(name));
            }
            ui.end_row();
            for &(kind, tally) in &summary.kinds {
                ui.label(kind.name());
                ui.label(format!("{}타", tally.needed));
                // 글쇠 한둘만 친 종류는 속도를 낼 수 없다.
                if tally.timed == 0 {
                    ui.label(dim("–"));
                } else {
                    ui.label(format!("{:.0} 타/분", tally.per_minute()));
                }
                ui.label(format!("{:.1}%", tally.accuracy() * 100.0));
                match usual(kind).filter(|_| tally.timed > 0) {
                    Some(usual) => {
                        let gap = gap(tally.per_minute(), usual.speed, 0, FASTER);
                        gap_label(ui, palette, gap);
                    }
                    None => {
                        ui.label(dim("–"));
                    }
                }
                ui.end_row();
            }
        });
    ui.add_space(4.0);
    let small = |text: String| RichText::new(text).small().color(palette.dim);
    let english = summary
        .kinds
        .iter()
        .find(|(kind, _)| *kind == Kind::English);
    if let Some((_, tally)) = english.filter(|(_, tally)| tally.timed > 0) {
        ui.label(small(format!("영문은 {:.0} WPM입니다.", tally.wpm())));
    }
    if summary.kinds.iter().any(|&(kind, _)| usual(kind).is_some()) {
        let words = "평소는 그 종류를 친 최근 연습들의 평균 속도입니다.";
        ui.label(small(words.to_owned()));
    }
}

/// 자주 틀린 글쇠와, 틀린 낱말들. 낱말에는 틀리기 시작한 글자를 표시한다.
fn mistakes_ui(ui: &mut egui::Ui, palette: &Palette, summary: &Summary<'_>) {
    title(ui, "틀린 곳", palette.dim);
    if summary.misses.is_empty() {
        ui.label(RichText::new("틀린 글쇠 없이 끝냈습니다.").color(palette.good));
        return;
    }
    let keys: Vec<String> = summary
        .misses
        .iter()
        .take(5)
        .map(|&(key, count)| format!("{} {count}번", key_name(key)))
        .collect();
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("자주 틀린 글쇠").color(palette.dim));
        ui.label(keys.join(" · "));
    });
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
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("틀린 낱말").color(palette.dim));
        for (word, slips) in summary.slips.iter().take(SLIPS_SHOWN) {
            let mut job = LayoutJob::default();
            // 낱말 뒤의 빈칸에서 틀렸으면 빈칸까지 보인다.
            let shown = word.chars().chain(std::iter::once(' '));
            for (at, c) in shown.enumerate() {
                let slipped = slips.contains(&at);
                if c == ' ' && !slipped {
                    continue;
                }
                let color = if slipped { palette.bad } else { palette.text };
                job.append(c.encode_utf8(&mut [0; 4]), 0.0, format(color, slipped));
            }
            // 낱말마다 작은 딱지에 담아, 어디서 어디까지가 한 낱말인지 보이게 한다.
            Frame::new()
                .fill(palette.card)
                .corner_radius(4.0)
                .inner_margin(egui::Margin::symmetric(6, 2))
                .show(ui, |ui| ui.label(job));
        }
    });
    if let Some(more) = summary.slips.len().checked_sub(SLIPS_SHOWN + 1) {
        let note = format!("틀린 낱말이 {}개 더 있습니다.", more + 1);
        ui.label(RichText::new(note).small().color(palette.dim));
    }
}

/// 창 맨 위: 무슨 연습의 결과인지, 그리고 새로 세운 기록이나 기록에 남기지 않은 까닭.
fn header_ui(ui: &mut egui::Ui, palette: &Palette, summary: &Summary<'_>) {
    let standing = summary.standing;
    let total = summary.total;
    ui.horizontal_wrapped(|ui| {
        let title = format!("{}분 연습 결과", summary.minutes);
        ui.label(RichText::new(title).heading().color(palette.plain));
        let pages = format!("문단 {}개를 끝냈습니다", summary.pages);
        ui.label(RichText::new(pages).color(palette.dim));
        // 새 기록은 끝까지 한 연습으로만 세운다.
        let beats =
            |best: Option<f64>, now: f64| summary.saved() && best.is_some_and(|best| now > best);
        let fastest = beats(
            standing.fastest.map(|best| best.speed()),
            total.per_minute(),
        );
        let cleanest = beats(
            standing.cleanest.map(|best| best.accuracy()),
            total.accuracy(),
        );
        let news = match (fastest, cleanest) {
            (true, true) => "새 기록 · 지금까지 가장 빠르고 가장 정확합니다",
            (true, false) => "새 기록 · 지금까지 가장 빠릅니다",
            (false, true) => "새 기록 · 지금까지 가장 정확합니다",
            (false, false) => return,
        };
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::new(news).color(palette.good));
        });
    });
    if let Some(after) = summary.stopped_after {
        let note = format!(
            "{} 만에 그만두었습니다. 정한 시간을 끝까지 채운 연습만 기록에 남깁니다.",
            span(after)
        );
        ui.label(RichText::new(note).color(palette.dim));
    }
    if let Some(trouble) = summary.trouble {
        ui.label(RichText::new(trouble).color(palette.bad));
    }
}

/// 다음 행동을 고르는 단추들. 누른 것이 있으면 돌려준다.
/// `one_row`가 아니면 단추들이 다음 줄로 넘어갈 수 있다.
fn choices_ui(ui: &mut egui::Ui, palette: &Palette, one_row: bool) -> Option<Choice> {
    fn carry_on(ui: &mut egui::Ui, palette: &Palette, picked: &mut Option<Choice>) {
        let again = RichText::new("한 번 더 (Enter)").color(palette.card);
        if ui
            .add(egui::Button::new(again).fill(palette.caret))
            .clicked()
        {
            *picked = Some(Choice::Again);
        }
        let others = [
            ("같은 글 다시 (R)", Choice::Retry),
            ("다시 보기 (V)", Choice::Replay),
            ("시간 바꾸기 (T)", Choice::ChangeTime),
        ];
        for (label, choice) in others {
            if ui.button(label).clicked() {
                *picked = Some(choice);
            }
        }
    }
    // 끝내는 단추에는 글쇠를 두지 않는다. 잘못 눌러 프로그램이 닫히면 되돌릴 수 없다.
    fn quit(ui: &mut egui::Ui, picked: &mut Option<Choice>) {
        if ui.button("종료하기").clicked() {
            *picked = Some(Choice::Quit);
        }
    }
    let mut picked = None;
    if one_row {
        ui.horizontal(|ui| {
            carry_on(ui, palette, &mut picked);
            // 다른 단추들과 떨어뜨려 오른쪽 끝에 둔다.
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                quit(ui, &mut picked);
            });
        });
    } else {
        ui.horizontal_wrapped(|ui| {
            carry_on(ui, palette, &mut picked);
            quit(ui, &mut picked);
        });
    }
    picked
}

/// 결과 창을 그린다. 단추를 눌렀으면 고른 행동을 돌려준다.
pub fn show(ctx: &egui::Context, palette: &Palette, summary: &Summary<'_>) -> Option<Choice> {
    let shown = ((summary.age / APPEAR) as f32).clamp(0.0, 1.0);
    if shown < 1.0 {
        ctx.request_repaint();
    }
    let id = Id::unique("result");
    let frame = Frame::popup(&ctx.global_style()).multiply_with_opacity(shown);
    let screen = ctx.content_rect();
    let around = frame.total_margin().sum() + egui::Vec2::splat(2.0 * EDGE);
    let width = (screen.width() - around.x).clamp(240.0, WIDEST);
    let modal = Modal::new(id)
        // 떠오르는 정도를 여기서 직접 정하므로 egui의 것은 끈다.
        .area(Modal::default_area(id).fade_in(false))
        .frame(frame)
        .backdrop_color(Color32::from_black_alpha((BACKDROP * shown) as u8));
    let modal = modal.show(ctx, |ui| {
        ui.set_opacity(shown);
        ui.set_width(width);
        header_ui(ui, palette, summary);
        ui.add_space(GAP);
        tiles_ui(ui, palette, summary);
        // 창이 낮으면 조언도 아래 구획들과 함께 내려 보는 곳에 넣는다.
        // 늘 보이는 것이 많으면 내려 볼 자리가 한두 줄밖에 남지 않는다.
        let roomy = screen.height() >= ROOMY;
        let counsel = |ui: &mut egui::Ui| {
            if let Some((advice, reason)) = &summary.counsel {
                section(ui, palette, 0.0, |ui| {
                    counsel_ui(ui, palette, advice, reason);
                });
                ui.add_space(GAP);
            }
        };
        ui.add_space(GAP);
        if roomy {
            counsel(ui);
        }

        // 창이 작으면 아래 구획들이 화면을 넘는다. 이 부분만 내려 볼 수 있게 해서
        // 큰 숫자와 조언, 단추는 늘 보이게 한다.
        let one_row = width >= BUTTONS_FIT;
        let buttons = GAP + BUTTON_ROW * if one_row { 1.0 } else { 2.0 };
        let above = ui.cursor().top() - ui.min_rect().top();
        let room = screen.height() - around.y - above - buttons;
        let room = room.max(BUTTON_ROW);
        egui::ScrollArea::vertical()
            .auto_shrink([false, true])
            .max_height(room)
            // 창이 처음 뜰 때 자리가 모자라 보여도 줄이지 않는다. 줄이면 창이 몇 장면에 걸쳐 자란다.
            .min_scrolled_height(room)
            .show(ui, |ui| {
                if !roomy {
                    counsel(ui);
                }
                if width >= SIDE_BY_SIDE {
                    let mut compare = |ui: &mut egui::Ui| compare_ui(ui, palette, summary);
                    let mut kinds = |ui: &mut egui::Ui| kinds_ui(ui, palette, summary);
                    side_by_side(ui, palette, "tables", &mut [&mut compare, &mut kinds]);
                } else {
                    section(ui, palette, 0.0, |ui| compare_ui(ui, palette, summary));
                    ui.add_space(GAP);
                    section(ui, palette, 0.0, |ui| kinds_ui(ui, palette, summary));
                }
                ui.add_space(GAP);
                section(ui, palette, 0.0, |ui| mistakes_ui(ui, palette, summary));
            });
        ui.add_space(GAP);
        choices_ui(ui, palette, one_row)
    });
    modal.inner
}

#[cfg(test)]
mod tests {
    use super::*;
    use keyboard_battle::records::Record;

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
    fn ago_needs_no_calendar() {
        assert_eq!(ago(0), "방금");
        assert_eq!(ago(59), "방금");
        assert_eq!(ago(60), "1분 전");
        assert_eq!(ago(3599), "59분 전");
        assert_eq!(ago(3600), "1시간 전");
        assert_eq!(ago(86_399), "23시간 전");
        assert_eq!(ago(86_400), "1일 전");
        assert_eq!(ago(40 * 86_400), "40일 전");
    }

    #[test]
    fn long_times_are_written_in_hours_and_minutes() {
        assert_eq!(hours_minutes(0), "0분");
        assert_eq!(hours_minutes(45), "45분");
        assert_eq!(hours_minutes(60), "1시간");
        assert_eq!(hours_minutes(135), "2시간 15분");
    }

    #[test]
    fn a_gap_is_put_in_words_and_no_gap_is_called_the_same() {
        let said = |now, before, decimals, words| gap(now, before, decimals, words);
        assert_eq!(
            said(318.4, 300.0, 0, FASTER),
            ("18 빠름".to_owned(), Tone::Better)
        );
        assert_eq!(
            said(288.0, 300.0, 0, FASTER),
            ("12 느림".to_owned(), Tone::Worse)
        );
        assert_eq!(
            said(97.4, 98.0, 1, CLEANER),
            ("0.6 낮음".to_owned(), Tone::Worse)
        );
        assert_eq!(
            said(98.3, 98.0, 1, CLEANER),
            ("0.3 높음".to_owned(), Tone::Better)
        );
        // 반올림해서 0이면 같다고 적는다.
        assert_eq!(said(300.2, 300.0, 0, FASTER).1, Tone::Same);
        assert_eq!(said(299.8, 300.0, 0, FASTER).1, Tone::Same);
        assert_eq!(
            said(98.0, 98.0, 1, CLEANER),
            ("같음".to_owned(), Tone::Same)
        );
    }

    #[test]
    fn space_key_gets_a_readable_name() {
        assert_eq!(key_name(' '), "사이띄개");
        assert_eq!(key_name('ㅆ'), "ㅆ");
    }

    /// 흐름은 왼쪽에서 오른쪽으로 가고, 클수록 위에 놓인다.
    #[test]
    fn a_trend_runs_left_to_right_and_higher_is_up() {
        let area = Rect::from_min_max(Pos2::new(10.0, 100.0), Pos2::new(110.0, 120.0));
        let points = trend_points(&[300.0, 350.0, 325.0], area);
        assert_eq!(
            points,
            [
                Pos2::new(10.0, 120.0),
                Pos2::new(60.0, 100.0),
                Pos2::new(110.0, 110.0)
            ]
        );
        // 값이 모두 같으면 가운데 높이에 곧게 놓인다.
        let flat = trend_points(&[98.0, 98.0], area);
        assert_eq!(flat, [Pos2::new(10.0, 110.0), Pos2::new(110.0, 110.0)]);
        assert_eq!(trend_points(&[5.0], area), [Pos2::new(10.0, 110.0)]);
        assert!(trend_points(&[], area).is_empty());
    }

    fn tally(speed: u32, wrong: u32) -> Tally {
        let timed = speed * 3;
        Tally {
            needed: timed + 1,
            chars: timed / 2,
            presses: timed + 1 + wrong,
            wrong,
            timed,
            seconds: 180.0,
        }
    }

    fn record(at: u64, speed: u32, wrong: u32) -> Record {
        Record {
            at,
            minutes: 3,
            total: tally(speed, wrong),
            kinds: [tally(speed, wrong), tally(speed, wrong), Tally::default()],
        }
    }

    fn summary(standing: &Standing, stopped_after: Option<f64>) -> Summary<'_> {
        let slips = (0..20).map(|n| (format!("낱말{n}"), vec![0, 3]));
        Summary {
            minutes: 3,
            stopped_after,
            total: tally(320, 9),
            pages: 3,
            kinds: vec![
                (Kind::Hangul, tally(350, 4)),
                (Kind::English, tally(300, 5)),
                (Kind::Rust, Tally::default()),
            ],
            standing,
            today: 100_000,
            trouble: Some("기록을 저장하지 못했습니다: 시험"),
            misses: vec![('ㅆ', 3), (' ', 2)],
            slips: slips.collect(),
            repair_seconds: 12.3,
            counsel: Some(("조금 늦춰 보세요.".to_owned(), "근거입니다.".to_owned())),
            age: 10.0,
        }
    }

    /// 기록이 없을 때, 하나일 때, 많을 때, 도중에 그만두었을 때, 그리고 넓고 좁은 창에서
    /// 결과 창을 끝까지 그려 본다. 화면 없이 그리므로 모양은 보지 못하고, 멈추지 않는지만 본다.
    #[test]
    fn the_result_window_draws_for_every_kind_of_history() {
        let none = Standing::default();
        let one = Standing::of(&[record(99_000, 300, 6)]);
        let many: Vec<Record> = (0..30)
            .map(|n| record(n * 3000, 280 + 3 * n as u32, n as u32 % 7))
            .collect();
        let many = Standing::of(&many);
        let cases = [
            summary(&none, None),
            summary(&none, Some(42.0)),
            summary(&one, None),
            summary(&many, None),
            summary(&many, Some(42.0)),
        ];
        for palette in [egui::Theme::Dark, egui::Theme::Light].map(Palette::of) {
            for size in [
                [960.0, 700.0],
                [720.0, 480.0],
                [620.0, 480.0],
                [1600.0, 1000.0],
            ] {
                for case in &cases {
                    let ctx = egui::Context::default();
                    let screen = Rect::from_min_size(Pos2::ZERO, size.into());
                    // 구획의 높이를 맞추는 데 지난 화면의 값을 쓰므로 두 번 그린다.
                    for _ in 0..2 {
                        let input = egui::RawInput {
                            screen_rect: Some(screen),
                            ..Default::default()
                        };
                        let mut picked = None;
                        let mut drawn = ctx.run_ui(input, |ui| {
                            picked = show(ui.ctx(), &palette, case);
                        });
                        // 그린 것을 화면에 올리지 않으므로, 글꼴 그림은 받지 않고 버린다.
                        drawn.textures_delta.clear();
                        assert_eq!(picked, None, "누르지 않았으면 고른 것이 없다");
                    }
                }
            }
        }
    }
}
