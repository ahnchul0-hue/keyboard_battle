//! 연습 화면.
//!
//! 과정 하나는 문장 네 개다. 문장을 블럭으로 늘어놓고 지금 칠 문장 아래에 친 글을 보여 준다.
//! 과정이 끝나면 결과 창을 띄우고, 거기서 새 문장으로 넘어간다.
//! 이 화면은 글상자를 쓰지 않으므로 OS 입력기는 켜지지 않는다.

use std::time::Duration;

use eframe::egui::{self, Align, Event, Key, Layout, RichText};
use keyboard_battle::measure::Tally;
use keyboard_battle::player::Player;
use keyboard_battle::round::Round;
use keyboard_battle::texts;
use keyboard_battle::tour;

use crate::input::{InputStats, key_pos};
use crate::result::{self, Choice, Summary, clock};
use crate::settings::{self, Settings};
use crate::theme::{self, Palette};
use crate::view::{self, Look, Stage};

/// 결과 창이 뜬 뒤 이만큼(초)은 글쇠로 창을 넘기지 못한다.
/// 문장 끝에 버릇처럼 누른 Enter 때문에 결과를 못 보고 지나치는 일을 막는다.
const RESULT_GRACE: f64 = 0.8;
/// 속도는 이만큼(초) 친 뒤부터 보여 준다. 처음 한두 글쇠로 낸 속도는 터무니없이 크다.
const LIVE_SPEED_AFTER: f64 = 2.0;
/// 문장을 끝낸 블럭의 테두리가 초록으로 빛나는 시간(초).
const PASS_GLOW: f64 = 0.35;
/// 방금 친 과정을 되감아 보이는 동안 화면에 띄우는 설명.
const REPLAY_CAPTION: &str = "다시 보기";
/// 위쪽의 속도·정확도·시간을 고쳐 적는 간격(초). 글쇠마다 바뀌는 숫자는 치는 눈을 끌어당긴다.
const LIVE_EVERY: f64 = 1.0;

pub struct PracticeApp {
    /// 지금 과정의 번호 (0부터).
    number: usize,
    round: Round,
    /// 과정을 끝낸 시각(초). 값이 있는 동안 결과 창이 떠 있다.
    finished_at: Option<f64>,
    /// 막 끝낸 문장의 번호와 끝낸 시각(초). 그 블럭을 잠깐 빛나게 한다.
    passed_at: Option<(usize, f64)>,
    /// 이번 실행에서 끝낸 과정들의 기록.
    history: Vec<Tally>,
    /// 방금 친 과정을 되감아 보이는 중이면 그 장치. 이 동안에는 화면이 이쪽의 과정을 보여 준다.
    player: Option<Player>,
    stats: InputStats,
    show_recent: bool,
    settings: Settings,
    show_settings: bool,
    /// 이번 화면 갱신에 글쇠가 눌렸는지. 눌렸으면 지금 칠 블럭이 보이는 곳으로 화면을 옮긴다.
    follow: bool,
    /// 위쪽에 보여 주는 지금까지의 기록과, 그것을 적은 시각(초)·그때 끝낸 문장 수.
    live: Tally,
    live_at: f64,
    live_lines: usize,
    /// 화면을 그리는 데 쓰인 그래픽 장치. 확인 결과에 적는다.
    renderer: String,
}

impl Default for PracticeApp {
    fn default() -> Self {
        Self {
            number: 0,
            round: Round::new(texts::round(0)),
            finished_at: None,
            passed_at: None,
            history: Vec::new(),
            player: None,
            stats: InputStats::default(),
            show_recent: false,
            settings: Settings::default(),
            show_settings: false,
            follow: false,
            live: Tally::default(),
            live_at: f64::NEG_INFINITY,
            live_lines: 0,
            renderer: String::new(),
        }
    }
}

impl PracticeApp {
    /// 이번 화면 갱신에 들어온 입력을 처리한다. `now`는 지금 시각(초)이다.
    fn handle_events(&mut self, events: &[Event], now: f64) {
        let pressed = |event: &Event| matches!(event, Event::Key { pressed: true, .. });
        self.follow = events.iter().any(pressed);
        if self.player.is_some() {
            // 되감아 보이는 동안에는 치는 것을 받지 않는다. Esc로 그만둘 수만 있다.
            let escape = |event: &Event| {
                matches!(
                    event,
                    Event::Key {
                        key: Key::Escape,
                        pressed: true,
                        ..
                    }
                )
            };
            if events.iter().any(escape) {
                self.player = None;
            }
            return;
        }
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
                    if let Some(finished_at) = self.finished_at {
                        if now - finished_at >= RESULT_GRACE {
                            self.result_key(*key, *physical_key, now);
                        }
                        continue;
                    }
                    match key {
                        Key::F1 => self.start_tour(now),
                        Key::Escape => self.round.clear_line(now),
                        Key::Backspace => self.round.backspace(now),
                        _ => {
                            // 브라우저는 글쇠와 문자를 따로 보내지 않으므로 이 판정을 쓸 수 없다.
                            let os_sent_text = cfg!(target_arch = "wasm32")
                                || matches!(events.get(index + 1), Some(Event::Text(_)));
                            let line = self.round.results().len();
                            self.press(*physical_key, shift, os_sent_text, now);
                            if self.round.results().len() > line {
                                self.passed_at = Some((line, now));
                            }
                        }
                    }
                    if self.round.is_finished() {
                        self.finished_at = Some(now);
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
    }

    fn press(&mut self, physical_key: Option<Key>, shift: bool, os_sent_text: bool, now: f64) {
        let Some(key) = physical_key else {
            self.stats.without_position += 1;
            return;
        };
        if let Some(pos) = key_pos(key) {
            self.stats.keys += 1;
            self.stats.taken_by_ime += u32::from(!os_sent_text);
            self.round.press(pos, shift, now);
        }
    }

    /// 결과 창이 떠 있을 때의 글쇠. R과 V는 OS 입력기 상태와 무관하도록 글쇠 자리로 읽는다.
    fn result_key(&mut self, key: Key, physical_key: Option<Key>, now: f64) {
        if key == Key::Enter {
            self.choose(Choice::Next, now);
        } else if physical_key == Some(Key::R) {
            self.choose(Choice::Retry, now);
        } else if physical_key == Some(Key::V) {
            self.choose(Choice::Replay, now);
        }
    }

    /// 결과 창에서 고른 대로 한다.
    fn choose(&mut self, choice: Choice, now: f64) {
        match choice {
            // 되감아 보는 동안 결과는 그대로 두었다가, 끝나면 결과 창으로 돌아온다.
            Choice::Replay => self.player = Some(Player::replay(&self.round, REPLAY_CAPTION, now)),
            Choice::Next | Choice::Retry => {
                self.history
                    .push(self.round.results().iter().copied().sum());
                if choice == Choice::Next {
                    self.number += 1;
                }
                self.round = Round::new(texts::round(self.number));
                self.finished_at = None;
                self.passed_at = None;
            }
        }
    }

    /// 둘러보기를 시작한다. 문장을 치는 도중이면 그 문장의 시간이 흘러가 버리므로 시작하지 않는다.
    fn start_tour(&mut self, now: f64) {
        if self.can_tour() {
            self.player = Some(tour::tour(now));
        }
    }

    fn can_tour(&self) -> bool {
        self.player.is_none() && self.finished_at.is_none() && !self.round.is_timing()
    }

    /// 화면에 보여 줄 과정. 되감아 보이는 중이면 그쪽의 과정이다.
    fn shown(&self) -> &Round {
        self.player.as_ref().map_or(&self.round, Player::round)
    }

    /// 되감아 보이는 장치를 `now`까지 돌린다. 다 보였으면 치운다.
    fn play(&mut self, now: f64) {
        let Some(player) = &mut self.player else {
            return;
        };
        self.follow |= player.advance(now);
        if player.is_over(now) {
            self.player = None;
        }
    }

    /// 위쪽에 보여 주는 기록을 고쳐 적는다. 정해진 간격이 지났거나 문장을 막 끝냈을 때만 바꾼다.
    fn refresh_live(&mut self, now: f64) {
        // 되감아 보이는 과정에는 본디 시각이 남아 있으므로, 그 시각으로 기록을 낸다.
        let clock = self.player.as_ref().map_or(now, Player::clock);
        let lines = self.shown().results().len();
        let so_far = self.shown().so_far(clock);
        let due = now - self.live_at >= LIVE_EVERY;
        if due || lines != self.live_lines || so_far.presses < self.live.presses {
            (self.live, self.live_at, self.live_lines) = (so_far, now, lines);
        }
    }

    fn summary(&self, now: f64) -> Summary<'_> {
        Summary {
            number: self.number + 1,
            lines: self.round.lines(),
            results: self.round.results(),
            previous: self.history.last().copied(),
            best: self.history.iter().map(Tally::per_minute).reduce(f64::max),
            misses: self.round.misses(),
            slips: self.round.slips(),
            age: self
                .finished_at
                .map_or(0.0, |finished_at| now - finished_at),
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
            "keyboard_battle {} ({} {})\n그래픽: {}\n과정 {}, 문장 {}/{}, 끝낸 과정 {}개\n\
             문자 글쇠 {keys} / 입력기가 먼저 받음 {taken_by_ime} / 자리 모름 {without_position} / \
             입력기 조합 이벤트 {ime_events}\n지금 친 글: {:?}\n최근 입력:\n",
            env!("CARGO_PKG_VERSION"),
            std::env::consts::OS,
            std::env::consts::ARCH,
            self.renderer,
            self.number + 1,
            self.round.results().len(),
            self.round.lines().len(),
            self.history.len(),
            self.round.typed().text(),
        );
        for entry in recent {
            report.push_str("  ");
            report.push_str(entry);
            report.push('\n');
        }
        report
    }

    /// 과정 번호, 문장 번호, 진행 막대, 그리고 지금까지의 속도와 정확도.
    fn progress_ui(&self, ui: &mut egui::Ui, palette: &Palette) {
        let lines = self.shown().lines().len();
        let line = (self.shown().results().len() + 1).min(lines);
        let fraction = self.shown().progress();
        ui.horizontal(|ui| {
            let percent = (fraction * 100.0).floor();
            let place = format!(
                "과정 {} · 문장 {line}/{lines} · {percent:.0}%",
                self.number + 1
            );
            ui.label(RichText::new(place).color(palette.plain));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let so_far = self.live;
                let live = if so_far.presses == 0 {
                    "첫 글쇠를 누르면 시간을 잽니다".to_owned()
                } else {
                    let speed = if so_far.seconds >= LIVE_SPEED_AFTER {
                        format!("{:.0}", so_far.per_minute())
                    } else {
                        "–".to_owned()
                    };
                    format!(
                        "{speed} 타/분 · 정확도 {:.0}% · {}",
                        (so_far.accuracy() * 100.0).floor(),
                        clock(so_far.seconds)
                    )
                };
                ui.label(RichText::new(live).color(palette.dim));
            });
        });
        ui.add_space(4.0);
        view::progress_bar(ui, fraction, palette);
    }

    /// `index`번째 블럭이 막 끝낸 문장으로서 빛나는 정도 (0 ~ 1).
    fn glow(&self, index: usize, now: f64) -> f32 {
        match self.passed_at {
            Some((line, at)) if line == index => {
                (1.0 - (now - at) / PASS_GLOW).clamp(0.0, 1.0) as f32
            }
            _ => 0.0,
        }
    }

    /// 문장 하나를 담은 블럭.
    fn card_ui(&self, ui: &mut egui::Ui, look: &Look<'_>, index: usize, now: f64) {
        let round = self.shown();
        let line = round.lines()[index];
        let is_current = round.results().len() == index;
        let stage = match round.results().get(index) {
            Some(tally) => Stage::Done {
                tally,
                glow: self.glow(index, now),
            },
            None if is_current => Stage::Current(round.typed()),
            None => Stage::Waiting,
        };
        let card = view::card(ui, look, &line, stage);
        if is_current && self.follow {
            card.scroll_to_me(Some(Align::Center));
        }
        ui.add_space(6.0);
    }

    /// OS가 입력을 어떻게 넘겼는지 보는 곳. 다른 PC에서 생긴 문제를 알릴 때 쓴다.
    fn diagnostics_ui(&mut self, ui: &mut egui::Ui, palette: &Palette) {
        egui::CollapsingHeader::new("입력 진단").show(ui, |ui| {
            ui.label(
                "OS 입력기를 영문 상태와 한글 상태로 각각 두고 한 과정씩 쳐 본 뒤, \
                 결과를 복사해 알려 주세요.",
            );
            ui.label(self.stats.verdict());
            ui.horizontal(|ui| {
                if ui.button("확인 결과 복사").clicked() {
                    ui.ctx().copy_text(self.report());
                }
                ui.checkbox(&mut self.show_recent, "최근 입력 보기");
            });
            if self.show_recent {
                egui::ScrollArea::vertical()
                    .max_height(160.0)
                    .auto_shrink([false, true])
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for entry in &self.stats.recent {
                            ui.label(RichText::new(entry).monospace().color(palette.dim));
                        }
                    });
            }
        });
    }

    /// 제목, 화면 밝기, 진행 상황. 창이 작아도 늘 보이는 곳에 둔다.
    fn header_ui(&mut self, ui: &mut egui::Ui, palette: &Palette, now: f64) {
        let can_tour = self.can_tour();
        let mut tour_asked = false;
        ui.horizontal(|ui| {
            let title = RichText::new("키보드 배틀").heading();
            ui.label(title.color(palette.plain));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.toggle_value(&mut self.show_settings, "설정");
                let tour = egui::Button::new("둘러보기 (F1)");
                tour_asked = ui.add_enabled(can_tour, tour).clicked();
                ui.separator();
                theme::switcher(ui);
            });
        });
        if tour_asked {
            self.start_tour(now);
        }
        if self.show_settings {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.set_width(ui.available_width());
                settings::ui(ui, palette, &mut self.settings);
            });
        }
        ui.add_space(6.0);
        self.progress_ui(ui, palette);
        if let Some(player) = &self.player {
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(player.caption()).color(palette.caret));
                ui.label(RichText::new("· Esc를 누르면 그만둡니다").color(palette.dim));
            });
        }
        ui.add_space(10.0);
    }

    /// 문장 블럭들과 그 아래의 도움말.
    fn blocks_ui(&mut self, ui: &mut egui::Ui, palette: &Palette, now: f64) {
        let look = Look {
            palette,
            size: view::typing_size(ui.available_width()),
            marker: self.settings.marker,
            echo: self.settings.echo,
        };
        for index in 0..self.shown().lines().len() {
            self.card_ui(ui, &look, index, now);
        }
        let help = "문장을 정확히 다 치면 다음 문장으로 넘어갑니다 · \
                    Backspace 지우기 · Esc 줄 비우기 · 한/영 키는 누르지 않습니다";
        ui.label(RichText::new(help).color(palette.dim));
        if self.stats.keys == 0 && self.player.is_none() {
            // 아직 아무것도 치지 않은 사람에게만 권한다.
            let invite = "처음이면 오른쪽 위의 둘러보기(F1)가 20초 동안 쓰는 법을 보여 줍니다";
            ui.label(RichText::new(invite).color(palette.dim));
        }
        ui.add_space(6.0);
        self.diagnostics_ui(ui, palette);
    }
}

impl eframe::App for PracticeApp {
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
        let now = ui.input(|input| input.time);
        let events = ui.input(|input| input.events.clone());
        self.handle_events(&events, now);
        self.play(now);
        self.refresh_live(now);
        let palette = Palette::of(ui.ctx().theme());

        egui::CentralPanel::default().show(ui, |ui| {
            self.header_ui(ui, &palette, now);
            // 창이 작으면 블럭이 다 들어오지 않으므로 블럭 쪽만 내려 볼 수 있게 한다.
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| self.blocks_ui(ui, &palette, now));
        });

        if let Some(player) = &self.player {
            // 다음 글쇠를 눌러 보일 때에 맞춰 화면을 다시 그린다.
            let wait = player.wait(now).max(0.01);
            ui.ctx()
                .request_repaint_after(Duration::from_secs_f64(wait));
        } else if self.finished_at.is_some() {
            let choice = result::show(ui.ctx(), &palette, &self.summary(now));
            if let Some(choice) = choice {
                self.choose(choice, now);
            }
        } else if self.round.is_timing() {
            // 글쇠를 누르지 않는 동안에도, 다음에 숫자를 고쳐 적을 때가 되면 화면을 다시 그린다.
            let wait = (self.live_at + LIVE_EVERY - now).max(0.05);
            ui.ctx()
                .request_repaint_after(Duration::from_secs_f64(wait));
        }

        if self.passed_at.is_some_and(|(_, at)| now - at < PASS_GLOW) {
            // 빛이 다 사그라들 때까지 화면을 이어서 그린다.
            ui.ctx().request_repaint();
        }

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
    use keyboard_battle::keymap::keystroke_for;
    use keyboard_battle::measure::key_units;

    /// `unit`(낱자나 문자)을 치는 글쇠 입력. `with_text`가 거짓이면 OS 입력기가 먼저 받은 것처럼
    /// 문자(Text) 없이 글쇠 자리만 보낸다.
    fn press(unit: char, with_text: bool) -> Vec<Event> {
        let (pos, shift) = keystroke_for(unit).expect("글쇠 하나로 치는 문자");
        let key = *Key::ALL
            .iter()
            .find(|&&key| key_pos(key) == Some(pos))
            .expect("문자 글쇠");
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
        let text = with_text.then(|| Event::Text(unit.to_string()));
        std::iter::once(key).chain(text).collect()
    }

    fn named(key: Key) -> Vec<Event> {
        vec![Event::Key {
            key,
            physical_key: Some(key),
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::default(),
        }]
    }

    /// 지금 문장을 `start`초부터 0.1초에 한 글쇠씩 끝까지 친다. 다 친 시각을 돌려준다.
    fn type_line(app: &mut PracticeApp, start: f64) -> f64 {
        let text = app.round.current().expect("칠 문장이 남아 있다").text;
        let mut now = start;
        for unit in key_units(text) {
            now += 0.1;
            app.handle_events(&press(unit, true), now);
        }
        now
    }

    /// 과정 하나를 끝까지 친다. 다 친 시각을 돌려준다.
    fn type_round(app: &mut PracticeApp, start: f64) -> f64 {
        let mut now = start;
        while !app.round.is_finished() {
            now = type_line(app, now);
        }
        now
    }

    #[test]
    fn typing_a_line_moves_on_to_the_next() {
        let mut app = PracticeApp::default();
        let first = app.round.lines()[0].text;
        type_line(&mut app, 0.0);
        assert_eq!(app.round.results().len(), 1);
        assert_eq!(app.round.typed().text(), "");
        let keys = key_units(first).len() as u32;
        assert_eq!((app.stats.keys, app.stats.taken_by_ime), (keys, 0));
        assert_eq!(app.finished_at, None);
    }

    /// 위쪽 숫자는 글쇠마다 바뀌지 않고, 1초가 지나거나 문장을 끝냈을 때만 바뀐다.
    #[test]
    fn live_numbers_change_once_a_second_or_when_a_line_ends() {
        let mut app = PracticeApp::default();
        app.refresh_live(0.0);
        assert_eq!(app.live.presses, 0);

        app.handle_events(&press('ㄷ', true), 5.0);
        app.refresh_live(5.0);
        assert_eq!(app.live.presses, 1);
        for (unit, now) in [('ㅏ', 5.2), ('ㄹ', 5.4), ('ㅏ', 5.6)] {
            app.handle_events(&press(unit, true), now);
            app.refresh_live(now);
        }
        assert_eq!(app.live.presses, 1, "1초가 지나기 전에는 그대로다");
        app.refresh_live(6.0);
        assert_eq!(app.live.presses, 4);

        let mut app = PracticeApp::default();
        let done = type_line(&mut app, 0.0);
        app.refresh_live(done);
        let first = app.round.results()[0];
        assert_eq!(
            app.live.needed, first.needed,
            "문장을 끝내면 바로 고쳐 적는다"
        );

        // 새 과정이 시작되면 숫자도 처음으로 돌아간다.
        type_round(&mut app, done);
        app.refresh_live(done + 100.0);
        app.choose(Choice::Next, done + 100.0);
        app.refresh_live(done + 100.1);
        assert_eq!(app.live, Tally::default());
    }

    #[test]
    fn a_line_just_passed_glows_and_then_fades() {
        let mut app = PracticeApp::default();
        assert_eq!(app.glow(0, 0.0), 0.0);
        let done = type_line(&mut app, 0.0);
        assert_eq!(app.glow(0, done), 1.0);
        assert_eq!(app.glow(1, done), 0.0, "다른 블럭은 빛나지 않는다");
        let halfway = app.glow(0, done + PASS_GLOW / 2.0);
        assert!((halfway - 0.5).abs() < 0.01);
        assert_eq!(app.glow(0, done + PASS_GLOW), 0.0);

        // 다음 문장을 끝내면 빛이 그 블럭으로 옮겨 간다.
        let done = type_line(&mut app, done + 1.0);
        assert_eq!((app.glow(0, done), app.glow(1, done)), (0.0, 1.0));
    }

    #[test]
    fn keys_the_ime_took_first_are_still_typed() {
        let mut app = PracticeApp::default();
        // 첫 문장은 '다'로 시작한다.
        app.handle_events(&press('ㄷ', false), 0.0);
        app.handle_events(&press('ㅏ', false), 0.1);
        assert_eq!(app.round.typed().text(), "다");
        assert_eq!((app.stats.keys, app.stats.taken_by_ime), (2, 2));
    }

    #[test]
    fn finishing_a_round_opens_the_result_and_enter_brings_new_sentences() {
        let mut app = PracticeApp::default();
        let first_round: Vec<&str> = app.round.lines().iter().map(|line| line.text).collect();
        let done = type_round(&mut app, 0.0);
        assert_eq!(app.finished_at, Some(done));
        assert_eq!(app.summary(0.0).results.len(), 4);

        app.handle_events(&named(Key::Enter), done + RESULT_GRACE);
        assert_eq!((app.number, app.finished_at), (1, None));
        assert_eq!(app.history.len(), 1);
        let second_round: Vec<&str> = app.round.lines().iter().map(|line| line.text).collect();
        assert!(second_round.iter().all(|text| !first_round.contains(text)));
    }

    /// 문장 끝에 버릇처럼 누른 Enter로 결과 창이 닫히면 안 된다.
    #[test]
    fn enter_right_after_finishing_does_not_skip_the_result() {
        let mut app = PracticeApp::default();
        let done = type_round(&mut app, 0.0);
        app.handle_events(&named(Key::Enter), done + 0.1);
        app.handle_events(&press('ㄱ', true), done + 0.2);
        assert_eq!((app.number, app.finished_at), (0, Some(done)));
        assert!(app.history.is_empty());
    }

    #[test]
    fn r_repeats_the_same_sentences_and_keeps_the_record() {
        let mut app = PracticeApp::default();
        let first_round = app.round.lines().to_vec();
        let done = type_round(&mut app, 0.0);
        // 한글 상태에서 R 자리의 글쇠는 'ㄱ'이지만, 자리로 읽으므로 똑같이 통한다.
        app.handle_events(&press('ㄱ', false), done + 1.0);
        assert_eq!((app.number, app.finished_at), (0, None));
        assert_eq!(app.round.lines(), first_round);
        assert!(app.round.results().is_empty());
        assert_eq!(app.history.len(), 1);
    }

    /// 결과 창에서 V를 누르면 방금 친 과정을 처음부터 되감아 보이고, 끝나면 결과 창으로 돌아온다.
    #[test]
    fn v_replays_the_round_and_returns_to_the_result() {
        let mut app = PracticeApp::default();
        let done = type_round(&mut app, 0.0);
        let record = app.round.results().to_vec();
        // 한글 상태에서 V 자리의 글쇠는 'ㅍ'이다.
        let start = done + 1.0;
        app.handle_events(&press('ㅍ', false), start);
        assert!(app.player.is_some());
        assert!(app.shown().results().is_empty(), "처음부터 다시 보인다");

        // 되감아 보이는 동안 친 것은 받지 않는다.
        app.handle_events(&press('ㄱ', true), start + 0.1);
        app.handle_events(&named(Key::Enter), start + 0.2);
        app.play(start + 1.0);
        assert!(!app.shown().results().is_empty() || !app.shown().typed().text().is_empty());
        assert_eq!((app.number, app.history.len()), (0, 0));

        app.play(start + 1000.0);
        assert!(app.player.is_none());
        assert_eq!(app.finished_at, Some(done), "결과 창으로 돌아온다");
        assert_eq!(app.round.results(), record);
    }

    /// 둘러보기는 따로 노는 과정을 보여 주고, 끝나면 치던 과정으로 그대로 돌아온다.
    #[test]
    fn f1_runs_the_tour_without_touching_the_real_round() {
        let mut app = PracticeApp::default();
        app.handle_events(&named(Key::F1), 1.0);
        assert!(app.player.is_some());
        app.play(1.0 + 10.0);
        assert!(
            !app.shown().typed().text().is_empty(),
            "둘러보기가 스스로 친다"
        );
        assert!(app.round.typed().text().is_empty() && app.round.log().is_empty());

        app.play(1.0 + 1000.0);
        assert!(app.player.is_none());
        assert_eq!((app.finished_at, app.stats.keys), (None, 0));
    }

    #[test]
    fn the_tour_does_not_start_in_the_middle_of_a_line() {
        let mut app = PracticeApp::default();
        app.handle_events(&press('ㄷ', true), 0.0);
        app.handle_events(&named(Key::F1), 1.0);
        assert!(app.player.is_none());

        // 과정을 끝내 결과 창이 떠 있을 때도 시작하지 않는다.
        let mut app = PracticeApp::default();
        let done = type_round(&mut app, 0.0);
        app.handle_events(&named(Key::F1), done + 5.0);
        assert!(app.player.is_none());
    }

    #[test]
    fn escape_stops_the_replay() {
        let mut app = PracticeApp::default();
        let done = type_round(&mut app, 0.0);
        app.choose(Choice::Replay, done + 1.0);
        assert!(app.player.is_some());
        app.handle_events(&named(Key::Escape), done + 2.0);
        assert!(app.player.is_none());
        assert_eq!(app.finished_at, Some(done));
    }

    #[test]
    fn the_result_compares_with_the_previous_round_and_the_best() {
        let mut app = PracticeApp::default();
        assert!(app.summary(0.0).previous.is_none() && app.summary(0.0).best.is_none());
        let done = type_round(&mut app, 0.0);
        app.handle_events(&named(Key::Enter), done + 1.0);
        type_round(&mut app, done + 2.0);

        let summary = app.summary(0.0);
        let previous = summary.previous.expect("앞 과정의 기록");
        assert_eq!(summary.number, 2);
        assert_eq!(summary.best, Some(previous.per_minute()));
        // 0.1초에 한 글쇠씩 쳤다.
        assert!((previous.per_minute() - 600.0).abs() < 0.01);
        assert_eq!(previous.accuracy(), 1.0);
    }
}
