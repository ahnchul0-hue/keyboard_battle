//! 연습 화면.
//!
//! 먼저 몇 분 동안 연습할지 고른다. 그 시간 동안 문장이 이어서 나오고, 문장을 블럭으로 늘어놓아
//! 지금 칠 문장 아래에 친 글을 보여 준다. 시간이 다 되면 결과 창을 띄운다.
//! 이 화면은 글상자를 쓰지 않으므로 OS 입력기는 켜지지 않는다. 연습 시간의 숫자도 글쇠로 직접 읽는다.

use std::time::Duration;

use eframe::egui::text::LayoutJob;
use eframe::egui::{self, Align, Event, FontId, Key, Layout, RichText, Stroke, TextFormat};
use keyboard_battle::coach::{self, Advice, Ledger, Review};
use keyboard_battle::measure::Tally;
use keyboard_battle::player::Player;
use keyboard_battle::round::Round;
use keyboard_battle::texts::{Deck, Kind, Sentence};
use keyboard_battle::tour;

use crate::input::{InputStats, key_pos};
use crate::result::{self, Choice, Summary, clock};
use crate::settings::{self, Settings};
use crate::theme::{self, Palette};
use crate::view::{self, Look, Stage};

/// 그냥 Enter를 눌렀을 때의 연습 시간(분).
const DEFAULT_MINUTES: u32 = 3;
/// 넣을 수 있는 가장 긴 연습 시간(분). 숫자는 두 자리까지 받는다.
const LONGEST_MINUTES: u32 = 99;
/// 숫자를 치지 않고 단추로 바로 고를 수 있는 연습 시간(분).
const PRESETS: [u32; 4] = [3, 5, 10, 15];
/// 화면에 늘어놓는 문장 블럭 수. 방금 끝낸 문장 하나, 지금 칠 문장, 이어서 칠 문장 둘이다.
const SHOWN_LINES: usize = 4;
/// 문장을 끝냈을 때 블럭들이 한 칸 올라오는 데 걸리는 시간(초).
const SLIDE: f64 = 0.15;
/// 결과 창은 치던 손이 이만큼(초) 쉰 뒤에야 글쇠를 받는다.
/// 시간이 다 된 줄 모르고 이어서 친 글쇠가 R이나 V로 읽혀 결과를 못 보고 지나치는 일을 막는다.
const RESULT_GRACE: f64 = 0.8;
/// 연습 시간을 넣는 칸의 너비(점).
const MINUTES_BOX: f32 = 220.0;
/// 속도는 이만큼(초) 친 뒤부터 보여 준다. 처음 한두 글쇠로 낸 속도는 터무니없이 크다.
const LIVE_SPEED_AFTER: f64 = 2.0;
/// 문장을 끝낸 블럭의 테두리가 초록으로 빛나는 시간(초).
const PASS_GLOW: f64 = 0.35;
/// 마지막 글쇠를 누르고 이만큼(초) 지나면 치기를 멈춘 것으로 본다.
const FOCUS_HOLD: f64 = 2.0;
/// 치는 동안 제목·단추·도움말을 이만큼 흐리게 한다 (0이면 그대로, 1이면 안 보임).
const FOCUS_FADE: f32 = 0.55;
/// 글쇠와 글쇠 사이에 마우스가 이보다(점) 많이 움직이면 단추를 찾는 것으로 본다.
/// 손바닥이 터치패드를 스친 정도는 넘긴다.
const POINTER_REACH: f32 = 16.0;
/// 방금 친 것을 되감아 보이는 동안 화면에 띄우는 설명.
const REPLAY_CAPTION: &str = "다시 보기";
/// 위쪽의 속도·정확도를 고쳐 적는 간격(초). 글쇠마다 바뀌는 숫자는 치는 눈을 끌어당긴다.
const LIVE_EVERY: f64 = 1.0;

/// 숫자 글쇠가 나타내는 숫자.
fn digit_of(key: Key) -> Option<char> {
    const DIGITS: [Key; 10] = [
        Key::Num0,
        Key::Num1,
        Key::Num2,
        Key::Num3,
        Key::Num4,
        Key::Num5,
        Key::Num6,
        Key::Num7,
        Key::Num8,
        Key::Num9,
    ];
    let digit = DIGITS.iter().position(|&digit| digit == key)?;
    char::from_digit(digit as u32, 10)
}

/// 연습 시간으로 누른 숫자가 뜻하는 분. 아무것도 누르지 않았으면 기본 시간이고,
/// 0분처럼 받을 수 없는 값이면 `None`이다.
fn minutes_typed(typed: &str) -> Option<u32> {
    if typed.is_empty() {
        return Some(DEFAULT_MINUTES);
    }
    let minutes = typed.parse().ok()?;
    (1..=LONGEST_MINUTES).contains(&minutes).then_some(minutes)
}

pub struct PracticeApp {
    /// 문장을 골라 주는 곳.
    deck: Deck,
    /// 연습 시간을 고르는 중이면 지금까지 누른 숫자. 연습을 시작하면 없다.
    choosing: Option<String>,
    /// 이번 연습의 길이(분).
    minutes: u32,
    /// 이번 연습에서 치는 문장들. 시간이 다 될 때까지 뒤에 이어 붙인다.
    round: Round,
    /// 이번 연습의 첫 글쇠를 누른 시각(초). 이때부터 정한 시간이 흐른다.
    started_at: Option<f64>,
    /// 연습을 끝낸 시각(초). 값이 있는 동안 결과 창이 떠 있다.
    finished_at: Option<f64>,
    /// 결과 창이 뜬 뒤에도 이어서 친 마지막 글쇠의 시각(초). 여기서부터 손이 쉬어야 결과 창이 글쇠를 받는다.
    still_typing_at: f64,
    /// 막 끝낸 문장의 번호와 끝낸 시각(초). 그 블럭을 잠깐 빛나게 한다.
    passed_at: Option<(usize, f64)>,
    /// 블럭들이 한 칸 올라오기 시작한 시각(초).
    slid_at: Option<f64>,
    /// 블럭 하나가 차지하는 높이(점). 지난 화면에서 잰 값이다.
    slot_height: f32,
    /// 연습 시간을 바로 고르는 단추 줄의 너비(점). 지난 화면에서 잰 값으로 가운데에 놓는다.
    presets_width: f32,
    /// 마지막으로 글쇠를 눌러 친 시각(초). 치는 동안 주변을 흐리게 하는 데 쓴다.
    typed_at: Option<f64>,
    /// 그 뒤로 마우스가 움직인 거리(점).
    pointer_travel: f32,
    /// 이번 실행에서 끝낸 연습들의 기록.
    history: Vec<Tally>,
    /// 이번 실행에서 친 것을 글쇠별로 모은 것. 버릇이 된 실수를 찾는 데 쓴다.
    ledger: Ledger,
    /// 방금 끝낸 연습을 되짚어 본 것과, 다음에 바꿔 볼 한 가지. 결과 창이 떠 있는 동안 있다.
    look_back: Option<(Review, Advice)>,
    /// 되감아 보이는 중이면 그 장치. 이 동안에는 화면이 이쪽의 과정을 보여 준다.
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

impl PracticeApp {
    /// `deck`이 내는 문장으로 연습한다. 연습 시간을 고르는 화면에서 시작한다.
    pub fn new(deck: Deck) -> Self {
        Self {
            deck,
            choosing: Some(String::new()),
            minutes: DEFAULT_MINUTES,
            round: Round::new(Vec::new()),
            started_at: None,
            finished_at: None,
            still_typing_at: f64::NEG_INFINITY,
            passed_at: None,
            slid_at: None,
            slot_height: 120.0,
            presets_width: 260.0,
            typed_at: None,
            pointer_travel: 0.0,
            history: Vec::new(),
            ledger: Ledger::default(),
            look_back: None,
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

    /// `minutes`분 동안의 연습을 시작한다. `lines`를 주면 그 문장들부터, 아니면 새 문장으로 친다.
    /// 시간은 첫 글쇠를 누를 때부터 흐른다.
    fn begin(&mut self, minutes: u32, lines: Option<Vec<Sentence>>) {
        self.minutes = minutes;
        self.round = Round::new(lines.unwrap_or_else(|| self.deck.deal()));
        self.top_up();
        self.choosing = None;
        self.started_at = None;
        self.finished_at = None;
        self.passed_at = None;
        self.slid_at = None;
        self.typed_at = None;
        self.look_back = None;
    }

    /// 지금 칠 문장 뒤에 보여 줄 문장이 모자라지 않게 새 문장을 이어 붙인다.
    fn top_up(&mut self) {
        while self.round.lines().len() < self.round.results().len() + SHOWN_LINES {
            self.round.extend(self.deck.deal());
        }
    }

    /// 이번 연습이 끝나는 시각(초). 첫 글쇠를 누르기 전에는 없다.
    fn deadline(&self) -> Option<f64> {
        let length = f64::from(self.minutes) * 60.0;
        self.started_at.map(|started_at| started_at + length)
    }

    /// 정한 시간이 다 됐으면 연습을 끝낸다. 늦게 알아차렸더라도 끝난 시각은 정한 그 시각이다.
    fn tick(&mut self, now: f64) {
        if self.finished_at.is_none()
            && let Some(deadline) = self.deadline()
            && now >= deadline
        {
            self.finish(deadline);
        }
    }

    /// 연습을 `at`에 끝내고, 되짚어 본 뒤 결과 창을 띄운다.
    fn finish(&mut self, at: f64) {
        self.finished_at = Some(at);
        self.still_typing_at = at;
        self.typed_at = None;
        let review = Review::of(&self.round);
        self.ledger.add(&self.round, &review);
        let total = self.total(at);
        let advice = coach::advise(&total, &self.round, &review, &self.ledger);
        self.look_back = Some((review, advice));
    }

    /// 연습을 도중에 그만둔다. 친 것이 있으면 거기까지의 결과를 보이고,
    /// 아직 한 글쇠도 치지 않았으면 연습 시간을 고르는 데로 돌아간다.
    fn stop(&mut self, now: f64) {
        if self.started_at.is_some() {
            self.finish(now);
        } else {
            self.choosing = Some(String::new());
        }
    }

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
        // 시간이 다 된 뒤에 들어온 글쇠는 치는 것으로 받지 않는다.
        self.tick(now);
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
                    if self.finished_at.is_some() {
                        // 치던 손이 잠깐 쉰 뒤에야 결과 창의 글쇠를 받는다. 쉬지 않고 이어 친 글쇠는
                        // 치던 글로 보고 버린다. Enter는 문장에 들지 않으므로 이어 친 것으로 치지 않는다.
                        if now - self.still_typing_at >= RESULT_GRACE {
                            self.result_key(*key, *physical_key, now);
                        } else if *key != Key::Enter {
                            self.still_typing_at = now;
                        }
                        continue;
                    }
                    if self.choosing.is_some() {
                        self.choose_key(*key, *physical_key, now);
                        continue;
                    }
                    self.typed_at = Some(now);
                    self.pointer_travel = 0.0;
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
                                // 둘째 문장부터는 끝낼 때마다 블럭들이 한 칸씩 올라온다.
                                if line >= 1 {
                                    self.slid_at = Some(now);
                                }
                                self.top_up();
                            }
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
    }

    /// 문자 글쇠 하나를 친다. `os_sent_text`는 OS가 이 글쇠의 문자도 함께 보냈는지다.
    fn press(&mut self, physical_key: Option<Key>, shift: bool, os_sent_text: bool, now: f64) {
        let Some(key) = physical_key else {
            self.stats.without_position += 1;
            return;
        };
        if let Some(pos) = key_pos(key) {
            self.stats.keys += 1;
            self.stats.taken_by_ime += u32::from(!os_sent_text);
            // 연습 시간은 첫 글쇠를 누를 때부터 흐른다.
            self.started_at.get_or_insert(now);
            self.round.press(pos, shift, now);
        }
    }

    /// 연습 시간을 고르는 동안의 글쇠: 숫자, 지우기, Enter.
    /// 숫자는 OS 입력기 상태와 무관하도록 글쇠 자리로 먼저 읽는다.
    fn choose_key(&mut self, key: Key, physical_key: Option<Key>, now: f64) {
        let Some(typed) = &mut self.choosing else {
            return;
        };
        let digit = physical_key.and_then(digit_of).or_else(|| digit_of(key));
        if key == Key::Enter || physical_key == Some(Key::Enter) {
            if let Some(minutes) = minutes_typed(typed) {
                self.begin(minutes, None);
            }
        } else if key == Key::Backspace {
            typed.pop();
        } else if key == Key::Escape {
            typed.clear();
        } else if key == Key::F1 {
            self.start_tour(now);
        } else if let Some(digit) = digit
            && typed.len() < 2
        {
            typed.push(digit);
        }
    }

    /// 결과 창이 떠 있을 때의 글쇠. R, V, T는 OS 입력기 상태와 무관하도록 글쇠 자리로 읽는다.
    fn result_key(&mut self, key: Key, physical_key: Option<Key>, now: f64) {
        if key == Key::Enter {
            self.choose(Choice::Again, now);
        } else if physical_key == Some(Key::R) {
            self.choose(Choice::Retry, now);
        } else if physical_key == Some(Key::V) {
            self.choose(Choice::Replay, now);
        } else if physical_key == Some(Key::T) {
            self.choose(Choice::ChangeTime, now);
        }
    }

    /// 결과 창에서 고른 대로 한다.
    fn choose(&mut self, choice: Choice, now: f64) {
        if choice == Choice::Replay {
            // 되감아 보는 동안 결과는 그대로 두었다가, 끝나면 결과 창으로 돌아온다.
            self.player = Some(Player::replay(&self.round, REPLAY_CAPTION, now));
            return;
        }
        self.history.push(self.total(now));
        match choice {
            // 한 번 더는 새 문장으로, 다시 치기는 방금 친 문장부터 한다.
            Choice::Retry => self.begin(self.minutes, Some(self.round.lines().to_vec())),
            Choice::ChangeTime => {
                self.choosing = Some(String::new());
                self.started_at = None;
                self.finished_at = None;
                self.look_back = None;
            }
            Choice::Again | Choice::Replay => self.begin(self.minutes, None),
        }
    }

    /// 둘러보기를 시작한다. 연습 시간이 흐르는 동안에는 그 시간이 흘러가 버리므로 시작하지 않는다.
    fn start_tour(&mut self, now: f64) {
        if self.can_tour() {
            self.player = Some(tour::tour(now));
        }
    }

    /// 지금 치는 중이어서 주변을 흐리게 할 때인지.
    fn focusing(&self, now: f64) -> bool {
        let typing = self.typed_at.is_some_and(|at| now - at < FOCUS_HOLD);
        self.settings.focus && typing && self.player.is_none() && self.finished_at.is_none()
    }

    /// 마우스가 `travel`점 움직였거나 단추가 눌렸음(`pressed`)을 알린다.
    /// 치다가 마우스를 뚜렷이 움직였으면 단추를 찾는 것이므로, 흐리게 하던 것을 바로 되돌린다.
    fn pointer_moved(&mut self, travel: f32, pressed: bool) {
        self.pointer_travel += travel;
        if pressed || self.pointer_travel > POINTER_REACH {
            self.typed_at = None;
        }
    }

    fn can_tour(&self) -> bool {
        self.player.is_none() && self.finished_at.is_none() && self.started_at.is_none()
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
        // 연습이 끝난 뒤에는 결과 창에 적는 기록과 같은 데서 멈춘다.
        let own_clock = self.measured_until(now);
        let clock = self.player.as_ref().map_or(own_clock, Player::clock);
        let lines = self.shown().results().len();
        let so_far = self.shown().so_far(clock);
        let due = now - self.live_at >= LIVE_EVERY;
        if due || lines != self.live_lines || so_far.presses < self.live.presses {
            (self.live, self.live_at, self.live_lines) = (so_far, now, lines);
        }
    }

    /// 치다 만 문장의 시간을 어디까지 잴지. 연습 중에는 `now`까지 잰다.
    /// 연습이 끝난 뒤에는 마지막 글쇠까지만 잰다. 시간이 다 되기를 기다리며 손을 놓고 있던 동안을
    /// 그 문장을 친 시간에 넣으면, 치지도 않은 시간 때문에 속도가 깎인다.
    fn measured_until(&self, now: f64) -> f64 {
        match self.finished_at {
            Some(finished_at) => self.round.last_action_at().unwrap_or(finished_at),
            None => now,
        }
    }

    /// 이번 연습 전체의 기록. 치다 만 문장도 맞게 친 데까지 들어 있다.
    fn total(&self, now: f64) -> Tally {
        self.round.so_far(self.measured_until(now))
    }

    fn summary(&self, now: f64) -> Summary<'_> {
        let end = self.finished_at.unwrap_or(now);
        let measured_until = self.measured_until(now);
        let round = &self.round;
        // 종류마다, 끝낸 문장들의 기록에 치다 만 문장의 기록을 더한다.
        let mut kinds = Vec::new();
        for kind in [Kind::Hangul, Kind::English, Kind::Rust] {
            let done = round.lines().iter().zip(round.results());
            let done = done.filter(|(line, _)| line.kind == kind);
            let sentences = done.clone().count();
            let mut tally: Tally = done.map(|(_, tally)| *tally).sum();
            if round.current().is_some_and(|line| line.kind == kind) {
                tally = tally + round.in_progress(measured_until).unwrap_or_default();
            }
            if tally.presses > 0 {
                kinds.push((kind, sentences, tally));
            }
        }
        let slips = round.lines().iter().zip(round.slips());
        let slips = slips.filter(|(_, slips)| !slips.is_empty());
        let length = f64::from(self.minutes) * 60.0;
        let spent = self.started_at.map(|started_at| end - started_at);
        let look_back = self.look_back.as_ref();
        let total = self.total(now);
        Summary {
            minutes: self.minutes,
            stopped_after: spent.filter(|&spent| spent + 0.001 < length),
            total,
            sentences: round.results().len(),
            kinds,
            previous: self.history.last().copied(),
            best: self.history.iter().map(Tally::per_minute).reduce(f64::max),
            misses: round.misses(),
            slips: slips.map(|(line, slips)| (*line, slips)).collect(),
            repair_seconds: look_back.map_or(0.0, |(review, _)| review.repair_seconds),
            counsel: look_back.map(|(review, advice)| advice.words(&total, review)),
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
            "keyboard_battle {} ({} {})\n그래픽: {}\n{}분 연습, 끝낸 문장 {}개, 끝낸 연습 {}번\n\
             문자 글쇠 {keys} / 입력기가 먼저 받음 {taken_by_ime} / 자리 모름 {without_position} / \
             입력기 조합 이벤트 {ime_events}\n지금 친 글: {:?}\n최근 입력:\n",
            env!("CARGO_PKG_VERSION"),
            std::env::consts::OS,
            std::env::consts::ARCH,
            self.renderer,
            self.minutes,
            self.round.results().len(),
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

    /// 위쪽 오른편에 적는 지금까지의 기록. `end`는 연습이 끝났으면 끝난 시각, 아니면 지금이다.
    fn live_text(&self, end: f64) -> String {
        let so_far = self.live;
        let practicing = self.player.is_none();
        if practicing && self.choosing.is_some() {
            return String::new();
        }
        if so_far.presses == 0 {
            return if practicing {
                format!("첫 글쇠를 누르면 {}분을 잽니다", self.minutes)
            } else {
                String::new()
            };
        }
        let speed = if so_far.seconds >= LIVE_SPEED_AFTER {
            format!("{:.0}", so_far.per_minute())
        } else {
            "–".to_owned()
        };
        let accuracy = (so_far.accuracy() * 100.0).floor();
        // 연습 중에는 남은 시간을, 되감아 보이는 중에는 친 시간을 적는다.
        let time = match self.deadline().filter(|_| practicing) {
            Some(deadline) => format!("{} 남음", clock((deadline - end).max(0.0).ceil())),
            None => clock(so_far.seconds),
        };
        format!("{speed} 타/분 · 정확도 {accuracy:.0}% · {time}")
    }

    /// 연습 시간과 문장 번호, 시간 막대, 그리고 지금까지의 속도와 정확도.
    /// 그만하기 단추를 눌렀으면 `true`를 돌려준다.
    fn progress_ui(&self, ui: &mut egui::Ui, palette: &Palette, now: f64) -> bool {
        let end = self.finished_at.unwrap_or(now);
        let line = self.shown().results().len() + 1;
        let (place, fraction) = if let Some(player) = &self.player {
            (format!("문장 {line}"), player.progress())
        } else if self.choosing.is_some() {
            ("연습 시간을 고르는 중".to_owned(), 0.0)
        } else {
            let spent = self.started_at.map_or(0.0, |started_at| end - started_at);
            let fraction = spent / (f64::from(self.minutes) * 60.0);
            let place = format!("{}분 연습 · 문장 {line}", self.minutes);
            (place, fraction as f32)
        };
        let practicing =
            self.player.is_none() && self.choosing.is_none() && self.finished_at.is_none();
        let mut stop_asked = false;
        ui.horizontal(|ui| {
            ui.label(RichText::new(place).color(palette.plain));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if practicing {
                    stop_asked = ui.button("그만하기").clicked();
                }
                ui.label(RichText::new(self.live_text(end)).color(palette.dim));
            });
        });
        ui.add_space(4.0);
        view::progress_bar(ui, fraction, palette);
        stop_asked
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

    /// 문장 하나를 담은 블럭. 블럭 하나가 차지한 높이를 돌려준다.
    fn card_ui(&self, ui: &mut egui::Ui, look: &Look<'_>, index: usize, now: f64) -> f32 {
        const GAP: f32 = 6.0;
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
        ui.add_space(GAP);
        card.rect.height() + GAP
    }

    /// OS가 입력을 어떻게 넘겼는지 보는 곳. 다른 PC에서 생긴 문제를 알릴 때 쓴다.
    fn diagnostics_ui(&mut self, ui: &mut egui::Ui, palette: &Palette) {
        egui::CollapsingHeader::new("입력 진단").show(ui, |ui| {
            ui.label(
                "OS 입력기를 영문 상태와 한글 상태로 각각 두고 1분씩 쳐 본 뒤, \
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
    /// `chrome`은 치는 데 쓰이지 않는 것들(제목, 단추, 도움말)의 또렷한 정도(0 ~ 1)다.
    fn header_ui(&mut self, ui: &mut egui::Ui, palette: &Palette, now: f64, chrome: f32) {
        let can_tour = self.can_tour();
        let mut tour_asked = false;
        ui.scope(|ui| {
            ui.multiply_opacity(chrome);
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
            if self.show_settings {
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    settings::ui(ui, palette, &mut self.settings);
                });
            }
        });
        if tour_asked {
            self.start_tour(now);
        }
        ui.add_space(6.0);
        if self.progress_ui(ui, palette, now) {
            self.stop(now);
        }
        if let Some(player) = &self.player {
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(player.caption()).color(palette.caret));
                ui.label(RichText::new("· Esc를 누르면 그만둡니다").color(palette.dim));
            });
        }
        ui.add_space(10.0);
    }

    /// 화면 아래쪽의 도움말과 입력 진단. `help`는 치는 법을 적은 줄이다.
    fn footer_ui(&mut self, ui: &mut egui::Ui, palette: &Palette, chrome: f32, help: &str) {
        ui.scope(|ui| {
            ui.multiply_opacity(chrome);
            ui.label(RichText::new(help).color(palette.dim));
            if self.stats.keys == 0 && self.player.is_none() {
                // 아직 아무것도 치지 않은 사람에게만 권한다.
                let invite = "처음이면 오른쪽 위의 둘러보기(F1)가 20초 동안 쓰는 법을 보여 줍니다";
                ui.label(RichText::new(invite).color(palette.dim));
            }
            ui.add_space(6.0);
            self.diagnostics_ui(ui, palette);
        });
    }

    /// 연습 시간을 고르는 화면. 숫자는 글쇠로 넣고, 단추로 바로 고를 수도 있다.
    fn choose_ui(&mut self, ui: &mut egui::Ui, palette: &Palette, chrome: f32) {
        let typed = self.choosing.clone().unwrap_or_default();
        let mut picked = None;
        ui.vertical_centered(|ui| {
            ui.add_space(36.0);
            let question = RichText::new("몇 분 동안 연습할까요?").size(26.0);
            ui.label(question.color(palette.plain));
            ui.add_space(18.0);

            // 숫자가 들어가는 칸. 지금 칠 문장의 블럭과 같은 테두리를 둘러, 여기에 친다는 것을 알린다.
            let frame = egui::Frame::new()
                .fill(palette.card)
                .corner_radius(8.0)
                .stroke(Stroke::new(2.0, palette.caret))
                .inner_margin(egui::Margin::symmetric(28, 8));
            frame.show(ui, |ui| {
                ui.set_width(MINUTES_BOX);
                let format = |size, color| TextFormat {
                    font_id: FontId::monospace(size),
                    color,
                    ..Default::default()
                };
                let mut job = LayoutJob::default();
                if typed.is_empty() {
                    // 아무것도 넣지 않았으면, 그냥 Enter를 눌렀을 때의 시간을 흐리게 보여 준다.
                    let default = DEFAULT_MINUTES.to_string();
                    job.append(&default, 0.0, format(48.0, palette.dim));
                } else {
                    job.append(&typed, 0.0, format(48.0, palette.plain));
                }
                job.append("분", 10.0, format(22.0, palette.dim));
                ui.label(job);
            });
            ui.add_space(14.0);
            if minutes_typed(&typed).is_some() {
                let hint = format!(
                    "숫자를 누르고 Enter를 누르세요. 그냥 Enter를 누르면 {DEFAULT_MINUTES}분입니다."
                );
                ui.label(RichText::new(hint).color(palette.dim));
            } else {
                let limit = format!("1분부터 {LONGEST_MINUTES}분까지 넣을 수 있습니다.");
                ui.label(RichText::new(limit).color(palette.bad));
            }
            ui.add_space(22.0);

            // 자주 쓰는 시간은 단추로 바로 고른다. 줄 전체를 가운데에 놓으려고 지난 화면에서 잰 너비를 쓴다.
            let side = ((ui.available_width() - self.presets_width) / 2.0).max(0.0);
            let row = ui.horizontal(|ui| {
                ui.add_space(side);
                let left = ui.cursor().left();
                for minutes in PRESETS {
                    if ui.button(format!("{minutes}분")).clicked() {
                        picked = Some(minutes);
                    }
                }
                ui.min_rect().right() - left
            });
            self.presets_width = row.inner;
            ui.add_space(28.0);
        });
        if let Some(minutes) = picked {
            self.begin(minutes, None);
        }
        let help = "시간은 첫 글쇠를 누를 때부터 잽니다 · 한/영 키는 누르지 않습니다";
        self.footer_ui(ui, palette, chrome, help);
    }

    /// 문장 블럭들과 그 아래의 도움말.
    fn blocks_ui(&mut self, ui: &mut egui::Ui, palette: &Palette, now: f64, chrome: f32) {
        let look = Look {
            palette,
            size: view::typing_size(ui.available_width()),
            marker: self.settings.marker,
            echo: self.settings.echo,
        };
        // 방금 끝낸 문장 하나를 위에 남기고, 지금 칠 문장과 이어서 칠 문장들을 보인다.
        let (current, lines) = {
            let round = self.shown();
            (round.results().len(), round.lines().len())
        };
        let first = current
            .saturating_sub(1)
            .min(lines.saturating_sub(SHOWN_LINES));
        // 문장을 끝내면 블럭들이 한 칸 아래에서 제자리로 올라온다. 갑자기 바뀌면 읽던 줄을 놓친다.
        if self.player.is_none()
            && let Some(slid_at) = self.slid_at
        {
            let left = (1.0 - (now - slid_at) / SLIDE).clamp(0.0, 1.0) as f32;
            if left > 0.0 {
                // 처음에 빠르고 끝에 느리게 온다.
                ui.add_space(left * left * self.slot_height);
                ui.ctx().request_repaint();
            }
        }
        for index in first..(first + SHOWN_LINES).min(lines) {
            self.slot_height = self.card_ui(ui, &look, index, now);
        }
        let help = "문장을 정확히 다 치면 다음 문장으로 넘어갑니다 · \
                    Backspace 지우기 · Esc 줄 비우기 · 한/영 키는 누르지 않습니다";
        self.footer_ui(ui, palette, chrome, help);
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

        let (travel, pressed) = ui.input(|input| {
            let pointer = &input.pointer;
            (pointer.delta().length(), pointer.any_pressed())
        });
        self.pointer_moved(travel, pressed);
        // 치는 동안에는 치는 데 쓰이지 않는 것들을 흐리게 물리고, 멈추면 되돌린다.
        let focusing = self.focusing(now);
        let faded = ui
            .ctx()
            .animate_bool_with_time(egui::Id::unique("focus"), focusing, 0.25);
        let chrome = 1.0 - FOCUS_FADE * faded;
        if let Some(typed_at) = self.typed_at.filter(|_| focusing) {
            // 손을 멈춘 지 정해진 시간이 지나면 되돌리도록 그때 화면을 다시 그린다.
            let wait = (typed_at + FOCUS_HOLD - now).max(0.01);
            ui.ctx()
                .request_repaint_after(Duration::from_secs_f64(wait));
        }

        egui::CentralPanel::default().show(ui, |ui| {
            self.header_ui(ui, &palette, now, chrome);
            // 창이 작으면 블럭이 다 들어오지 않으므로 블럭 쪽만 내려 볼 수 있게 한다.
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if self.choosing.is_some() && self.player.is_none() {
                        self.choose_ui(ui, &palette, chrome);
                    } else {
                        self.blocks_ui(ui, &palette, now, chrome);
                    }
                });
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
        } else if let Some(deadline) = self.deadline() {
            // 글쇠를 누르지 않는 동안에도 시간은 흐른다. 남은 시간이 1초 줄 때, 위쪽 숫자를
            // 고쳐 적을 때, 시간이 다 될 때에 맞춰 화면을 다시 그린다.
            let next_second = (deadline - now).max(0.0).fract() + 0.01;
            let next_numbers = (self.live_at + LIVE_EVERY - now).max(0.05);
            let wait = next_second.min(next_numbers);
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
    use std::collections::BTreeSet;

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

    /// 문장을 섞지 않는 묶음으로 `minutes`분 연습을 막 시작한 상태. 아직 한 글쇠도 치지 않았다.
    fn practicing(minutes: u32) -> PracticeApp {
        let mut app = PracticeApp::new(Deck::in_order());
        app.begin(minutes, None);
        app
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

    /// 시간이 다 될 때까지 0.1초에 한 글쇠씩 이어 친다. 연습이 끝난 시각을 돌려준다.
    fn type_until_time_is_up(app: &mut PracticeApp, start: f64) -> f64 {
        let mut now = start;
        loop {
            let text = app.round.current().expect("문장은 떨어지지 않는다").text;
            for unit in key_units(text) {
                now += 0.1;
                app.handle_events(&press(unit, true), now);
                if let Some(finished_at) = app.finished_at {
                    return finished_at;
                }
            }
        }
    }

    fn texts(app: &PracticeApp) -> Vec<&'static str> {
        app.round.lines().iter().map(|line| line.text).collect()
    }

    /// 프로그램은 연습 시간을 묻는 데서 시작한다. 그때 누른 문자 글쇠는 치는 것으로 받지 않는다.
    #[test]
    fn it_starts_by_asking_how_long_to_practise() {
        let mut app = PracticeApp::new(Deck::in_order());
        assert_eq!(app.choosing.as_deref(), Some(""));
        app.handle_events(&press('ㄷ', true), 0.0);
        assert_eq!(app.stats.keys, 0);
        assert!(app.round.lines().is_empty() && app.started_at.is_none());
    }

    #[test]
    fn plain_enter_practises_for_three_minutes() {
        let mut app = PracticeApp::new(Deck::in_order());
        app.handle_events(&named(Key::Enter), 0.0);
        assert_eq!((app.choosing.as_deref(), app.minutes), (None, 3));
        assert!(app.round.lines().len() >= SHOWN_LINES);
        assert_eq!(
            app.deadline(),
            None,
            "첫 글쇠를 누르기 전에는 시간이 흐르지 않는다"
        );
        app.handle_events(&press('ㄷ', true), 10.0);
        assert_eq!(app.deadline(), Some(10.0 + 180.0));
    }

    #[test]
    fn typed_digits_set_the_minutes() {
        let mut app = PracticeApp::new(Deck::in_order());
        for key in [Key::Num1, Key::Num5] {
            app.handle_events(&named(key), 0.0);
        }
        assert_eq!(app.choosing.as_deref(), Some("15"));
        // 셋째 숫자는 받지 않고, 지우개로 지울 수 있다.
        app.handle_events(&named(Key::Num9), 0.0);
        assert_eq!(app.choosing.as_deref(), Some("15"));
        app.handle_events(&named(Key::Backspace), 0.0);
        app.handle_events(&named(Key::Num0), 0.0);
        assert_eq!(app.choosing.as_deref(), Some("10"));
        app.handle_events(&named(Key::Enter), 0.0);
        assert_eq!((app.choosing.as_deref(), app.minutes), (None, 10));
    }

    /// 글쇠 이름이 엉뚱하게 들어와도 숫자는 글쇠 자리로 읽는다.
    #[test]
    fn digits_are_read_by_position() {
        let mut app = PracticeApp::new(Deck::in_order());
        let key = Event::Key {
            key: Key::A,
            physical_key: Some(Key::Num7),
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::default(),
        };
        app.handle_events(&[key], 0.0);
        assert_eq!(app.choosing.as_deref(), Some("7"));
    }

    #[test]
    fn zero_minutes_is_not_accepted() {
        let mut app = PracticeApp::new(Deck::in_order());
        app.handle_events(&named(Key::Num0), 0.0);
        app.handle_events(&named(Key::Enter), 0.0);
        assert_eq!(app.choosing.as_deref(), Some("0"), "0분은 시작하지 않는다");
        // Esc는 넣던 숫자를 비운다.
        app.handle_events(&named(Key::Escape), 0.0);
        assert_eq!(app.choosing.as_deref(), Some(""));

        assert_eq!(minutes_typed(""), Some(DEFAULT_MINUTES));
        assert_eq!(minutes_typed("0"), None);
        assert_eq!(minutes_typed("00"), None);
        assert_eq!(minutes_typed("05"), Some(5));
        assert_eq!(minutes_typed("99"), Some(99));
    }

    /// 시간은 첫 글쇠부터 흐르고, 정한 시간이 되면 치던 문장을 다 치지 못했어도 끝난다.
    #[test]
    fn the_practice_ends_when_the_time_is_up() {
        let mut app = practicing(1);
        app.handle_events(&[], 100.0);
        assert!(app.started_at.is_none() && app.finished_at.is_none());
        app.handle_events(&press('ㄷ', true), 100.0);
        app.handle_events(&press('ㅏ', true), 159.9);
        app.handle_events(&[], 159.99);
        assert_eq!(app.finished_at, None);

        // 화면을 조금 늦게 그렸더라도 끝난 시각은 정한 그 시각이다.
        app.handle_events(&[], 160.3);
        assert_eq!(app.finished_at, Some(160.0));
        // 그 뒤에 누른 글쇠는 치는 것으로 받지 않는다.
        let typed = app.round.log().len();
        app.handle_events(&press('ㄹ', true), 160.4);
        assert_eq!(app.round.log().len(), typed);
        assert_eq!(app.round.typed().text(), "다");

        // 시간이 다 된 바로 그 순간에 함께 들어온 글쇠도 받지 않는다.
        let mut app = practicing(1);
        app.handle_events(&press('ㄷ', true), 0.0);
        app.handle_events(&press('ㅏ', true), 60.0);
        assert_eq!(app.finished_at, Some(60.0));
        assert_eq!(app.round.typed().text(), "ㄷ");
    }

    /// 문장은 시간이 다 될 때까지 떨어지지 않고, 그 전에는 결과 창이 뜨지 않는다.
    #[test]
    fn sentences_keep_coming_until_the_time_is_up() {
        let mut app = practicing(1);
        let mut now = 0.0;
        for done in 1..=6 {
            now = type_line(&mut app, now);
            assert_eq!(app.round.results().len(), done);
            assert!(app.round.lines().len() >= done + SHOWN_LINES);
            assert_eq!(app.finished_at, None);
        }
        let lines = texts(&app);
        let distinct: BTreeSet<&str> = lines.iter().copied().collect();
        assert_eq!(
            distinct.len(),
            lines.len(),
            "같은 문장이 다시 나오지 않는다"
        );
    }

    #[test]
    fn typing_a_line_moves_on_to_the_next() {
        let mut app = practicing(3);
        let first = app.round.lines()[0].text;
        type_line(&mut app, 0.0);
        assert_eq!(app.round.results().len(), 1);
        assert_eq!(app.round.typed().text(), "");
        let keys = key_units(first).len() as u32;
        assert_eq!((app.stats.keys, app.stats.taken_by_ime), (keys, 0));
        assert_eq!(app.finished_at, None);
    }

    /// 첫 문장을 끝낼 때는 블럭이 그대로 있고, 둘째 문장부터는 끝낼 때마다 한 칸씩 올라온다.
    #[test]
    fn blocks_slide_up_from_the_second_line_on() {
        let mut app = practicing(3);
        let done = type_line(&mut app, 0.0);
        assert_eq!(app.slid_at, None);
        let done = type_line(&mut app, done);
        assert_eq!(app.slid_at, Some(done));
    }

    /// 위쪽 숫자는 글쇠마다 바뀌지 않고, 1초가 지나거나 문장을 끝냈을 때만 바뀐다.
    #[test]
    fn live_numbers_change_once_a_second_or_when_a_line_ends() {
        let mut app = practicing(1);
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

        let mut app = practicing(1);
        let done = type_line(&mut app, 0.0);
        app.refresh_live(done);
        let first = app.round.results()[0];
        assert_eq!(
            app.live.needed, first.needed,
            "문장을 끝내면 바로 고쳐 적는다"
        );

        // 연습이 끝나면 숫자는 결과 창의 기록에서 멈추고, 새 연습을 시작하면 처음으로 돌아간다.
        let end = type_until_time_is_up(&mut app, done);
        app.refresh_live(end + 100.0);
        assert_eq!(app.live, app.summary(end + 100.0).total);
        app.choose(Choice::Again, end + 100.0);
        app.refresh_live(end + 100.1);
        assert_eq!(app.live, Tally::default());
    }

    #[test]
    fn a_line_just_passed_glows_and_then_fades() {
        let mut app = practicing(3);
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
        let mut app = practicing(3);
        // 첫 문장은 '다'로 시작한다.
        app.handle_events(&press('ㄷ', false), 0.0);
        app.handle_events(&press('ㅏ', false), 0.1);
        assert_eq!(app.round.typed().text(), "다");
        assert_eq!((app.stats.keys, app.stats.taken_by_ime), (2, 2));
    }

    #[test]
    fn time_up_opens_the_result_and_enter_practises_again_with_new_sentences() {
        let mut app = practicing(1);
        let end = type_until_time_is_up(&mut app, 0.0);
        assert!((end - 60.1).abs() < 1e-9, "첫 글쇠를 누른 0.1초부터 1분");
        let typed_lines = texts(&app);

        let summary = app.summary(end + 5.0);
        assert_eq!((summary.minutes, summary.stopped_after), (1, None));
        let last_key = app.round.last_action_at().expect("친 것이 있다");
        assert_eq!(summary.total, app.round.so_far(last_key));
        assert_eq!(summary.sentences, app.round.results().len());
        assert!(summary.sentences >= 8, "{}문장", summary.sentences);
        // 0.1초에 한 글쇠씩 쳤다.
        assert!((summary.total.per_minute() - 600.0).abs() < 5.0);

        app.handle_events(&named(Key::Enter), end + RESULT_GRACE + 0.01);
        assert_eq!((app.minutes, app.finished_at), (1, None));
        assert_eq!((app.started_at, app.history.len()), (None, 1));
        assert!(texts(&app).iter().all(|text| !typed_lines.contains(text)));
    }

    /// 시간이 다 된 줄 모르고 이어서 친 글쇠로 결과 창이 닫히면 안 된다.
    /// R, V, T 자리의 글쇠(ㄱ, ㅍ, ㅅ)는 치던 글에도 흔히 나온다.
    #[test]
    fn keys_typed_on_after_the_time_is_up_do_not_skip_the_result() {
        let mut app = practicing(1);
        let end = type_until_time_is_up(&mut app, 0.0);
        app.handle_events(&named(Key::Enter), end + 0.1);
        // 5초 동안 쉬지 않고 이어 친다.
        let mut now = end;
        for unit in "ㄱㅍㅅㄱㅏㅅㅣㄱㅍㅅ".chars().cycle().take(25) {
            now += 0.2;
            app.handle_events(&press(unit, true), now);
        }
        assert_eq!(
            (app.finished_at, app.choosing.as_deref()),
            (Some(end), None)
        );
        assert!(app.history.is_empty() && app.player.is_none());

        // 손을 쉰 뒤에 누른 글쇠는 결과 창이 받는다.
        app.handle_events(&press('ㄱ', false), now + RESULT_GRACE + 0.01);
        assert_eq!((app.finished_at, app.history.len()), (None, 1));
    }

    /// Enter는 치던 글에 들지 않으므로, 결과 창이 뜨고 조금만 지나면 받는다.
    /// 급하게 여러 번 눌러도 받는 때가 뒤로 밀리지 않는다.
    #[test]
    fn enter_works_shortly_after_the_result_appears() {
        let mut app = practicing(1);
        let end = type_until_time_is_up(&mut app, 0.0);
        for tenth in 1..=7 {
            app.handle_events(&named(Key::Enter), end + f64::from(tenth) / 10.0);
            assert_eq!(app.finished_at, Some(end), "아직 받지 않는다");
        }
        app.handle_events(&named(Key::Enter), end + RESULT_GRACE + 0.01);
        assert_eq!((app.finished_at, app.history.len()), (None, 1));
    }

    #[test]
    fn r_practises_the_same_sentences_again() {
        let mut app = practicing(1);
        let end = type_until_time_is_up(&mut app, 0.0);
        let lines = app.round.lines().to_vec();
        // 한글 상태에서 R 자리의 글쇠는 'ㄱ'이지만, 자리로 읽으므로 똑같이 통한다.
        app.handle_events(&press('ㄱ', false), end + 1.0);
        assert_eq!((app.finished_at, app.started_at), (None, None));
        assert_eq!(&app.round.lines()[..lines.len()], lines);
        assert!(app.round.results().is_empty() && app.round.log().is_empty());
        assert_eq!((app.minutes, app.history.len()), (1, 1));
    }

    #[test]
    fn t_goes_back_to_choosing_the_time() {
        let mut app = practicing(1);
        let end = type_until_time_is_up(&mut app, 0.0);
        // 한글 상태에서 T 자리의 글쇠는 'ㅅ'이다.
        app.handle_events(&press('ㅅ', false), end + 1.0);
        assert_eq!(app.choosing.as_deref(), Some(""));
        assert_eq!((app.finished_at, app.history.len()), (None, 1));

        // 다시 고른 시간으로 새 연습이 시작된다.
        app.handle_events(&named(Key::Num2), end + 2.0);
        app.handle_events(&named(Key::Enter), end + 2.1);
        assert_eq!((app.choosing.as_deref(), app.minutes), (None, 2));
        assert!(app.round.log().is_empty() && app.started_at.is_none());
    }

    /// 결과 창에서 V를 누르면 방금 친 것을 처음부터 되감아 보이고, 끝나면 결과 창으로 돌아온다.
    #[test]
    fn v_replays_the_practice_and_returns_to_the_result() {
        let mut app = practicing(1);
        let end = type_until_time_is_up(&mut app, 0.0);
        let record = app.round.results().to_vec();
        // 한글 상태에서 V 자리의 글쇠는 'ㅍ'이다.
        let start = end + 1.0;
        app.handle_events(&press('ㅍ', false), start);
        assert!(app.player.is_some());
        assert!(app.shown().results().is_empty(), "처음부터 다시 보인다");

        // 되감아 보이는 동안 친 것은 받지 않는다.
        app.handle_events(&press('ㄱ', true), start + 0.1);
        app.handle_events(&named(Key::Enter), start + 0.2);
        app.play(start + 1.0);
        assert!(!app.shown().typed().text().is_empty());
        assert!(app.history.is_empty());

        app.play(start + 1000.0);
        assert!(app.player.is_none());
        assert_eq!(app.finished_at, Some(end), "결과 창으로 돌아온다");
        assert_eq!(app.round.results(), record);
    }

    /// 둘러보기는 따로 노는 과정을 보여 주고, 끝나면 있던 곳으로 그대로 돌아온다.
    #[test]
    fn f1_runs_the_tour_and_comes_back_to_where_it_was() {
        // 연습 시간을 고르는 화면에서.
        let mut app = PracticeApp::new(Deck::in_order());
        app.handle_events(&named(Key::F1), 1.0);
        assert!(app.player.is_some());
        app.play(1.0 + 10.0);
        assert!(
            !app.shown().typed().text().is_empty(),
            "둘러보기가 스스로 친다"
        );
        app.play(1.0 + 1000.0);
        assert!(app.player.is_none());
        assert_eq!((app.choosing.as_deref(), app.stats.keys), (Some(""), 0));

        // 연습을 시작했지만 아직 첫 글쇠를 누르지 않았을 때.
        let mut app = practicing(3);
        app.handle_events(&named(Key::F1), 1.0);
        assert!(app.player.is_some());
        app.play(1.0 + 1000.0);
        assert!(app.round.log().is_empty() && app.started_at.is_none());
    }

    #[test]
    fn the_tour_does_not_start_while_the_clock_is_running() {
        let mut app = practicing(1);
        let done = type_line(&mut app, 0.0);
        // 문장과 문장 사이여도 연습 시간은 흐르고 있다.
        app.handle_events(&named(Key::F1), done + 1.0);
        assert!(app.player.is_none());

        // 결과 창이 떠 있을 때도 시작하지 않는다.
        let end = type_until_time_is_up(&mut app, done + 1.0);
        app.handle_events(&named(Key::F1), end + 5.0);
        assert!(app.player.is_none());
    }

    /// 치는 동안에만 주변을 흐리게 하고, 손을 멈추면 되돌린다.
    #[test]
    fn focus_holds_while_typing_and_lets_go_after_a_pause() {
        let mut app = practicing(3);
        assert!(!app.focusing(0.0));
        app.handle_events(&press('ㄷ', true), 10.0);
        assert!(app.focusing(10.0) && app.focusing(10.0 + FOCUS_HOLD - 0.1));
        assert!(!app.focusing(10.0 + FOCUS_HOLD));

        // 설정에서 끄면 흐리게 하지 않는다.
        app.handle_events(&press('ㅏ', true), 20.0);
        app.settings.focus = false;
        assert!(!app.focusing(20.0));

        // 결과 창이 떠 있을 때와 연습 시간을 고를 때는 흐리게 하지 않는다.
        let mut app = practicing(1);
        let end = type_until_time_is_up(&mut app, 0.0);
        assert!(!app.focusing(end));
        let mut app = PracticeApp::new(Deck::in_order());
        app.handle_events(&named(Key::Num5), 0.0);
        assert!(!app.focusing(0.0));
    }

    /// 연습을 끝내면 되짚어 본 것이 결과 창에 실리고, 다음 연습으로 넘어가면 치운다.
    #[test]
    fn the_result_brings_advice_until_the_next_practice() {
        let mut app = practicing(1);
        assert!(app.summary(0.0).counsel.is_none());
        let end = type_until_time_is_up(&mut app, 0.0);
        let summary = app.summary(end);
        // 틀리지 않고 고르게 쳤으므로 속도를 올려 보라고 권한다.
        let (advice, _) = summary.counsel.expect("연습을 끝내면 조언이 있다");
        assert!(advice.ends_with("타/분을 노려 보세요."), "{advice}");
        assert_eq!(summary.repair_seconds, 0.0);
        // 글쇠별 기록은 연습을 끝낼 때 한 번만 더한다.
        let needed = app.ledger.key('ㅏ').needed;
        assert!(needed > 0);
        app.summary(end + 1.0);
        assert_eq!(app.ledger.key('ㅏ').needed, needed);

        app.handle_events(&named(Key::Enter), end + RESULT_GRACE + 0.01);
        assert!(app.look_back.is_none());
        assert!(app.summary(end + 2.0).counsel.is_none());
    }

    #[test]
    fn reaching_for_the_mouse_brings_the_surroundings_back() {
        let mut app = practicing(3);
        app.handle_events(&press('ㄷ', true), 10.0);
        // 손바닥이 터치패드를 스친 정도로는 돌아오지 않는다.
        app.pointer_moved(POINTER_REACH / 2.0, false);
        assert!(app.focusing(10.1));
        app.pointer_moved(POINTER_REACH, false);
        assert!(!app.focusing(10.1));

        // 다시 치면 다시 흐려지고, 마우스 단추를 누르면 바로 돌아온다.
        app.handle_events(&press('ㅏ', true), 11.0);
        assert!(app.focusing(11.1));
        app.pointer_moved(0.0, true);
        assert!(!app.focusing(11.1));

        // 스친 거리는 글쇠를 누를 때마다 처음부터 다시 잰다.
        for tenth in 0..10 {
            let now = 12.0 + f64::from(tenth) / 10.0;
            app.handle_events(&press('ㅏ', true), now);
            app.pointer_moved(POINTER_REACH / 2.0, false);
            assert!(app.focusing(now));
        }
    }

    #[test]
    fn escape_stops_the_replay() {
        let mut app = practicing(1);
        let end = type_until_time_is_up(&mut app, 0.0);
        app.choose(Choice::Replay, end + 1.0);
        assert!(app.player.is_some());
        app.handle_events(&named(Key::Escape), end + 2.0);
        assert!(app.player.is_none());
        assert_eq!(app.finished_at, Some(end));
    }

    #[test]
    fn the_result_compares_with_the_previous_practice_and_the_best() {
        let mut app = practicing(1);
        assert!(app.summary(0.0).previous.is_none() && app.summary(0.0).best.is_none());
        let end = type_until_time_is_up(&mut app, 0.0);
        app.handle_events(&named(Key::Enter), end + 1.0);
        let end = type_until_time_is_up(&mut app, end + 2.0);

        let summary = app.summary(end);
        let previous = summary.previous.expect("앞 연습의 기록");
        assert_eq!(summary.best, Some(previous.per_minute()));
        // 0.1초에 한 글쇠씩 쳤다.
        assert!((previous.per_minute() - 600.0).abs() < 5.0);
        assert_eq!(previous.accuracy(), 1.0);
    }

    #[test]
    fn stopping_before_the_first_key_goes_back_to_choosing_the_time() {
        let mut app = practicing(5);
        app.stop(3.0);
        assert_eq!(app.choosing.as_deref(), Some(""));
        assert!(app.finished_at.is_none() && app.history.is_empty());
    }

    /// 도중에 그만두면 거기까지의 결과를 보인다. 치다 만 문장에서 누른 글쇠도 기록에 든다.
    #[test]
    fn stopping_early_shows_the_result_so_far() {
        let mut app = practicing(5);
        let done = type_line(&mut app, 0.0);
        // 둘째 문장은 '얘'로 시작한다.
        app.handle_events(&press('ㅇ', true), done + 0.5);
        app.stop(done + 2.0);
        assert_eq!(app.finished_at, Some(done + 2.0));

        let summary = app.summary(done + 3.0);
        let spent = summary.stopped_after.expect("시간을 다 채우지 않았다");
        assert!((spent - (done + 2.0 - 0.1)).abs() < 1e-9);
        assert_eq!(summary.sentences, 1);
        assert_eq!(summary.total.presses, app.round.results()[0].presses + 1);
        assert!(summary.counsel.is_some());
    }

    /// 시간이 다 되기를 기다리며 손을 놓고 있던 동안은, 치다 만 문장을 친 시간에 넣지 않는다.
    #[test]
    fn waiting_for_the_time_to_run_out_does_not_slow_the_record() {
        let mut app = practicing(1);
        // 첫 문장을 0.1초에 한 글쇠씩 여섯 글쇠만 치고, 시간이 다 될 때까지 기다린다.
        for (index, unit) in "ㄷㅏㄹㅏㅁㅈ".chars().enumerate() {
            app.handle_events(&press(unit, true), 0.1 * (index + 1) as f64);
        }
        app.handle_events(&[], 200.0);
        assert!(app.finished_at.is_some());
        let total = app.summary(200.0).total;
        assert_eq!((total.needed, total.presses), (6, 6));
        assert!((total.seconds - 0.5).abs() < 1e-9);
        assert!((total.per_minute() - 600.0).abs() < 1e-6);
    }

    /// 결과는 한글, 영문, 러스트로 나눠 보이고, 치다 만 문장도 그 종류에 넣는다.
    #[test]
    fn the_result_splits_the_record_by_kind() {
        let mut app = practicing(5);
        // 한글 둘과 영문 하나를 끝내고, 러스트를 두 글쇠 치다가 그만둔다.
        let mut now = 0.0;
        for _ in 0..3 {
            now = type_line(&mut app, now);
        }
        app.handle_events(&press('l', true), now + 0.1);
        app.handle_events(&press('e', true), now + 0.2);
        app.stop(now + 1.0);

        let summary = app.summary(now + 1.0);
        let sentences: Vec<(Kind, usize)> = summary
            .kinds
            .iter()
            .map(|&(kind, sentences, _)| (kind, sentences))
            .collect();
        let expected = [(Kind::Hangul, 2), (Kind::English, 1), (Kind::Rust, 0)];
        assert_eq!(sentences, expected);
        let rust = summary.kinds[2].2;
        assert_eq!((rust.presses, rust.needed), (2, 2));
        // 종류별 기록을 더하면 전체 기록이 된다.
        let sum: Tally = summary.kinds.iter().map(|&(_, _, tally)| tally).sum();
        assert_eq!(
            (sum.presses, sum.needed, sum.timed),
            (
                summary.total.presses,
                summary.total.needed,
                summary.total.timed
            )
        );
        assert!((sum.seconds - summary.total.seconds).abs() < 1e-9);
    }
}
