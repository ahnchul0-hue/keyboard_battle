//! 0단계 입력 확인 화면.
//!
//! OS 입력기 상태와 무관하게, 글쇠 위치만으로 한글·영문·코드가 쳐지는지 실제 PC에서 확인한다.
//! 이 화면은 글상자를 쓰지 않으므로 OS 입력기는 켜지지 않는다.

use std::collections::VecDeque;

use eframe::egui::text::LayoutJob;
use eframe::egui::{self, Color32, Event, FontId, Key, RichText, Stroke, TextFormat};
use keyboard_battle::keymap::KeyPos;
use keyboard_battle::typed::{Script, TypedText};

const TYPING_FONT_SIZE: f32 = 24.0;
const RECENT_EVENTS: usize = 16;

struct Line {
    name: &'static str,
    script: Script,
    target: &'static str,
}

const LINES: [Line; 4] = [
    Line {
        name: "한글",
        script: Script::Hangul,
        target: "다람쥐 헌 쳇바퀴에 타고파",
    },
    Line {
        name: "한글 겹낱자",
        script: Script::Hangul,
        target: "얘, 옛 학생은 값싼 떡볶이와 닭꼬치, 짭짤한 빵을 좋아했다.",
    },
    Line {
        name: "영문",
        script: Script::Latin,
        target: "The quick brown fox jumps over the lazy dog.",
    },
    Line {
        name: "러스트 코드",
        script: Script::Latin,
        target: "let v: Vec<u8> = vec![1, 2]; println!(\"{:?}\", &v);",
    },
];

/// egui가 알려 준 글쇠를 자판 위의 자리로 바꾼다. 문자 글쇠가 아니면 `None`.
fn key_pos(key: Key) -> Option<KeyPos> {
    let legend = match key {
        Key::Backtick => '`',
        Key::Minus => '-',
        Key::Equals => '=',
        Key::OpenBracket => '[',
        Key::CloseBracket => ']',
        Key::Backslash => '\\',
        Key::Semicolon => ';',
        Key::Quote => '\'',
        Key::Comma => ',',
        Key::Period => '.',
        Key::Slash => '/',
        Key::Space => ' ',
        // 글자·숫자 글쇠는 이름이 곧 쿼티 자판에 새겨진 글자다 ("A", "7").
        _ => match *key.name().as_bytes() {
            [legend] => char::from(legend).to_ascii_lowercase(),
            _ => return None,
        },
    };
    KeyPos::from_us(legend)
}

/// OS가 글쇠 입력을 어떻게 넘겨줬는지에 대한 기록. 다른 PC에서 생긴 문제를 좁히는 데 쓴다.
#[derive(Default)]
struct InputStats {
    /// 눌린 문자 글쇠 수.
    keys: u32,
    /// 그중 OS가 문자를 함께 보내지 않은 글쇠 수. OS 입력기가 글쇠를 먼저 받았다는 뜻이다.
    taken_by_ime: u32,
    /// 자판 위의 자리를 알 수 없던 글쇠 수.
    without_position: u32,
    /// OS 입력기가 보낸 조합 이벤트 수.
    ime_events: u32,
    recent: VecDeque<String>,
}

impl InputStats {
    fn log(&mut self, entry: String) {
        if self.recent.len() == RECENT_EVENTS {
            self.recent.pop_front();
        }
        self.recent.push_back(entry);
    }

    fn verdict(&self) -> String {
        let (keys, taken_by_ime, unknown) = (self.keys, self.taken_by_ime, self.without_position);
        if unknown > 0 {
            format!(
                "자리를 알 수 없는 글쇠가 {unknown}개 있었습니다. 확인 결과를 복사해 알려 주세요."
            )
        } else if keys == 0 {
            "아직 입력이 없습니다.".to_owned()
        } else if taken_by_ime > 0 || self.ime_events > 0 {
            format!(
                "글쇠 {keys}개 중 {taken_by_ime}개는 OS 입력기가 먼저 받았지만, 글쇠 자리로 읽어 그대로 처리했습니다."
            )
        } else {
            format!("글쇠 {keys}개 모두 OS 입력기를 거치지 않고 들어왔습니다.")
        }
    }
}

struct Palette {
    good: Color32,
    bad: Color32,
    plain: Color32,
    dim: Color32,
    cursor: Color32,
}

impl Palette {
    fn of(visuals: &egui::Visuals) -> Self {
        let good = if visuals.dark_mode {
            Color32::from_rgb(120, 205, 140)
        } else {
            Color32::from_rgb(20, 125, 65)
        };
        Self {
            good,
            bad: visuals.error_fg_color,
            plain: visuals.strong_text_color(),
            dim: visuals.weak_text_color(),
            cursor: visuals.selection.bg_fill,
        }
    }
}

/// 친 글을 목표 문장과 견주어 색을 입힌다. 조합 중인 글자에는 밑줄을, 그 밖에는 끝에 커서를 둔다.
fn typed_job(typed: &TypedText, target: &str, palette: &Palette) -> LayoutJob {
    let format = |color, underline| TextFormat {
        font_id: FontId::monospace(TYPING_FONT_SIZE),
        color,
        underline: Stroke::new(underline, color),
        ..Default::default()
    };
    let mut job = LayoutJob::default();
    let mut expected = target.chars();
    for c in typed.done().chars() {
        // 틀린 사이띄개도 보이도록 틀린 글자에는 밑줄을 긋는다.
        let section = if expected.next() == Some(c) {
            format(palette.good, 0.0)
        } else {
            format(palette.bad, 2.0)
        };
        job.append(c.encode_utf8(&mut [0; 4]), 0.0, section);
    }
    match typed.composing() {
        Some(c) => job.append(c.encode_utf8(&mut [0; 4]), 0.0, format(palette.plain, 2.0)),
        None => {
            let mut cursor = format(palette.plain, 0.0);
            cursor.background = palette.cursor;
            job.append(" ", 0.0, cursor);
        }
    }
    job
}

#[derive(Default)]
pub struct CheckApp {
    /// 지금 칠 문장의 번호. `LINES.len()`이면 모두 통과한 상태다.
    line: usize,
    typed: TypedText,
    /// 네 문장을 끝까지 통과한 횟수.
    rounds: u32,
    stats: InputStats,
    show_recent: bool,
    /// 화면을 그리는 데 쓰인 그래픽 장치. 확인 결과에 적는다.
    renderer: String,
}

impl CheckApp {
    fn handle_events(&mut self, events: &[Event]) {
        for (index, event) in events.iter().enumerate() {
            match event {
                Event::Key {
                    key,
                    physical_key,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    let shift = modifiers.shift;
                    self.stats
                        .log(format!("Key {key:?} 자리={physical_key:?} shift={shift}"));
                    if modifiers.ctrl || modifiers.alt || modifiers.command {
                        continue;
                    }
                    match key {
                        Key::Escape => self.typed.clear(),
                        Key::Backspace => self.typed.backspace(),
                        Key::Enter if self.line == LINES.len() => self.line = 0,
                        _ => {
                            let os_sent_text =
                                matches!(events.get(index + 1), Some(Event::Text(_)));
                            self.press(*physical_key, shift, os_sent_text);
                        }
                    }
                }
                Event::Text(text) => self.stats.log(format!("Text {text:?}")),
                Event::Ime(ime) => {
                    self.stats.ime_events += 1;
                    self.stats.log(format!("{ime:?}"));
                }
                _ => {}
            }
        }
        if LINES
            .get(self.line)
            .is_some_and(|line| self.typed.text() == line.target)
        {
            self.typed.clear();
            self.line += 1;
            if self.line == LINES.len() {
                self.rounds += 1;
            }
        }
    }

    fn press(&mut self, physical_key: Option<Key>, shift: bool, os_sent_text: bool) {
        let Some(line) = LINES.get(self.line) else {
            return;
        };
        let Some(key) = physical_key else {
            self.stats.without_position += 1;
            return;
        };
        if let Some(pos) = key_pos(key) {
            self.stats.keys += 1;
            self.stats.taken_by_ime += u32::from(!os_sent_text);
            self.typed.press(pos, shift, line.script);
        }
    }

    /// 다른 사람이 붙여 넣어 보낼 수 있는 확인 결과.
    fn report(&self) -> String {
        let InputStats {
            keys,
            taken_by_ime,
            without_position,
            ime_events,
            recent,
        } = &self.stats;
        let mut report = format!(
            "keyboard_battle {} ({} {})\n그래픽: {}\n통과한 문장 {}/{}, 완주 {}회\n\
             문자 글쇠 {keys} / 입력기가 먼저 받음 {taken_by_ime} / 자리 모름 {without_position} / \
             입력기 조합 이벤트 {ime_events}\n지금 친 글: {:?}\n최근 입력:\n",
            env!("CARGO_PKG_VERSION"),
            std::env::consts::OS,
            std::env::consts::ARCH,
            self.renderer,
            self.line,
            LINES.len(),
            self.rounds,
            self.typed.text(),
        );
        for entry in recent {
            report.push_str("  ");
            report.push_str(entry);
            report.push('\n');
        }
        report
    }

    fn lines_ui(&self, ui: &mut egui::Ui, palette: &Palette) {
        for (index, line) in LINES.iter().enumerate() {
            let (status, color) = match index.cmp(&self.line) {
                std::cmp::Ordering::Less => ("통과", palette.good),
                std::cmp::Ordering::Equal => ("지금", palette.plain),
                std::cmp::Ordering::Greater => ("대기", palette.dim),
            };
            ui.label(RichText::new(format!("{status} · {}", line.name)).color(color));
            let target_color = if index == self.line {
                palette.plain
            } else {
                palette.dim
            };
            let target = RichText::new(line.target)
                .monospace()
                .size(TYPING_FONT_SIZE);
            ui.label(target.color(target_color));
            if index == self.line {
                ui.label(typed_job(&self.typed, line.target, palette));
            }
            ui.add_space(12.0);
        }
    }
}

impl eframe::App for CheckApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        if self.renderer.is_empty() {
            self.renderer = frame.wgpu_render_state().map_or_else(
                || "알 수 없음".to_owned(),
                |state| {
                    let info = state.adapter.get_info();
                    format!("{:?} · {}", info.backend, info.name)
                },
            );
        }
        let events = ui.input(|input| input.events.clone());
        self.handle_events(&events);
        let palette = Palette::of(ui.visuals());

        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("키보드 배틀 · 0단계 입력 확인");
            ui.add_space(4.0);
            ui.label(
                "한/영 키를 누르지 말고 아래 문장을 그대로 치세요. \
                 OS 입력기가 한글 상태든 영문 상태든 똑같이 쳐져야 합니다.",
            );
            ui.label(RichText::new("Backspace 지우기 · Esc 줄 비우기").color(palette.dim));
            ui.add_space(16.0);

            self.lines_ui(ui, &palette);

            if self.line == LINES.len() {
                let rounds = self.rounds;
                ui.label(RichText::new(format!("네 문장을 모두 통과했습니다 ({rounds}회째).")).color(palette.good));
                ui.label("OS 입력기를 반대 상태(한글 ↔ 영문)로 바꾼 뒤 Enter를 눌러 한 번 더 확인해 주세요.");
                ui.add_space(12.0);
            }

            ui.separator();
            ui.label(self.stats.verdict());
            ui.horizontal(|ui| {
                if ui.button("확인 결과 복사").clicked() {
                    ui.ctx().copy_text(self.report());
                }
                ui.checkbox(&mut self.show_recent, "최근 입력 보기");
            });
            if self.show_recent {
                for entry in &self.stats.recent {
                    ui.label(RichText::new(entry).monospace().color(palette.dim));
                }
            }
        });

        // 버튼이 초점을 쥐면 사이띄개와 Enter가 그 버튼을 눌러 버리므로, 초점을 늘 비워 둔다.
        ui.memory_mut(|memory| {
            if let Some(id) = memory.focused() {
                memory.surrender_focus(id);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use keyboard_battle::keymap::qwerty;

    #[test]
    fn every_character_key_has_exactly_one_egui_key() {
        let mut legends: Vec<char> = Key::ALL
            .iter()
            .filter_map(|&key| key_pos(key))
            .map(|pos| qwerty(pos, false))
            .collect();
        legends.sort_unstable();
        let mut expected: Vec<char> = KeyPos::all().map(|pos| qwerty(pos, false)).collect();
        expected.sort_unstable();
        assert_eq!(legends, expected);
        assert_eq!(key_pos(Key::R), KeyPos::from_us('r'));
        assert_eq!(key_pos(Key::Num7), KeyPos::from_us('7'));
        assert_eq!(key_pos(Key::F1), None);
        assert_eq!(key_pos(Key::Enter), None);
    }

    fn press(key: Key, shift: bool) -> Vec<Event> {
        let modifiers = egui::Modifiers {
            shift,
            ..Default::default()
        };
        let key = Event::Key {
            key,
            physical_key: Some(key),
            pressed: true,
            repeat: false,
            modifiers,
        };
        vec![key, Event::Text(String::new())]
    }

    #[test]
    fn typing_a_line_moves_on_to_the_next() {
        let mut app = CheckApp::default();
        // "다람쥐 헌 쳇바퀴에 타고파"를 두벌식으로 치는 손놀림.
        for legend in "ekfkawnl gjs cptqkznldp xkrhvk".chars() {
            let key = *Key::ALL
                .iter()
                .find(|&&key| key_pos(key) == KeyPos::from_us(legend))
                .expect("문자 글쇠");
            app.handle_events(&press(key, false));
        }
        assert_eq!(app.line, 1);
        assert_eq!(app.typed.text(), "");
        assert_eq!((app.stats.keys, app.stats.taken_by_ime), (30, 0));
    }

    #[test]
    fn keys_the_ime_took_first_are_still_typed() {
        let mut app = CheckApp::default();
        // OS 입력기가 먼저 받은 글쇠는 문자(Text) 없이 글쇠 자리만 온다.
        let without_text = |key| vec![press(key, false).swap_remove(0)];
        app.handle_events(&without_text(Key::E));
        app.handle_events(&without_text(Key::K));
        assert_eq!(app.typed.text(), "다");
        assert_eq!((app.stats.keys, app.stats.taken_by_ime), (2, 2));
    }
}
