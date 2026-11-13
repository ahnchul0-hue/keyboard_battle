//! 연습 화면.
//!
//! 먼저 몇 분 동안 연습할지 고른다. 그 시간 동안 글이 한 쪽씩 이어서 나온다. 한 쪽은 같은 종류의
//! 문장을 이은 문단이고, 다 칠 때까지 한 블럭에 그대로 있다. 시간이 다 되면 결과 창을 띄운다.
//! 이 화면은 글상자를 쓰지 않으므로 OS 입력기는 켜지지 않는다. 연습 시간의 숫자도 글쇠로 직접 읽는다.

use std::time::Duration;

use eframe::egui::text::LayoutJob;
use eframe::egui::{self, Align, Event, FontId, Key, Layout, RichText, Stroke, TextFormat};
use keyboard_battle::coach::{self, Advice, Ledger, Review};
use keyboard_battle::measure::Tally;
use keyboard_battle::player::Player;
use keyboard_battle::records::{Record, Standing};
use keyboard_battle::round::Round;
use keyboard_battle::texts::{Deck, Kind};
use keyboard_battle::tour;

use crate::input::{InputStats, key_pos};
use crate::result::{self, Choice, Summary, clock};
use crate::settings::{self, Settings};
use crate::store::{self, Store};
use crate::theme::{self, Palette};
use crate::view::{self, Look, Page};

/// 그냥 Enter를 눌렀을 때의 연습 시간(분).
const DEFAULT_MINUTES: u32 = 3;
/// 넣을 수 있는 가장 긴 연습 시간(분). 숫자는 두 자리까지 받는다.
const LONGEST_MINUTES: u32 = 99;
/// 숫자를 치지 않고 단추로 바로 고를 수 있는 연습 시간(분).
const PRESETS: [u32; 4] = [3, 5, 10, 15];
/// 한 쪽을 끝냈을 때 다음 글의 블럭이 올라오는 데 걸리는 시간(초).
const SLIDE: f64 = 0.2;
/// 결과 창은 치던 손이 이만큼(초) 쉰 뒤에야 글쇠를 받는다.
/// 시간이 다 된 줄 모르고 이어서 친 글쇠가 R이나 V로 읽혀 결과를 못 보고 지나치는 일을 막는다.
const RESULT_GRACE: f64 = 0.8;
/// 연습 시간을 넣는 칸의 너비(점).
const MINUTES_BOX: f32 = 220.0;
/// 속도는 이만큼(초) 친 뒤부터 보여 준다. 처음 한두 글쇠로 낸 속도는 터무니없이 크다.
const LIVE_SPEED_AFTER: f64 = 2.0;
/// 한 쪽을 끝내고 넘어온 블럭의 테두리가 초록으로 빛나는 시간(초).
const PASS_GLOW: f64 = 0.6;
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

/// 한 쪽을 막 끝낸 일.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Passed {
    /// 그 쪽의 마지막 글 번호.
    last: usize,
    /// 끝낸 시각(초).
    at: f64,
    /// 다음 글의 블럭을 아래에서 올라오게 할지. 화면을 내려 보고 있었으면 그러지 않는다.
    slide: bool,
}

/// 방금 끝낸 연습을 되짚어 본 것.
struct LookBack {
    review: Review,
    /// 다음에 바꿔 볼 한 가지.
    advice: Advice,
    /// 이 연습을 하기 전까지의 기록에서 뽑은, 견줄 것들.
    standing: Standing,
}

/// 연습 시간을 고르는 화면에 적는, 지금까지의 기록 한 줄.
fn record_line(standing: &Standing) -> String {
    let (Some(previous), Some(fastest)) = (standing.previous, standing.fastest) else {
        return "끝까지 한 연습은 기록에 남아, 다음 연습의 결과와 견줘 볼 수 있습니다.".to_owned();
    };
    format!(
        "지금까지 {}번 · 지난번 {:.0} 타/분, 정확도 {:.1}% · 가장 빨랐을 때 {:.0} 타/분",
        standing.count,
        previous.speed(),
        previous.accuracy() * 100.0,
        fastest.speed()
    )
}

pub struct PracticeApp {
    /// 문장을 골라 주는 곳.
    deck: Deck,
    /// 연습 시간을 고르는 중이면 지금까지 누른 숫자. 연습을 시작하면 없다.
    choosing: Option<String>,
    /// 이번 연습의 길이(분).
    minutes: u32,
    /// 이번 연습에서 치는 글들. 시간이 다 될 때까지 한 쪽씩 뒤에 이어 붙인다.
    round: Round,
    /// 이번 연습의 첫 글쇠를 누른 시각(초). 이때부터 정한 시간이 흐른다.
    started_at: Option<f64>,
    /// 연습을 끝낸 시각(초). 값이 있는 동안 결과 창이 떠 있다.
    finished_at: Option<f64>,
    /// 결과 창이 뜬 뒤에도 이어서 친 마지막 글쇠의 시각(초). 여기서부터 손이 쉬어야 결과 창이 글쇠를 받는다.
    still_typing_at: f64,
    /// 한 쪽을 막 끝냈으면 그 일. 넘어온 블럭을 잠깐 빛나게 하고, 아래에서 올라오게 한다.
    passed: Option<Passed>,
    /// 지금 치는 블럭이 차지하는 높이(점). 지난 화면에서 잰 값이다.
    slot_height: f32,
    /// 글이 창보다 길어서 화면을 내려 보고 있는지. 지난 화면에서 잰 값이다.
    scrolled: bool,
    /// 연습 시간을 바로 고르는 단추 줄의 너비(점). 지난 화면에서 잰 값으로 가운데에 놓는다.
    presets_width: f32,
    /// 마지막으로 글쇠를 눌러 친 시각(초). 치는 동안 주변을 흐리게 하는 데 쓴다.
    typed_at: Option<f64>,
    /// 그 뒤로 마우스가 움직인 거리(점).
    pointer_travel: f32,
    /// 끝까지 한 연습들의 기록. 오래된 것부터다. 프로그램을 껐다 켜도 남는다.
    records: Vec<Record>,
    /// 그 기록들에서 뽑은, 새 결과와 견줄 것들.
    standing: Standing,
    /// 기록을 두는 곳.
    store: Store,
    /// 종료하기를 골랐는지. 창으로 연 프로그램은 닫히고, 브라우저에서는 처음 화면에 인사를 띄운다.
    farewell: bool,
    /// 이번 실행에서 친 것을 글쇠별로 모은 것. 버릇이 된 실수를 찾는 데 쓴다.
    ledger: Ledger,
    /// 방금 끝낸 연습을 되짚어 본 것. 결과 창이 떠 있는 동안 있다.
    look_back: Option<LookBack>,
    /// 되감아 보이는 중이면 그 장치. 이 동안에는 화면이 이쪽의 과정을 보여 준다.
    player: Option<Player>,
    stats: InputStats,
    show_recent: bool,
    settings: Settings,
    show_settings: bool,
    /// 이번 화면 갱신에 글쇠가 눌렸는지. 눌렸으면 지금 칠 블럭이 보이는 곳으로 화면을 옮긴다.
    follow: bool,
    /// 위쪽에 보여 주는 지금까지의 기록과, 그것을 적은 시각(초)·그때 끝낸 글 수.
    live: Tally,
    live_at: f64,
    live_lines: usize,
    /// 화면을 그리는 데 쓰인 그래픽 장치. 확인 결과에 적는다.
    renderer: String,
}

impl PracticeApp {
    /// `deck`이 내는 문장으로 연습한다. 연습 시간을 고르는 화면에서 시작한다.
    /// `records`는 `store`에 남아 있던 지난 기록이고, 새 기록도 `store`에 적는다.
    pub fn new(deck: Deck, store: Store, records: Vec<Record>) -> Self {
        Self {
            deck,
            choosing: Some(String::new()),
            minutes: DEFAULT_MINUTES,
            round: Round::new(Vec::new()),
            started_at: None,
            finished_at: None,
            still_typing_at: f64::NEG_INFINITY,
            passed: None,
            slot_height: 120.0,
            scrolled: false,
            presets_width: 260.0,
            typed_at: None,
            pointer_travel: 0.0,
            standing: Standing::of(&records),
            records,
            store,
            farewell: false,
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

    /// `minutes`분 동안의 연습을 시작한다. `round`를 주면 그 글들부터, 아니면 새 글로 친다.
    /// 시간은 첫 글쇠를 누를 때부터 흐른다.
    fn begin(&mut self, minutes: u32, round: Option<Round>) {
        self.minutes = minutes;
        let length = self.settings.length.keys();
        self.round = round.unwrap_or_else(|| Round::new(self.deck.page(length)));
        self.top_up();
        self.choosing = None;
        self.started_at = None;
        self.finished_at = None;
        self.passed = None;
        self.typed_at = None;
        self.look_back = None;
        self.farewell = false;
    }

    /// 연습 시간을 고르는 화면으로 돌아간다.
    fn ask_time(&mut self) {
        self.choosing = Some(String::new());
        self.started_at = None;
        self.finished_at = None;
        self.look_back = None;
    }

    /// 지금 치는 쪽 다음에 미리 보일 쪽이 늘 하나는 있도록 새 글을 이어 붙인다.
    fn top_up(&mut self) {
        loop {
            let (lines, current) = (self.round.lines().len(), self.round.results().len());
            if current < lines && self.round.page_of(current).end < lines {
                break;
            }
            self.round
                .extend(self.deck.page(self.settings.length.keys()));
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

    /// 시간을 다 채우지 않고 `end`에 끝냈으면, 그때까지 흐른 시간(초).
    fn stopped_after(&self, end: f64) -> Option<f64> {
        let length = f64::from(self.minutes) * 60.0;
        let spent = end - self.started_at?;
        (spent + 0.001 < length).then_some(spent)
    }

    /// 연습을 `at`에 끝내고, 되짚어 본 뒤 결과 창을 띄운다.
    /// 정한 시간을 다 채운 연습이면 기록에 남긴다. 도중에 그만둔 연습은 견줄 만한 기록이 못 된다.
    fn finish(&mut self, at: f64) {
        self.finished_at = Some(at);
        self.still_typing_at = at;
        self.typed_at = None;
        let review = Review::of(&self.round);
        self.ledger.add(&self.round, &review);
        let total = self.total(at);
        let advice = coach::advise(&total, &self.round, &review, &self.ledger);
        // 견줄 것은 이번 연습을 넣기 전의 기록에서 뽑은 것이다.
        let standing = self.standing.clone();
        if self.started_at.is_some() && self.stopped_after(at).is_none() {
            let record = Record {
                at: store::now(),
                minutes: self.minutes,
                total,
                kinds: self.kinds(at),
            };
            self.records.push(record);
            self.store.save(&record, &self.records);
            self.standing = Standing::of(&self.records);
        }
        self.look_back = Some(LookBack {
            review,
            advice,
            standing,
        });
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
                        Key::Escape => self.round.rewind(now),
                        Key::Backspace => self.round.backspace(now),
                        _ => {
                            // 브라우저는 글쇠와 문자를 따로 보내지 않으므로 이 판정을 쓸 수 없다.
                            let os_sent_text = cfg!(target_arch = "wasm32")
                                || matches!(events.get(index + 1), Some(Event::Text(_)));
                            let line = self.round.results().len();
                            self.press(*physical_key, shift, os_sent_text, now);
                            // 쪽의 마지막 글을 끝냈으면 다음 쪽으로 넘어간다.
                            if self.round.results().len() == self.round.page_of(line).end {
                                self.passed = Some(Passed {
                                    last: line,
                                    at: now,
                                    slide: !self.scrolled,
                                });
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
        match choice {
            // 한 번 더는 새 글로, 같은 글 다시는 방금 친 글부터 한다.
            Choice::Again => self.begin(self.minutes, None),
            Choice::Retry => self.begin(self.minutes, Some(self.round.fresh())),
            // 되감아 보는 동안 결과는 그대로 두었다가, 끝나면 결과 창으로 돌아온다.
            Choice::Replay => {
                self.player = Some(Player::replay(&self.round, REPLAY_CAPTION, now));
            }
            Choice::ChangeTime => self.ask_time(),
            Choice::Quit => {
                self.ask_time();
                self.farewell = true;
            }
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

    /// 위쪽에 보여 주는 기록을 고쳐 적는다. 정해진 간격이 지났거나 글을 막 끝냈을 때만 바꾼다.
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

    /// 치다 만 글의 시간을 어디까지 잴지. 연습 중에는 `now`까지 잰다.
    /// 연습이 끝난 뒤에는 마지막 글쇠까지만 잰다. 시간이 다 되기를 기다리며 손을 놓고 있던 동안을
    /// 그 글을 친 시간에 넣으면, 치지도 않은 시간 때문에 속도가 깎인다.
    fn measured_until(&self, now: f64) -> f64 {
        match self.finished_at {
            Some(finished_at) => self.round.last_action_at().unwrap_or(finished_at),
            None => now,
        }
    }

    /// 이번 연습 전체의 기록. 치다 만 글도 맞게 친 데까지 들어 있다.
    fn total(&self, now: f64) -> Tally {
        self.round.so_far(self.measured_until(now))
    }

    /// 한글, 영문, 러스트를 따로 본 기록. 끝낸 글들의 기록에 치다 만 글의 기록을 더한다.
    fn kinds(&self, now: f64) -> [Tally; 3] {
        let measured_until = self.measured_until(now);
        let round = &self.round;
        Kind::ALL.map(|kind| {
            let done = round.lines().iter().zip(round.results());
            let done = done.filter(|(line, _)| line.kind == kind);
            let mut tally: Tally = done.map(|(_, tally)| *tally).sum();
            if round.current().is_some_and(|line| line.kind == kind) {
                tally = tally + round.in_progress(measured_until).unwrap_or_default();
            }
            tally
        })
    }

    fn summary(&self, now: f64) -> Summary<'_> {
        let end = self.finished_at.unwrap_or(now);
        let round = &self.round;
        let kinds = Kind::ALL.into_iter().zip(self.kinds(now));
        let look_back = self.look_back.as_ref();
        let total = self.total(now);
        Summary {
            minutes: self.minutes,
            stopped_after: self.stopped_after(end),
            total,
            pages: round.pages_done(),
            // 친 것이 있는 종류만 보인다.
            kinds: kinds.filter(|(_, tally)| tally.presses > 0).collect(),
            standing: look_back.map_or(&self.standing, |look_back| &look_back.standing),
            today: store::now(),
            trouble: self.store.trouble(),
            misses: round.misses(),
            slips: round.slipped_words(),
            repair_seconds: look_back.map_or(0.0, |look_back| look_back.review.repair_seconds),
            counsel: look_back.map(|look_back| look_back.advice.words(&total, &look_back.review)),
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
            "keyboard_battle {} ({} {})\n그래픽: {}\n{}분 연습, 끝낸 문단 {}개, 기록에 남은 연습 {}번\n\
             문자 글쇠 {keys} / 입력기가 먼저 받음 {taken_by_ime} / 자리 모름 {without_position} / \
             입력기 조합 이벤트 {ime_events}\n지금 친 글: {:?}\n최근 입력:\n",
            env!("CARGO_PKG_VERSION"),
            std::env::consts::OS,
            std::env::consts::ARCH,
            self.renderer,
            self.minutes,
            self.round.pages_done(),
            self.records.len(),
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

    /// 연습 시간과 문단 번호, 시간 막대, 그리고 지금까지의 속도와 정확도.
    /// 그만하기 단추를 눌렀으면 `true`를 돌려준다.
    fn progress_ui(&self, ui: &mut egui::Ui, palette: &Palette, now: f64) -> bool {
        let end = self.finished_at.unwrap_or(now);
        let shown = self.shown();
        let page = shown.page_number(shown.results().len()) + 1;
        let (place, fraction) = if let Some(player) = &self.player {
            (format!("문단 {page}"), player.progress())
        } else if self.choosing.is_some() {
            ("연습 시간을 고르는 중".to_owned(), 0.0)
        } else {
            let spent = self.started_at.map_or(0.0, |started_at| end - started_at);
            let fraction = spent / (f64::from(self.minutes) * 60.0);
            let place = format!("{}분 연습 · 문단 {page}", self.minutes);
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

    /// `start`번째 글에서 시작하는 쪽이, 앞 쪽을 막 끝내고 넘어온 블럭으로서 빛나는 정도 (0 ~ 1).
    fn glow(&self, start: usize, now: f64) -> f32 {
        match self.passed {
            Some(passed) if passed.last + 1 == start => {
                (1.0 - (now - passed.at) / PASS_GLOW).clamp(0.0, 1.0) as f32
            }
            _ => 0.0,
        }
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
                    let small = |text: String, color| RichText::new(text).small().color(color);
                    ui.label(small(self.store.place(), palette.dim));
                    if let Some(trouble) = self.store.trouble() {
                        ui.label(small(trouble.to_owned(), palette.bad));
                    }
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

            // 숫자가 들어가는 칸. 지금 칠 글의 블럭과 같은 테두리를 둘러, 여기에 친다는 것을 알린다.
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
            ui.add_space(22.0);
            if self.farewell {
                // 브라우저에서는 프로그램이 탭을 닫을 수 없다.
                let bye = "연습을 마쳤습니다. 이 탭은 닫아도 됩니다.";
                ui.label(RichText::new(bye).color(palette.plain));
            }
            ui.label(RichText::new(record_line(&self.standing)).color(palette.dim));
            ui.add_space(22.0);
        });
        if let Some(minutes) = picked {
            self.begin(minutes, None);
        }
        let help = "시간은 첫 글쇠를 누를 때부터 잽니다 · 한/영 키는 누르지 않습니다";
        self.footer_ui(ui, palette, chrome, help);
    }

    /// 지금 치는 쪽의 블럭과, 그 아래 다음에 칠 쪽의 미리 보기. 지금 치는 블럭의 높이를 돌려준다.
    fn pages_ui(&self, ui: &mut egui::Ui, look: &Look<'_>, now: f64) -> f32 {
        let round = self.shown();
        let (current, total) = (round.results().len(), round.lines().len());
        let Some(last) = total.checked_sub(1) else {
            return 0.0;
        };
        // 다 끝낸 과정(둘러보기의 끝)이면 마지막 쪽을 그대로 보인다.
        let page = round.page_of(current.min(last));
        let lines = &round.lines()[page.clone()];
        let title = format!("지금 · {}", lines[0].kind.name());
        // 앞 쪽을 끝냈으면 그 기록을 블럭 귀퉁이에 적어 둔다. 한 쪽에 한 번만 바뀌므로 눈을 끌지 않는다.
        let note = page.start.checked_sub(1).map(|before| {
            let passed = round.page_of(before);
            let tally: Tally = round.results()[passed].iter().copied().sum();
            format!(
                "방금 문단 {:.0} 타/분 · 정확도 {:.0}%",
                tally.per_minute(),
                (tally.accuracy() * 100.0).floor()
            )
        });
        // 블럭이 올라오는 동안에는 화면을 옮기지 않는다. 제자리에 온 뒤에 보면 된다.
        let sliding = self
            .passed
            .is_some_and(|passed| passed.slide && now - passed.at < SLIDE);
        let block = Page {
            title: &title,
            note: note.as_deref(),
            lines,
            done: current.saturating_sub(page.start),
            typed: (current < total).then(|| round.typed()),
            glow: self.glow(page.start, now),
            follow: self.follow && !sliding,
        };
        let height = view::page(ui, look, &block).rect.height();
        ui.add_space(8.0);
        if page.end < total {
            let next = &round.lines()[round.page_of(page.end)];
            let title = format!("다음 · {}", next[0].kind.name());
            view::preview(ui, look, &title, next);
            ui.add_space(8.0);
        }
        height
    }

    /// 글 블럭들과 그 아래의 도움말.
    fn blocks_ui(&mut self, ui: &mut egui::Ui, palette: &Palette, now: f64, chrome: f32) {
        let look = Look {
            palette,
            size: view::typing_size(ui.available_width()),
            marker: self.settings.marker,
            echo: self.settings.echo,
        };
        self.scrolled = ui.cursor().top() < ui.clip_rect().top() - 0.5;
        // 한 쪽을 끝내면, 미리 보이던 다음 글이 있던 자리에서 지금 칠 자리로 올라온다.
        // 갑자기 바뀌면 읽던 줄을 놓친다.
        if self.player.is_none()
            && let Some(passed) = self.passed.filter(|passed| passed.slide)
        {
            let left = (1.0 - (now - passed.at) / SLIDE).clamp(0.0, 1.0) as f32;
            if left > 0.0 {
                // 처음에 빠르고 끝에 느리게 온다.
                ui.add_space(left * left * self.slot_height);
                ui.ctx().request_repaint();
            }
        }
        self.slot_height = self.pages_ui(ui, &look, now);
        let help = "글을 정확히 다 치면 다음 글로 넘어갑니다 · \
                    Backspace 지우기 · Esc 틀린 데까지 지우기 · 한/영 키는 누르지 않습니다";
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

        if self
            .passed
            .is_some_and(|passed| now - passed.at < PASS_GLOW)
        {
            // 빛이 다 사그라들 때까지 화면을 이어서 그린다.
            ui.ctx().request_repaint();
        }
        if self.farewell && cfg!(not(target_arch = "wasm32")) {
            // 기록은 연습을 끝낼 때 이미 적어 두었다.
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
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
    use crate::settings::Length;
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

    /// 문장을 섞지 않고, 기록을 어디에도 적지 않는 프로그램. 연습 시간을 고르는 화면에 있다.
    fn fresh(records: Vec<Record>) -> PracticeApp {
        PracticeApp::new(Deck::in_order(), Store::nowhere(), records)
    }

    /// `minutes`분 연습을 막 시작한 상태. 아직 한 글쇠도 치지 않았다.
    fn practicing(minutes: u32) -> PracticeApp {
        let mut app = fresh(Vec::new());
        app.begin(minutes, None);
        app
    }

    /// 지금 치는 글의 번호와, 그 글이 든 쪽의 번호.
    fn place(app: &PracticeApp) -> (usize, usize) {
        let line = app.round.results().len();
        (line, app.round.page_number(line))
    }

    /// 지금 글을 `start`초부터 0.1초에 한 글쇠씩 끝까지 친다. 다 친 시각을 돌려준다.
    /// 한글과 영문은 글 하나가 문단 하나이고, 러스트는 코드 한 줄이다.
    fn type_line(app: &mut PracticeApp, start: f64) -> f64 {
        let line = app.round.current().expect("칠 글이 남아 있다");
        let mut now = start;
        for unit in key_units(&line.text.clone()) {
            now += 0.1;
            app.handle_events(&press(unit, true), now);
        }
        now
    }

    /// 지금 쪽을 끝까지 친다. 다 친 시각을 돌려준다.
    fn type_page(app: &mut PracticeApp, start: f64) -> f64 {
        let page = place(app).1;
        let mut now = start;
        while place(app).1 == page {
            now = type_line(app, now);
        }
        now
    }

    /// 시간이 다 될 때까지 0.1초에 한 글쇠씩 이어 친다. 연습이 끝난 시각을 돌려준다.
    fn type_until_time_is_up(app: &mut PracticeApp, start: f64) -> f64 {
        let mut now = start;
        loop {
            let line = app.round.current().expect("글은 떨어지지 않는다");
            for unit in key_units(&line.text.clone()) {
                now += 0.1;
                app.handle_events(&press(unit, true), now);
                if let Some(finished_at) = app.finished_at {
                    return finished_at;
                }
            }
        }
    }

    fn texts(app: &PracticeApp) -> Vec<String> {
        let lines = app.round.lines().iter();
        lines.map(|line| line.text.clone()).collect()
    }

    /// 프로그램은 연습 시간을 묻는 데서 시작한다. 그때 누른 문자 글쇠는 치는 것으로 받지 않는다.
    #[test]
    fn it_starts_by_asking_how_long_to_practise() {
        let mut app = fresh(Vec::new());
        assert_eq!(app.choosing.as_deref(), Some(""));
        app.handle_events(&press('ㄷ', true), 0.0);
        assert_eq!(app.stats.keys, 0);
        assert!(app.round.lines().is_empty() && app.started_at.is_none());
    }

    #[test]
    fn plain_enter_practises_for_three_minutes() {
        let mut app = fresh(Vec::new());
        app.handle_events(&named(Key::Enter), 0.0);
        assert_eq!((app.choosing.as_deref(), app.minutes), (None, 3));
        let first = app.round.page_of(0);
        assert!(
            first.end < app.round.lines().len(),
            "지금 칠 쪽과 미리 보일 다음 쪽이 있다"
        );
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
        let mut app = fresh(Vec::new());
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
        let mut app = fresh(Vec::new());
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
        let mut app = fresh(Vec::new());
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

    /// 시간은 첫 글쇠부터 흐르고, 정한 시간이 되면 치던 글을 다 치지 못했어도 끝난다.
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

    /// 글은 한 쪽씩, 한글·영문·러스트의 차례로 시간이 다 될 때까지 이어서 나온다.
    #[test]
    fn pages_keep_coming_until_the_time_is_up() {
        let mut app = practicing(10);
        let mut now = 0.0;
        for turn in 0..6 {
            let (line, page) = place(&app);
            assert_eq!(page, turn);
            let kind = app.round.lines()[line].kind;
            assert_eq!(kind, Kind::ALL[turn % 3]);
            // 한 쪽은 한 종류의 글로만 채운다.
            let lines = &app.round.lines()[app.round.page_of(line)];
            assert!(lines.iter().all(|line| line.kind == kind));
            assert_eq!(lines.len() == 1, kind != Kind::Rust, "문단은 글 하나다");

            now = type_page(&mut app, now);
            assert_eq!(app.round.pages_done(), turn + 1);
            let next = app.round.results().len();
            assert!(
                app.round.page_of(next).end < app.round.lines().len(),
                "미리 보일 다음 쪽이 늘 있다"
            );
            assert_eq!(app.finished_at, None);
        }
        let lines = texts(&app);
        let distinct: BTreeSet<&String> = lines.iter().collect();
        assert_eq!(distinct.len(), lines.len(), "같은 글이 다시 나오지 않는다");
    }

    #[test]
    fn typing_a_page_moves_on_to_the_next() {
        let mut app = practicing(3);
        let first = app.round.lines()[0].text.clone();
        type_page(&mut app, 0.0);
        assert_eq!(place(&app), (1, 1));
        assert_eq!(app.round.typed().text(), "");
        let keys = key_units(&first).len() as u32;
        assert!(
            keys >= 260,
            "문단 하나는 한참 이어서 칠 만큼 길다: {keys}타"
        );
        assert_eq!((app.stats.keys, app.stats.taken_by_ime), (keys, 0));
        assert_eq!(app.finished_at, None);
    }

    /// 쪽을 끝냈을 때만 다음 블럭이 올라온다. 쪽 안에서 줄을 끝내는 것으로는 화면이 움직이지 않는다.
    #[test]
    fn only_finishing_a_page_brings_the_next_block_up() {
        let mut app = practicing(10);
        assert_eq!(app.passed, None);
        let done = type_page(&mut app, 0.0);
        let hangul = Passed {
            last: 0,
            at: done,
            slide: true,
        };
        assert_eq!(app.passed, Some(hangul));
        let done = type_page(&mut app, done);
        assert_eq!(
            app.passed.map(|passed| (passed.last, passed.at)),
            Some((1, done))
        );

        // 러스트 쪽은 여러 줄이다. 한 줄을 끝내도 쪽은 그대로다.
        let rust = app.round.page_of(2);
        assert!(rust.len() >= 3);
        let line_done = type_line(&mut app, done);
        assert_eq!(place(&app), (3, 2));
        assert_eq!(app.passed.map(|passed| passed.at), Some(done));
        let done = type_page(&mut app, line_done);
        assert_eq!(
            app.passed.map(|passed| (passed.last, passed.at)),
            Some((rust.end - 1, done))
        );

        // 글이 창보다 길어 화면을 내려 보고 있었으면, 올라오게 하지 않고 바로 바꾼다.
        app.scrolled = true;
        type_page(&mut app, done);
        assert_eq!(app.passed.map(|passed| passed.slide), Some(false));
    }

    /// 위쪽 숫자는 글쇠마다 바뀌지 않고, 1초가 지나거나 글을 끝냈을 때만 바뀐다.
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
            "글을 끝내면 바로 고쳐 적는다"
        );

        // 연습이 끝나면 숫자는 결과 창의 기록에서 멈추고, 새 연습을 시작하면 처음으로 돌아간다.
        let end = type_until_time_is_up(&mut app, done);
        app.refresh_live(end + 100.0);
        assert_eq!(app.live, app.summary(end + 100.0).total);
        app.choose(Choice::Again, end + 100.0);
        app.refresh_live(end + 100.1);
        assert_eq!(app.live, Tally::default());
    }

    /// 앞 쪽을 끝내고 넘어온 블럭이 잠깐 빛난다.
    #[test]
    fn the_block_that_follows_a_finished_page_glows_and_then_fades() {
        let mut app = practicing(3);
        assert_eq!(app.glow(0, 0.0), 0.0);
        let done = type_page(&mut app, 0.0);
        assert_eq!(app.glow(1, done), 1.0);
        assert_eq!(app.glow(0, done), 0.0, "다른 블럭은 빛나지 않는다");
        let halfway = app.glow(1, done + PASS_GLOW / 2.0);
        assert!((halfway - 0.5).abs() < 0.01);
        assert_eq!(app.glow(1, done + PASS_GLOW), 0.0);

        // 다음 쪽을 끝내면 빛이 그다음 블럭으로 옮겨 간다.
        let done = type_page(&mut app, done + 1.0);
        assert_eq!((app.glow(1, done), app.glow(2, done)), (0.0, 1.0));
    }

    #[test]
    fn keys_the_ime_took_first_are_still_typed() {
        let mut app = practicing(3);
        // 첫 글은 '다'로 시작한다.
        app.handle_events(&press('ㄷ', false), 0.0);
        app.handle_events(&press('ㅏ', false), 0.1);
        assert_eq!(app.round.typed().text(), "다");
        assert_eq!((app.stats.keys, app.stats.taken_by_ime), (2, 2));
    }

    #[test]
    fn time_up_opens_the_result_and_enter_practises_again_with_new_pages() {
        let mut app = practicing(1);
        let end = type_until_time_is_up(&mut app, 0.0);
        assert!((end - 60.1).abs() < 1e-9, "첫 글쇠를 누른 0.1초부터 1분");
        let typed_lines = texts(&app);

        let summary = app.summary(end + 5.0);
        assert_eq!((summary.minutes, summary.stopped_after), (1, None));
        let last_key = app.round.last_action_at().expect("친 것이 있다");
        assert_eq!(summary.total, app.round.so_far(last_key));
        // 0.1초에 한 글쇠씩 1분이면 문단 둘을 끝내고 셋째를 치다 만다.
        assert_eq!(summary.pages, 2);
        assert!((summary.total.per_minute() - 600.0).abs() < 5.0);

        app.handle_events(&named(Key::Enter), end + RESULT_GRACE + 0.01);
        assert_eq!((app.minutes, app.finished_at), (1, None));
        assert!(app.started_at.is_none() && app.round.log().is_empty());
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
        assert!(app.player.is_none() && !app.round.log().is_empty());

        // 손을 쉰 뒤에 누른 글쇠는 결과 창이 받는다. R 자리의 글쇠는 같은 글을 다시 친다.
        app.handle_events(&press('ㄱ', false), now + RESULT_GRACE + 0.01);
        assert!(app.finished_at.is_none() && app.round.log().is_empty());
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
        assert!(app.finished_at.is_none() && app.round.log().is_empty());
    }

    #[test]
    fn r_practises_the_same_pages_again() {
        let mut app = practicing(1);
        let end = type_until_time_is_up(&mut app, 0.0);
        let lines = app.round.lines().to_vec();
        let pages: Vec<_> = (0..lines.len())
            .map(|line| app.round.page_of(line))
            .collect();
        // 한글 상태에서 R 자리의 글쇠는 'ㄱ'이지만, 자리로 읽으므로 똑같이 통한다.
        app.handle_events(&press('ㄱ', false), end + 1.0);
        assert_eq!((app.finished_at, app.started_at), (None, None));
        assert_eq!(&app.round.lines()[..lines.len()], lines);
        for (line, page) in pages.into_iter().enumerate() {
            assert_eq!(app.round.page_of(line), page, "쪽을 나눈 자리도 그대로다");
        }
        assert!(app.round.results().is_empty() && app.round.log().is_empty());
        assert_eq!(app.minutes, 1);
    }

    #[test]
    fn t_goes_back_to_choosing_the_time() {
        let mut app = practicing(1);
        let end = type_until_time_is_up(&mut app, 0.0);
        // 한글 상태에서 T 자리의 글쇠는 'ㅅ'이다.
        app.handle_events(&press('ㅅ', false), end + 1.0);
        assert_eq!(app.choosing.as_deref(), Some(""));
        assert!(app.finished_at.is_none() && !app.farewell);

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
        assert_eq!(app.finished_at, Some(end));

        app.play(start + 1000.0);
        assert!(app.player.is_none());
        assert_eq!(app.finished_at, Some(end), "결과 창으로 돌아온다");
        assert_eq!(app.round.results(), record);
        assert_eq!(app.records.len(), 1, "다시 본다고 기록이 늘지는 않는다");
    }

    /// 둘러보기는 따로 노는 과정을 보여 주고, 끝나면 있던 곳으로 그대로 돌아온다.
    #[test]
    fn f1_runs_the_tour_and_comes_back_to_where_it_was() {
        // 연습 시간을 고르는 화면에서.
        let mut app = fresh(Vec::new());
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
        // 문단과 문단 사이여도 연습 시간은 흐르고 있다.
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
        let mut app = fresh(Vec::new());
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
        let _ = app.summary(end + 1.0);
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

    /// 끝까지 한 연습은 기록에 남고, 다음 결과는 그 기록과 견준다.
    #[test]
    fn a_practice_run_to_the_end_is_recorded_and_the_next_is_compared_with_it() {
        let mut app = practicing(1);
        assert!(app.records.is_empty() && app.standing.previous.is_none());
        let end = type_until_time_is_up(&mut app, 0.0);
        assert_eq!(app.records.len(), 1);
        let first = app.records[0];
        let summary = app.summary(end);
        assert_eq!((first.minutes, first.total), (1, summary.total));
        // 종류별 기록을 더하면 전체 기록이 된다.
        let kinds: Tally = first.kinds.iter().copied().sum();
        assert_eq!(
            (kinds.presses, kinds.needed, kinds.timed),
            (first.total.presses, first.total.needed, first.total.timed)
        );
        // 첫 결과에는 견줄 기록이 없다. 결과 창이 떠 있는 동안 이번 기록과 견주지는 않는다.
        assert_eq!(summary.standing.count, 0);
        assert!(summary.standing.previous.is_none());
        assert_eq!(app.standing.previous, Some(first));

        app.handle_events(&named(Key::Enter), end + 1.0);
        let end = type_until_time_is_up(&mut app, end + 2.0);
        assert_eq!(app.records.len(), 2);
        let summary = app.summary(end);
        assert_eq!(summary.standing.previous, Some(first));
        assert_eq!(summary.standing.fastest, Some(first));
        assert_eq!((summary.standing.count, summary.standing.minutes), (1, 1));
        // 0.1초에 한 글쇠씩 쳤다.
        assert!((first.speed() - 600.0).abs() < 5.0);
        assert_eq!(first.accuracy(), 1.0);
        assert_eq!(app.standing.count, 2);
    }

    /// 켤 때 받은 지난 기록이 처음 화면과 첫 결과에 쓰인다.
    #[test]
    fn records_from_earlier_runs_are_there_from_the_start() {
        let app = fresh(Vec::new());
        assert!(record_line(&app.standing).starts_with("끝까지 한 연습은 기록에 남아"));

        let mut earlier = practicing(1);
        type_until_time_is_up(&mut earlier, 0.0);
        let record = earlier.records[0];
        let mut app = fresh(vec![record, record]);
        assert_eq!(app.standing.count, 2);
        let line = record_line(&app.standing);
        assert!(line.starts_with("지금까지 2번 · 지난번 6"), "{line}");

        app.begin(1, None);
        let end = type_until_time_is_up(&mut app, 0.0);
        assert_eq!(app.summary(end).standing.count, 2);
        assert_eq!(app.records.len(), 3);
    }

    #[test]
    fn stopping_before_the_first_key_goes_back_to_choosing_the_time() {
        let mut app = practicing(5);
        app.stop(3.0);
        assert_eq!(app.choosing.as_deref(), Some(""));
        assert!(app.finished_at.is_none() && app.records.is_empty());
    }

    /// 도중에 그만두면 거기까지의 결과를 보이지만, 기록에는 남기지 않는다.
    #[test]
    fn stopping_early_shows_the_result_so_far_and_records_nothing() {
        let mut app = practicing(5);
        for (index, unit) in "ㄷㅏㄹㅏㅁㅈ".chars().enumerate() {
            app.handle_events(&press(unit, true), 0.1 * (index + 1) as f64);
        }
        app.stop(2.0);
        assert_eq!(app.finished_at, Some(2.0));

        let summary = app.summary(3.0);
        let spent = summary.stopped_after.expect("시간을 다 채우지 않았다");
        assert!((spent - 1.9).abs() < 1e-9);
        assert_eq!((summary.pages, summary.total.presses), (0, 6));
        assert!(summary.counsel.is_some());
        assert!(app.records.is_empty() && app.standing.count == 0);
    }

    /// 시간이 다 되기를 기다리며 손을 놓고 있던 동안은, 치다 만 글을 친 시간에 넣지 않는다.
    #[test]
    fn waiting_for_the_time_to_run_out_does_not_slow_the_record() {
        let mut app = practicing(1);
        // 0.1초에 한 글쇠씩 여섯 글쇠만 치고, 시간이 다 될 때까지 기다린다.
        for (index, unit) in "ㄷㅏㄹㅏㅁㅈ".chars().enumerate() {
            app.handle_events(&press(unit, true), 0.1 * (index + 1) as f64);
        }
        app.handle_events(&[], 200.0);
        assert!(app.finished_at.is_some());
        let total = app.summary(200.0).total;
        assert_eq!((total.needed, total.presses), (6, 6));
        assert!((total.seconds - 0.5).abs() < 1e-9);
        assert!((total.per_minute() - 600.0).abs() < 1e-6);
        // 시간은 다 채웠으므로 기록에 남는다.
        assert_eq!(app.records.len(), 1);
        assert_eq!(app.records[0].total, total);
    }

    /// 결과는 한글, 영문, 러스트로 나눠 보이고, 치다 만 글도 그 종류에 넣는다.
    #[test]
    fn the_result_splits_the_record_by_kind() {
        let mut app = practicing(5);
        // 한글 문단과 영문 문단을 끝내고, 러스트를 두 글쇠 치다가 그만둔다.
        let now = type_page(&mut app, 0.0);
        let now = type_page(&mut app, now);
        app.handle_events(&press('l', true), now + 0.1);
        app.handle_events(&press('e', true), now + 0.2);
        app.stop(now + 1.0);

        let summary = app.summary(now + 1.0);
        let kinds: Vec<Kind> = summary.kinds.iter().map(|&(kind, _)| kind).collect();
        assert_eq!(kinds, Kind::ALL);
        assert_eq!(summary.pages, 2);
        let rust = summary.kinds[2].1;
        assert_eq!((rust.presses, rust.needed), (2, 2));
        // 종류별 기록을 더하면 전체 기록이 된다.
        let sum: Tally = summary.kinds.iter().map(|&(_, tally)| tally).sum();
        assert_eq!(
            (sum.presses, sum.needed, sum.timed),
            (
                summary.total.presses,
                summary.total.needed,
                summary.total.timed
            )
        );
        assert!((sum.seconds - summary.total.seconds).abs() < 1e-9);

        // 치지 않은 종류는 결과에 넣지 않는다.
        let mut app = practicing(5);
        app.handle_events(&press('ㄷ', true), 0.1);
        app.stop(1.0);
        let summary = app.summary(1.0);
        let kinds: Vec<Kind> = summary.kinds.iter().map(|&(kind, _)| kind).collect();
        assert_eq!(kinds, [Kind::Hangul]);
    }

    /// 종료하기는 단추로만 고른다. 고르면 기록은 이미 남아 있고, 프로그램은 처음 화면으로 물러난다.
    #[test]
    fn quitting_is_chosen_only_with_the_button() {
        let mut app = practicing(1);
        let end = type_until_time_is_up(&mut app, 0.0);
        // Q 자리(ㅂ)를 비롯해, 정해 둔 글쇠가 아니면 결과 창은 그대로다.
        for (index, unit) in "ㅂㅁㅋ".chars().enumerate() {
            app.handle_events(&press(unit, false), end + 2.0 + index as f64);
            assert_eq!(app.finished_at, Some(end));
        }
        app.handle_events(&named(Key::Escape), end + 6.0);
        assert!(app.finished_at == Some(end) && !app.farewell);

        app.choose(Choice::Quit, end + 7.0);
        assert!(app.farewell && app.finished_at.is_none());
        assert_eq!(app.choosing.as_deref(), Some(""));
        assert_eq!(app.records.len(), 1);

        // 브라우저에서는 창이 닫히지 않는다. 다시 연습을 시작하면 인사를 치운다.
        app.handle_events(&named(Key::Enter), end + 8.0);
        assert!(!app.farewell && app.choosing.is_none());
    }

    /// 문단 길이는 설정을 따르고, 바꾼 길이는 다음에 내는 문단부터 쓰인다.
    #[test]
    fn the_paragraph_length_follows_the_setting() {
        let keys_of = |app: &PracticeApp, line: usize| -> usize {
            let lines = &app.round.lines()[app.round.page_of(line)];
            lines.iter().map(|line| key_units(&line.text).len()).sum()
        };
        let mut lengths = Vec::new();
        for length in [Length::Short, Length::Normal, Length::Long] {
            let mut app = fresh(Vec::new());
            app.settings.length = length;
            app.begin(3, None);
            let keys = keys_of(&app, 0);
            assert!(keys + 12 >= length.keys(), "{length:?}: {keys}타");
            lengths.push(keys);
        }
        assert!(lengths[0] < lengths[1] && lengths[1] < lengths[2]);
        assert!(lengths[2] >= 2 * lengths[0]);

        // 이미 나와 있는 문단은 그대로 두고, 그다음 문단부터 길어진다.
        let mut app = practicing(10);
        let (first, second) = (keys_of(&app, 0), keys_of(&app, 1));
        app.settings.length = Length::Long;
        let done = type_page(&mut app, 0.0);
        type_page(&mut app, done);
        assert_eq!((keys_of(&app, 0), keys_of(&app, 1)), (first, second));
        let rust = app.round.page_of(2).end;
        assert!(keys_of(&app, rust) >= Length::Long.keys() - 12);
    }
}
