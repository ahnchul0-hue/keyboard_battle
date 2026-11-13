//! 방금 친 과정을 되짚어, 다음에 무엇을 바꿔 치면 좋을지 찾는다.
//!
//! 근거는 모두 치는 사람 자신의 기록이다: 고치는 데 쓴 시간, 거듭 틀린 글쇠,
//! 낱말 사이에서 멈춘 시간, 머뭇거린 글쇠.
//! 한 번에 한 가지만 권한다. 여러 가지를 한꺼번에 고치려 하면 아무것도 고쳐지지 않는다.

use std::collections::BTreeMap;

use crate::keymap::{Finger, finger, keystroke_for, row};
use crate::measure::{Tally, common_prefix, key_units};
use crate::round::{Action, Round};

/// 정확도가 이보다 낮으면 무엇보다 먼저 속도를 늦추라고 권한다.
const CARELESS: f64 = 0.95;
/// 정확도가 이 이상이면 속도를 올려 보라고 권한다.
const PRECISE: f64 = 0.98;
/// 올릴 때는 지금 속도에서 적어도 이만큼 높인다. 조금씩 올려야 정확도가 버틴다.
const PUSH: f64 = 1.05;
/// 이번 실행에서 같은 글쇠를 이만큼 틀렸고, 그것이 쳐야 했던 횟수의 이 비율 이상이면 그 글쇠를 짚는다.
const REPEATED: u32 = 2;
const OFTEN: f64 = 0.1;
/// 이만큼은 쳐 본 글쇠가 다른 글쇠보다 이 배수 이상 걸리면 머뭇거린다고 본다.
const ENOUGH: u32 = 4;
const HESITATION: f64 = 1.6;
/// 낱말을 이만큼은 쳤고, 낱말의 첫 글쇠가 낱말 안의 글쇠보다 이 배수 이상 걸리면
/// 낱말 사이에서 멈춘다고 본다. 낱말의 첫 글쇠는 누구나 조금 더 걸리므로 넉넉히 잡는다.
const WORDS_ENOUGH: u32 = 8;
const WORD_GAP: f64 = 2.0;
/// 글쇠 하나에 걸린 시간은 이보다(초) 길게 치지 않는다. 딴 일을 하느라 멈춘 것까지 셈하지 않는다.
const LONGEST_PAUSE: f64 = 2.0;

/// 과정 하나를 되짚어 본 것.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Review {
    /// 틀린 글쇠를 누르고, 지우고, 다시 치는 데 쓴 시간(초).
    pub repair_seconds: f64,
    /// 새 글쇠를 맞게 치는 데 쓴 시간(초).
    pub clean_seconds: f64,
    /// 그렇게 친 글쇠 수. 문장의 첫 글쇠는 시계를 켜는 신호라서 세지 않는다.
    pub clean_keys: u32,
    /// 막힘 없이 이어 친 글쇠와 그 글쇠에 걸린 시간(초).
    /// 고친 직후에 친 글쇠는 평소 빠르기가 아니므로 넣지 않는다. 낱말의 첫 글쇠도 따로 센다.
    pub paces: Vec<(char, f64)>,
    /// 막힘 없이 이어 친 낱말의 첫 글쇠(빈칸 바로 다음 글쇠) 수와, 거기에 걸린 시간의 합(초).
    pub word_starts: u32,
    pub word_start_seconds: f64,
}

/// 되짚는 동안 지금 문장에 대해 기억해 둘 것.
#[derive(Default)]
struct Line {
    /// 어긋나지 않게 가장 멀리 친 자리(글쇠 수).
    best: usize,
    /// 앞선 일의 시각(초). 문장의 첫 글쇠를 누르기 전에는 없다.
    last: Option<f64>,
    /// 앞선 일이 새 글쇠를 맞게 친 것이었는지.
    flowing: bool,
}

impl Review {
    /// `round`에 적힌 일을 처음부터 다시 따라가며 시간을 나눠 본다.
    ///
    /// 글쇠 하나를 누를 때까지 걸린 시간은, 그 글쇠로 문장을 새로 한 자리 나아갔으면
    /// 맞게 친 시간이고 아니면(틀린 글쇠, 지우기, 지운 자리 다시 치기) 고친 시간이다.
    pub fn of(round: &Round) -> Self {
        let mut review = Self::default();
        let mut again = Round::new(round.lines().to_vec());
        let mut line = Line::default();
        for &(time, action) in round.log() {
            let number = again.results().len();
            let target = again.current().map(|line| key_units(&line.text));
            let target = target.unwrap_or_default();
            again.apply(action, time);

            let finished = again.results().len() > number;
            let reached = if finished {
                target.len()
            } else {
                common_prefix(&key_units(&again.typed().text()), &target)
            };
            let advanced = reached > line.best;
            if let Some(before) = line.last {
                let spent = time - before;
                if advanced {
                    review.clean_seconds += spent;
                    review.clean_keys += 1;
                    if line.flowing {
                        let spent = spent.min(LONGEST_PAUSE);
                        if line.best > 0 && target[line.best - 1] == ' ' {
                            review.word_starts += 1;
                            review.word_start_seconds += spent;
                        } else {
                            review.paces.push((target[line.best], spent));
                        }
                    }
                } else {
                    review.repair_seconds += spent;
                }
            }
            if finished {
                line = Line::default();
            } else {
                // 문장의 시계는 첫 글쇠를 누를 때 켜진다.
                if line.last.is_some() || matches!(action, Action::Press(..)) {
                    line.last = Some(time);
                }
                line.flowing = advanced;
                line.best = line.best.max(reached);
            }
        }
        review
    }

    /// 낱말 안에서 글쇠 하나에 걸린 시간(초).
    fn pace_within_words(&self) -> Option<f64> {
        let seconds: f64 = self.paces.iter().map(|&(_, seconds)| seconds).sum();
        (!self.paces.is_empty()).then(|| seconds / self.paces.len() as f64)
    }

    /// 틀리지 않았다면 나왔을 속도(타/분): 새 글쇠를 맞게 친 시간만으로 낸 값이다.
    pub fn clean_per_minute(&self) -> f64 {
        if self.clean_seconds > 0.0 {
            f64::from(self.clean_keys) * 60.0 / self.clean_seconds
        } else {
            0.0
        }
    }
}

/// 글쇠 하나에 대해 모은 것.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct KeyRecord {
    /// 쳐야 했던 횟수.
    pub needed: u32,
    /// 그 자리에서 다른 글쇠를 누른 횟수.
    pub missed: u32,
    /// 막힘 없이 이어 친 횟수와 그때 걸린 시간의 합(초).
    pub flowed: u32,
    pub seconds: f64,
}

impl KeyRecord {
    /// 막힘 없이 칠 때 이 글쇠에 걸리는 시간(초).
    fn pace(&self) -> f64 {
        self.seconds / f64::from(self.flowed.max(1))
    }
}

/// 이번 실행에서 끝낸 과정들을 글쇠별로 모은 것.
/// 과정 하나에는 글쇠마다 몇 번씩밖에 나오지 않아서, 여러 과정을 모아야 버릇이 보인다.
#[derive(Clone, Debug, Default)]
pub struct Ledger {
    keys: BTreeMap<char, KeyRecord>,
}

impl Ledger {
    /// 친 과정 하나를 더한다. 시간이 다 되어 문장을 치다 말았으면 친 데까지만 센다.
    pub fn add(&mut self, round: &Round, review: &Review) {
        let finished = &round.lines()[..round.results().len()];
        let typed = finished.iter().flat_map(|line| key_units(&line.text));
        for unit in typed.chain(round.reached().iter().copied()) {
            self.keys.entry(unit).or_default().needed += 1;
        }
        for (key, count) in round.misses() {
            self.keys.entry(key).or_default().missed += count;
        }
        for &(key, seconds) in &review.paces {
            let record = self.keys.entry(key).or_default();
            record.flowed += 1;
            record.seconds += seconds;
        }
    }

    /// 지금까지 모은 이 글쇠의 기록.
    pub fn key(&self, key: char) -> KeyRecord {
        self.keys.get(&key).copied().unwrap_or_default()
    }

    /// 막힘 없이 칠 때 `key`가 아닌 다른 글쇠 하나에 걸리는 시간(초).
    fn pace_of_others(&self, key: char) -> Option<f64> {
        let others = || self.keys.iter().filter(move |&(&other, _)| other != key);
        let flowed: u32 = others().map(|(_, record)| record.flowed).sum();
        let seconds: f64 = others().map(|(_, record)| record.seconds).sum();
        (flowed > 0).then(|| seconds / f64::from(flowed))
    }
}

/// 다음 과정에서 바꿔 볼 한 가지.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Advice {
    /// 틀린 글쇠가 많다. 손을 늦춰 틀리지 않게 친다.
    SlowDown,
    /// 한 글쇠를 거듭 틀린다: 쳐야 했던 `needed`번 가운데 `missed`번.
    Missed { key: char, missed: u32, needed: u32 },
    /// 낱말 사이에서 멈춘다: 낱말 안에서는 글쇠마다 `usual`초인데 낱말의 첫 글쇠는 `pause`초 걸린다.
    ReadAhead { pause: f64, usual: f64 },
    /// 한 글쇠 앞에서 머뭇거린다: 다른 글쇠는 `usual`초인데 이 글쇠는 `seconds`초 걸린다.
    Hesitant { key: char, seconds: f64, usual: f64 },
    /// 거의 틀리지 않았다. 이 속도(타/분)를 노려 본다. 외우기 쉽게 열 단위로 올려 잡은 값이다.
    SpeedUp { pace: f64 },
    /// 지금 속도를 지키면서 틀린 글쇠를 줄인다.
    Steady,
}

/// 방금 친 `round`에 대해 권할 한 가지를 고른다. `total`은 그 과정 전체의 기록이고,
/// `ledger`에는 이 과정까지 더해 둔다.
///
/// 틀린 글쇠가 많으면 다른 것은 접어 두고 속도부터 늦춘다. 그렇지 않으면 이번에도 틀린 글쇠 가운데
/// 버릇이 된 것, 낱말 사이에서 멈추는 버릇, 머뭇거리는 글쇠를 차례로 짚는다.
/// 짚을 것이 없고 정확하면 속도를 올린다.
pub fn advise(total: &Tally, round: &Round, review: &Review, ledger: &Ledger) -> Advice {
    if total.accuracy() < CARELESS {
        return Advice::SlowDown;
    }

    let rate = |record: KeyRecord| f64::from(record.missed) / f64::from(record.needed.max(1));
    let repeated = round
        .misses()
        .into_iter()
        .map(|(key, _)| (key, ledger.key(key)))
        .filter(|&(_, record)| record.missed >= REPEATED && rate(record) >= OFTEN)
        .max_by(|a, b| rate(a.1).total_cmp(&rate(b.1)));
    if let Some((key, record)) = repeated {
        let (missed, needed) = (record.missed, record.needed);
        return Advice::Missed {
            key,
            missed,
            needed,
        };
    }

    if review.word_starts >= WORDS_ENOUGH
        && let Some(usual) = review.pace_within_words()
    {
        let pause = review.word_start_seconds / f64::from(review.word_starts);
        if pause >= usual * WORD_GAP {
            return Advice::ReadAhead { pause, usual };
        }
    }

    // 이번 과정에서 친 글쇠 가운데, 다른 글쇠들보다 유난히 오래 걸리는 것.
    let mut typed: Vec<char> = review.paces.iter().map(|&(key, _)| key).collect();
    typed.sort_unstable();
    typed.dedup();
    let slowest = typed
        .into_iter()
        .filter_map(|key| {
            let record = ledger.key(key);
            let usual = ledger.pace_of_others(key)?;
            let hesitant = record.flowed >= ENOUGH && record.pace() >= usual * HESITATION;
            hesitant.then_some((key, record.pace(), usual))
        })
        .max_by(|a, b| (a.1 / a.2).total_cmp(&(b.1 / b.2)));
    if let Some((key, seconds, usual)) = slowest {
        return Advice::Hesitant {
            key,
            seconds,
            usual,
        };
    }

    if total.accuracy() >= PRECISE {
        // 셈의 끝자리 오차 때문에 딱 떨어지는 값이 한 단계 올라가지 않게 한다.
        let pace = ((total.per_minute() * PUSH - 1e-6) / 10.0).ceil() * 10.0;
        Advice::SpeedUp { pace }
    } else {
        Advice::Steady
    }
}

/// 글쇠 이름. 빈칸은 보이지 않으므로 이름으로 적는다.
fn key_name(key: char) -> String {
    if key == ' ' {
        "사이띄개".to_owned()
    } else {
        key.to_string()
    }
}

/// 표준 운지법에서 이 글쇠를 치는 자리를 알려 주는 말.
/// 예: "자리는 왼손 가운뎃손가락, 아랫줄입니다."
pub fn place(key: char) -> Option<String> {
    let (pos, shift) = keystroke_for(key)?;
    let place = match finger(pos) {
        (Some(hand), finger) => format!("{} {}, {}", hand.name(), finger.name(), row(pos).name()),
        (None, _) => Finger::Thumb.name().to_owned(),
    };
    Some(if shift {
        format!("자리는 {place}이고, Shift와 함께 누릅니다.")
    } else {
        format!("자리는 {place}입니다.")
    })
}

/// 근거 뒤에 붙이는 자리 안내. 칠 수 없는 문자면 아무것도 붙이지 않는다.
fn place_note(key: char) -> String {
    place(key).map_or_else(String::new, |place| format!(" {place}"))
}

impl Advice {
    /// 화면에 적을 말: 할 일 한 줄과 그 근거 한 줄.
    ///
    /// `total`은 과정 전체의 기록, `review`는 그 과정을 되짚어 본 것이다.
    pub fn words(&self, total: &Tally, review: &Review) -> (String, String) {
        // 지금 속도는 바로 위에 큰 숫자로 보이므로, 틀리지 않았을 때의 속도만 적는다.
        let cost = format!(
            "고치는 데 {:.1}초를 썼습니다. 틀리지 않았다면 {:.0}타/분쯤입니다.",
            review.repair_seconds,
            review.clean_per_minute()
        );
        match *self {
            Self::SlowDown => ("손을 조금 늦춰 틀리지 않고 쳐 보세요.".to_owned(), cost),
            Self::Missed {
                key,
                missed,
                needed,
            } => (
                format!("{} 글쇠 앞에서 한 박자 늦춰 보세요.", key_name(key)),
                format!("오늘 쳐야 했던 {needed}번 가운데 {missed}번 틀렸습니다.")
                    + &place_note(key),
            ),
            Self::ReadAhead { pause, usual } => (
                "낱말을 치는 동안 눈은 다음 낱말을 미리 읽어 보세요.".to_owned(),
                format!(
                    "낱말 사이에서 {pause:.2}초씩 멈춥니다. 낱말 안에서는 글쇠마다 {usual:.2}초입니다."
                ),
            ),
            Self::Hesitant {
                key,
                seconds,
                usual,
            } => (
                format!("{} 글쇠의 자리를 손에 익혀 보세요.", key_name(key)),
                format!("다른 글쇠는 {usual:.2}초인데 이 글쇠는 {seconds:.2}초 걸립니다.")
                    + &place_note(key),
            ),
            Self::SpeedUp { pace } => (
                format!("{pace:.0}타/분을 노려 보세요."),
                format!(
                    "정확도가 {:.1}%입니다. 조금 더 빨리 쳐도 됩니다.",
                    total.accuracy() * 100.0
                ),
            ),
            Self::Steady => (
                "지금 속도를 지키면서 틀린 글쇠를 하나만 줄여 보세요.".to_owned(),
                cost,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::texts::{Kind, Sentence};

    fn round(lines: &[(Kind, &'static str)]) -> Round {
        let lines = lines.iter().map(|&(kind, text)| Sentence::new(kind, text));
        Round::new(lines.collect())
    }

    /// 적어 준 대로 친다. 글쇠 사이는 `gap`초다.
    /// `<`는 지우기, `!`는 틀린 데까지 되돌리기, `_`는 `gap`만큼 더 쉬기이고, 나머지는 그 낱자나 문자의 글쇠다.
    fn play(round: &mut Round, script: &str, start: f64, gap: f64) -> f64 {
        let mut now = start;
        for step in script.chars() {
            now += gap;
            match step {
                '<' => round.backspace(now),
                '!' => round.rewind(now),
                '_' => {}
                unit => {
                    let (pos, shift) = keystroke_for(unit).expect("글쇠 하나로 치는 문자");
                    round.press(pos, shift, now);
                }
            }
        }
        now
    }

    fn total(round: &Round) -> Tally {
        round.results().iter().copied().sum()
    }

    /// 한 번도 틀리지 않았으면 고친 시간이 없고, 틀리지 않았을 때의 속도가 곧 실제 속도다.
    #[test]
    fn a_clean_round_has_nothing_to_repair() {
        let mut round = round(&[(Kind::Hangul, "가나"), (Kind::English, "ok")]);
        play(&mut round, "ㄱㅏㄴㅏ", 0.0, 0.5);
        play(&mut round, "ok", 10.0, 0.5);
        let review = Review::of(&round);
        assert_eq!(review.repair_seconds, 0.0);
        assert_eq!(review.clean_keys, total(&round).timed);
        assert_eq!(review.clean_seconds, total(&round).seconds);
        assert_eq!(review.clean_per_minute(), total(&round).per_minute());
        let keys: String = review.paces.iter().map(|&(key, _)| key).collect();
        assert_eq!(keys, "ㅏㄴㅏk", "문장의 첫 글쇠는 걸린 시간이 없다");
    }

    /// 틀린 글쇠를 누른 시간과 지운 시간은 고친 시간이다.
    #[test]
    fn a_wrong_key_and_its_erasing_are_repair_time() {
        let mut round = round(&[(Kind::English, "cat")]);
        // c, o(틀림), 지우기, a, t를 1초에 하나씩.
        play(&mut round, "co<at", 0.0, 1.0);
        let review = Review::of(&round);
        assert_eq!((review.repair_seconds, review.clean_seconds), (2.0, 2.0));
        assert_eq!(review.clean_keys, 2);
        // 실제로는 4초에 두 글쇠, 틀리지 않았다면 2초에 두 글쇠다.
        assert_eq!(total(&round).per_minute(), 30.0);
        assert_eq!(review.clean_per_minute(), 60.0);
        // 지운 바로 뒤에 친 a는 평소 빠르기로 치지 않는다.
        assert_eq!(review.paces, [('t', 1.0)]);
    }

    /// 지운 자리를 다시 치는 것도 고친 시간이다. 새로 나아간 글쇠만 맞게 친 시간이다.
    #[test]
    fn retyping_what_was_erased_is_repair_time() {
        let mut round = round(&[(Kind::English, "abcd")]);
        // abc까지 치고 모두 지운 뒤 처음부터 다시 친다.
        play(&mut round, "abc<<<abcd", 0.0, 1.0);
        let review = Review::of(&round);
        // 지우기 세 번과 다시 친 a, b, c가 고친 시간이다.
        assert_eq!((review.repair_seconds, review.clean_seconds), (6.0, 3.0));
        assert_eq!(review.clean_keys, 3);
    }

    /// 틀린 데까지 한 번에 되돌리는 것도 고친 시간이다.
    #[test]
    fn rewinding_to_the_slip_is_repair_time() {
        let mut round = round(&[(Kind::English, "abcd")]);
        // ab까지 맞게 치고, x를 잘못 누르고, 되돌린 뒤 이어서 친다.
        play(&mut round, "abx!cd", 0.0, 1.0);
        assert!(round.is_finished());
        let review = Review::of(&round);
        assert_eq!((review.repair_seconds, review.clean_seconds), (2.0, 3.0));
        assert_eq!(review.clean_keys, 3);
    }

    /// 문장의 첫 글쇠를 틀려도 셈이 맞는다.
    #[test]
    fn a_wrong_first_key_starts_the_clock() {
        let mut round = round(&[(Kind::English, "ab")]);
        play(&mut round, "x<ab", 0.0, 1.0);
        let review = Review::of(&round);
        assert_eq!(total(&round).seconds, 3.0);
        assert_eq!((review.repair_seconds, review.clean_seconds), (1.0, 2.0));
        assert_eq!(review.clean_keys, 2);
    }

    /// 어떻게 치든, 고친 시간과 맞게 친 시간을 더하면 걸린 시간이 된다.
    #[test]
    fn repair_and_clean_time_add_up_to_the_time_taken() {
        let lines = [
            (Kind::Hangul, "값싼 떡"),
            (Kind::English, "fox"),
            (Kind::Rust, "a + b"),
        ];
        let mut round = round(&lines);
        // 지우기부터 누르고, 받침을 틀리고, 틀린 채로 더 치고, 틀린 데까지 되돌리고, 쉬었다가 친다.
        play(&mut round, "<ㄱㅏㅂㅅㅆㅓ<ㅏㄴ ㄸㅓㄱ", 0.0, 0.3);
        play(&mut round, "fpxxxx<<ox", 20.0, 0.2);
        play(&mut round, "a xy!__+ b", 40.0, 0.4);
        assert!(round.is_finished());
        let review = Review::of(&round);
        let taken = total(&round).seconds;
        assert!((review.repair_seconds + review.clean_seconds - taken).abs() < 1e-9);
        assert!(review.repair_seconds > 0.0 && review.clean_seconds > 0.0);
    }

    /// 딴 일을 하느라 오래 멈춘 것은 그 글쇠가 느린 것으로 치지 않는다.
    #[test]
    fn a_long_pause_does_not_make_a_key_look_slow() {
        let mut round = round(&[(Kind::English, "ab")]);
        play(&mut round, "a", 0.0, 0.1);
        play(&mut round, "b", 0.0, 60.0);
        assert_eq!(Review::of(&round).paces, [('b', LONGEST_PAUSE)]);
    }

    /// 과정을 끝내고 되짚어 조언을 고른다. 과정마다 장부에 더한다.
    fn finish(round: &Round, ledger: &mut Ledger) -> (Review, Advice) {
        assert!(round.is_finished());
        let review = Review::of(round);
        ledger.add(round, &review);
        let advice = advise(&total(round), round, &review, ledger);
        (review, advice)
    }

    /// 시간이 다 되어 문장을 치다 만 과정도 되짚을 수 있다. 치지 않은 문장은 셈에 넣지 않는다.
    #[test]
    fn an_unfinished_round_is_reviewed_up_to_where_it_stopped() {
        let lines = [
            (Kind::English, "ab"),
            (Kind::English, "cde"),
            (Kind::English, "zz"),
        ];
        let mut round = round(&lines);
        play(&mut round, "ab", 0.0, 0.5);
        // 둘째 문장은 c, d까지만 치고 멈춘다. 셋째 문장은 손도 대지 않는다.
        play(&mut round, "cx<d", 10.0, 0.5);
        let review = Review::of(&round);
        let so_far = round.so_far(12.0);
        assert_eq!((review.clean_keys, review.repair_seconds), (2, 1.0));

        let mut ledger = Ledger::default();
        ledger.add(&round, &review);
        let needed = |key| ledger.key(key).needed;
        assert_eq!((needed('a'), needed('c'), needed('d')), (1, 1, 1));
        assert_eq!((needed('e'), needed('z')), (0, 0), "아직 치지 않은 글쇠");
        // 치다 만 문장에서 누른 것까지 넣은 기록으로 조언을 고른다.
        assert_eq!((so_far.presses, so_far.wrong), (5, 1));
        assert_eq!(advise(&so_far, &round, &review, &ledger), Advice::SlowDown);
    }

    #[test]
    fn many_mistakes_call_for_slowing_down() {
        let mut round = round(&[(Kind::English, "the cat sat on the mat")]);
        // 스물두 글쇠를 치며 세 번 틀리고 고친다. 정확도는 22/25 = 88%.
        play(&mut round, "the cs<at sar<t on thr<e mat", 0.0, 0.2);
        let (review, advice) = finish(&round, &mut Ledger::default());
        assert!(total(&round).accuracy() < CARELESS);
        assert_eq!(advice, Advice::SlowDown);
        // 실제 속도는 233타/분이지만, 틀리지 않고 칠 때는 0.2초에 한 글쇠, 곧 300타/분이다.
        assert!((total(&round).per_minute() - 233.3).abs() < 0.1);
        assert!((review.clean_per_minute() - 300.0).abs() < 1e-6);

        let (advice, reason) = advice.words(&total(&round), &review);
        assert_eq!(advice, "손을 조금 늦춰 틀리지 않고 쳐 보세요.");
        assert_eq!(
            reason,
            "고치는 데 1.2초를 썼습니다. 틀리지 않았다면 300타/분쯤입니다."
        );
    }

    /// 같은 글쇠를 거듭 틀리면 그 글쇠와 그 자리를 짚는다.
    #[test]
    fn a_key_missed_again_and_again_is_pointed_out() {
        let line = "차를 마시고 책을 읽고 춤을 추는 참새와 까치가 찾아온 아침";
        let mut round = round(&[(Kind::Hangul, line)]);
        // 첫 ㅊ과 둘째 ㅊ을 ㅈ으로 잘못 쳤다가 지우고 다시 친다.
        let units = key_units(line);
        let mut seen = 0;
        let mut steps = String::new();
        for &unit in &units {
            if unit == 'ㅊ' {
                seen += 1;
                if seen <= 2 {
                    steps += "ㅈ<";
                }
            }
            steps.push(unit);
        }
        play(&mut round, &steps, 0.0, 0.2);
        let (review, advice) = finish(&round, &mut Ledger::default());
        assert!(total(&round).accuracy() >= CARELESS);
        let needed = units.iter().filter(|&&unit| unit == 'ㅊ').count() as u32;
        assert_eq!(
            advice,
            Advice::Missed {
                key: 'ㅊ',
                missed: 2,
                needed
            }
        );
        let (advice, reason) = advice.words(&total(&round), &review);
        assert_eq!(advice, "ㅊ 글쇠 앞에서 한 박자 늦춰 보세요.");
        assert_eq!(
            reason,
            format!(
                "오늘 쳐야 했던 {needed}번 가운데 2번 틀렸습니다. \
                 자리는 왼손 가운뎃손가락, 아랫줄입니다."
            )
        );
    }

    /// 예전에 틀렸더라도 이번 과정에서 틀리지 않은 글쇠는 다시 짚지 않는다.
    #[test]
    fn a_key_typed_right_this_time_is_left_alone() {
        let mut ledger = Ledger::default();
        let text = "zip zap zoo and a lazy fox naps in the sun all day long";
        let mut first = round(&[(Kind::English, text)]);
        play(&mut first, &format!("x<zip x<{}", &text[4..]), 0.0, 0.2);
        let (_, advice) = finish(&first, &mut ledger);
        assert!(
            matches!(advice, Advice::Missed { key: 'z', .. }),
            "{advice:?}"
        );

        let mut second = round(&[(Kind::English, text)]);
        play(&mut second, text, 100.0, 0.2);
        let (_, advice) = finish(&second, &mut ledger);
        assert!(matches!(advice, Advice::SpeedUp { .. }), "{advice:?}");
    }

    /// 틀리지는 않지만 유난히 오래 걸리는 글쇠가 있으면 그 글쇠를 짚는다.
    #[test]
    fn a_key_that_takes_much_longer_is_pointed_out() {
        let text = "a{b{c{d{e{f";
        let mut round = round(&[(Kind::Rust, text)]);
        let mut now = 0.0;
        for unit in text.chars() {
            // 중괄호를 누르기까지만 0.6초, 나머지는 0.2초 걸린다.
            let gap = if unit == '{' { 0.6 } else { 0.2 };
            now = play(&mut round, &unit.to_string(), now, gap);
        }
        let (review, advice) = finish(&round, &mut Ledger::default());
        let Advice::Hesitant {
            key,
            seconds,
            usual,
        } = advice
        else {
            panic!("{advice:?}");
        };
        assert_eq!(key, '{');
        assert!((seconds - 0.6).abs() < 1e-9 && (usual - 0.2).abs() < 1e-9);
        let (advice, reason) = advice.words(&total(&round), &review);
        assert_eq!(advice, "{ 글쇠의 자리를 손에 익혀 보세요.");
        assert_eq!(
            reason,
            "다른 글쇠는 0.20초인데 이 글쇠는 0.60초 걸립니다. \
             자리는 오른손 새끼손가락, 윗줄이고, Shift와 함께 누릅니다."
        );
    }

    /// 낱말은 빨리 치는데 낱말과 낱말 사이에서 멈추면, 다음 낱말을 미리 읽으라고 권한다.
    #[test]
    fn stopping_between_words_calls_for_reading_ahead() {
        let text = "look before you leap and think before you speak to a friend";
        let mut round = round(&[(Kind::English, text)]);
        let (mut now, mut after_space) = (0.0, false);
        for unit in text.chars() {
            // 낱말의 첫 글쇠를 누르기까지만 0.5초, 나머지는 0.2초 걸린다.
            let gap = if after_space { 0.5 } else { 0.2 };
            now = play(&mut round, &unit.to_string(), now, gap);
            after_space = unit == ' ';
        }
        let (review, advice) = finish(&round, &mut Ledger::default());
        assert_eq!(review.word_starts, 11);
        let Advice::ReadAhead { pause, usual } = advice else {
            panic!("{advice:?}");
        };
        assert!((pause - 0.5).abs() < 1e-9 && (usual - 0.2).abs() < 1e-9);
        let (advice, reason) = advice.words(&total(&round), &review);
        assert_eq!(
            advice,
            "낱말을 치는 동안 눈은 다음 낱말을 미리 읽어 보세요."
        );
        assert_eq!(
            reason,
            "낱말 사이에서 0.50초씩 멈춥니다. 낱말 안에서는 글쇠마다 0.20초입니다."
        );
    }

    #[test]
    fn a_clean_even_round_calls_for_more_speed() {
        let mut round = round(&[(Kind::English, "slow and steady wins the race")]);
        play(&mut round, "slow and steady wins the race", 0.0, 0.2);
        let (review, advice) = finish(&round, &mut Ledger::default());
        // 300타/분에서 5%를 올린 315를, 외우기 쉽게 열 단위로 올려 320으로 권한다.
        assert_eq!(advice, Advice::SpeedUp { pace: 320.0 });
        let (advice, reason) = advice.words(&total(&round), &review);
        assert_eq!(advice, "320타/분을 노려 보세요.");
        assert_eq!(reason, "정확도가 100.0%입니다. 조금 더 빨리 쳐도 됩니다.");
    }

    /// 조금 틀렸고 딱히 짚을 글쇠가 없으면, 속도는 그대로 두고 틀린 글쇠를 줄이라고 권한다.
    #[test]
    fn a_few_scattered_mistakes_call_for_holding_the_pace() {
        let text = "pack my box with five dozen liquor jugs";
        let mut round = round(&[(Kind::English, text)]);
        play(
            &mut round,
            "pack my bi<ox with five dozen liquor jugs",
            0.0,
            0.2,
        );
        let (review, advice) = finish(&round, &mut Ledger::default());
        let accuracy = total(&round).accuracy();
        assert!((CARELESS..PRECISE).contains(&accuracy), "{accuracy}");
        assert_eq!(advice, Advice::Steady);
        let (advice, reason) = advice.words(&total(&round), &review);
        assert_eq!(
            advice,
            "지금 속도를 지키면서 틀린 글쇠를 하나만 줄여 보세요."
        );
        assert_eq!(
            reason,
            "고치는 데 0.4초를 썼습니다. 틀리지 않았다면 300타/분쯤입니다."
        );
    }

    #[test]
    fn places_name_the_finger_and_the_row() {
        let place = |key| place(key).unwrap_or_default();
        assert_eq!(place('ㅊ'), "자리는 왼손 가운뎃손가락, 아랫줄입니다.");
        assert_eq!(place('j'), "자리는 오른손 집게손가락, 가운데 줄입니다.");
        assert_eq!(
            place('ㅆ'),
            "자리는 왼손 집게손가락, 윗줄이고, Shift와 함께 누릅니다."
        );
        assert_eq!(place('1'), "자리는 왼손 새끼손가락, 숫자 줄입니다.");
        assert_eq!(place(' '), "자리는 엄지손가락입니다.");
        assert_eq!(place('가'), "", "글쇠 하나로 칠 수 없는 글자");
    }
}
