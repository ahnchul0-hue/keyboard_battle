//! 과정 하나: 문장 몇 개를 차례로 치고 기록을 남긴다.
//!
//! 문장은 끝까지 정확히 쳐야 다음으로 넘어간다. 틀린 글쇠는 지우고 다시 쳐야 하며,
//! 그렇게 누른 글쇠는 모두 정확도에 반영된다. 지우개(Backspace)는 세지 않는다.

use std::collections::{BTreeMap, BTreeSet};

use crate::keymap::KeyPos;
use crate::measure::{Tally, char_of_unit, common_prefix, key_units};
use crate::texts::Sentence;
use crate::typed::TypedText;

#[derive(Clone, Debug)]
pub struct Round {
    lines: Vec<Sentence>,
    /// 끝낸 문장들의 기록. 길이가 곧 지금 칠 문장의 번호다.
    done: Vec<Tally>,
    /// 과정 전체를 치는 데 필요한 글쇠 수.
    needed: u32,
    /// 지금 문장의 글쇠 순서.
    target: Vec<char>,
    /// 지금 문장에 친 글.
    typed: TypedText,
    presses: u32,
    wrong: u32,
    /// 지금 문장의 첫 글쇠를 누른 시각(초).
    started_at: Option<f64>,
    /// 틀렸을 때 눌러야 했던 글쇠와 그 횟수.
    misses: BTreeMap<char, u32>,
    /// 문장마다, 틀리기 시작한 자리(글자 번호)들.
    slips: Vec<BTreeSet<usize>>,
}

impl Round {
    pub fn new(lines: Vec<Sentence>) -> Self {
        let units = |line: &Sentence| key_units(line.text);
        Self {
            needed: lines.iter().map(|line| units(line).len() as u32).sum(),
            target: lines.first().map(units).unwrap_or_default(),
            done: Vec::new(),
            typed: TypedText::default(),
            presses: 0,
            wrong: 0,
            started_at: None,
            misses: BTreeMap::new(),
            slips: vec![BTreeSet::new(); lines.len()],
            lines,
        }
    }

    pub fn lines(&self) -> &[Sentence] {
        &self.lines
    }

    /// 끝낸 문장들의 기록. 문장 순서와 같다.
    pub fn results(&self) -> &[Tally] {
        &self.done
    }

    /// 지금 칠 문장. 과정이 끝났으면 `None`.
    pub fn current(&self) -> Option<&Sentence> {
        self.lines.get(self.done.len())
    }

    pub fn is_finished(&self) -> bool {
        self.current().is_none()
    }

    /// 지금 문장에 친 글.
    pub fn typed(&self) -> &TypedText {
        &self.typed
    }

    /// 지금 문장에서 문장과 어긋나지 않게 친 글쇠 수.
    fn matched(&self) -> usize {
        common_prefix(&key_units(&self.typed.text()), &self.target)
    }

    /// 문자 글쇠 하나를 누른다. `now`는 누른 시각(초)이다.
    pub fn press(&mut self, pos: KeyPos, shift: bool, now: f64) {
        let Some(line) = self.current().copied() else {
            return;
        };
        let position = self.matched();
        let expected = self.target.get(position).copied();
        let was_on_track = position == key_units(&self.typed.text()).len();

        self.started_at.get_or_insert(now);
        self.presses += 1;
        let before = self.typed.clone();
        self.typed.press(pos, shift, line.kind.script());

        if self.matched() < key_units(&self.typed.text()).len() {
            self.wrong += 1;
            // 잘 치다가 처음 어긋난 순간에만, 눌러야 했던 글쇠와 그 자리를 적어 둔다.
            if let Some(expected) = expected.filter(|_| was_on_track) {
                *self.misses.entry(expected).or_default() += 1;
                let line_number = self.done.len();
                self.slips[line_number].extend(char_of_unit(line.text, position));
            }
            // 틀린 줄 모르고 계속 쳐도 글이 문장보다 길어지지는 않게 한다.
            // 누른 것은 세되 글에는 넣지 않는다.
            if self.typed.text().chars().count() > line.text.chars().count() {
                self.typed = before;
            }
        }
        if self.typed.text() == line.text {
            self.finish_line(line, now);
        }
    }

    fn finish_line(&mut self, line: Sentence, now: f64) {
        let needed = self.target.len() as u32;
        self.done.push(Tally {
            needed,
            chars: line.text.chars().count() as u32,
            presses: self.presses,
            wrong: self.wrong,
            timed: needed.saturating_sub(1),
            seconds: now - self.started_at.take().unwrap_or(now),
        });
        self.typed.clear();
        self.presses = 0;
        self.wrong = 0;
        self.target = self
            .current()
            .map(|next| key_units(next.text))
            .unwrap_or_default();
    }

    /// 조합 중이면 낱자 하나를, 아니면 글자 하나를 지운다.
    pub fn backspace(&mut self) {
        self.typed.backspace();
    }

    /// 지금 문장에 친 글을 모두 지운다. 누른 글쇠 수와 시간은 그대로 남는다.
    pub fn clear_line(&mut self) {
        self.typed.clear();
    }

    /// 지금 문장의 시간을 재고 있는지. 문장의 첫 글쇠를 누른 때부터 그 문장을 끝낼 때까지다.
    pub fn is_timing(&self) -> bool {
        self.started_at.is_some()
    }

    /// 지금까지의 기록. 치고 있는 문장은 어긋나지 않게 친 데까지만 넣는다.
    pub fn so_far(&self, now: f64) -> Tally {
        let finished: Tally = self.done.iter().copied().sum();
        let Some(started_at) = self.started_at else {
            return finished;
        };
        let matched = self.matched() as u32;
        finished
            + Tally {
                needed: matched,
                chars: 0,
                presses: self.presses,
                wrong: self.wrong,
                timed: matched.saturating_sub(1),
                seconds: now - started_at,
            }
    }

    /// 과정 전체에서 얼마나 쳤는지 (0 ~ 1).
    pub fn progress(&self) -> f32 {
        if self.needed == 0 {
            return 1.0;
        }
        let finished: u32 = self.done.iter().map(|tally| tally.needed).sum();
        (finished + self.matched() as u32) as f32 / self.needed as f32
    }

    /// 문장마다, 틀리기 시작한 자리(글자 번호)들. 문장 순서와 같다.
    pub fn slips(&self) -> &[BTreeSet<usize>] {
        &self.slips
    }

    /// 틀렸을 때 눌러야 했던 글쇠들. 많이 틀린 것부터.
    pub fn misses(&self) -> Vec<(char, u32)> {
        let mut misses: Vec<(char, u32)> = self.misses.iter().map(|(&k, &n)| (k, n)).collect();
        misses.sort_by_key(|&(_, count)| std::cmp::Reverse(count));
        misses
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap::keystroke_for;
    use crate::texts::Kind;

    fn round(lines: &[(Kind, &'static str)]) -> Round {
        let lines = lines.iter().map(|&(kind, text)| Sentence { kind, text });
        Round::new(lines.collect())
    }

    /// `keys`(낱자나 문자)를 `start`초부터 1초에 하나씩 누른다.
    fn press_all(round: &mut Round, keys: &str, start: f64) {
        for (index, unit) in keys.chars().enumerate() {
            let (pos, shift) = keystroke_for(unit).expect("글쇠 하나로 치는 문자");
            round.press(pos, shift, start + index as f64);
        }
    }

    #[test]
    fn a_clean_line_is_recorded_and_the_next_one_begins() {
        let mut round = round(&[(Kind::Hangul, "가나"), (Kind::English, "ok")]);
        press_all(&mut round, "ㄱㅏㄴㅏ", 10.0);
        let expected = Tally {
            needed: 4,
            chars: 2,
            presses: 4,
            wrong: 0,
            timed: 3,
            seconds: 3.0,
        };
        assert_eq!(round.results(), [expected]);
        // 1초에 한 글쇠씩 쳤다.
        assert_eq!(expected.per_minute(), 60.0);
        assert_eq!(round.current().map(|line| line.text), Some("ok"));
        assert_eq!(round.typed().text(), "");
        assert!(!round.is_finished());

        press_all(&mut round, "ok", 20.0);
        assert!(round.is_finished());
        assert_eq!(round.results().len(), 2);
        assert_eq!(round.progress(), 1.0);
    }

    /// 받침이 다음 글자로 넘어가기 전의 모습('간')은 틀린 것이 아니다.
    #[test]
    fn a_final_waiting_to_move_on_is_not_a_mistake() {
        let mut round = round(&[(Kind::Hangul, "가나")]);
        press_all(&mut round, "ㄱㅏㄴ", 0.0);
        assert_eq!(round.typed().text(), "간");
        assert_eq!((round.presses, round.wrong), (3, 0));
        assert_eq!(round.progress(), 0.75);
    }

    #[test]
    fn a_mistake_counts_against_accuracy_even_after_it_is_fixed() {
        let mut round = round(&[(Kind::Hangul, "가나")]);
        press_all(&mut round, "ㄱㅓ", 0.0);
        assert_eq!(round.progress(), 0.25, "틀린 글쇠는 진행에 넣지 않는다");
        round.backspace();
        press_all(&mut round, "ㅏㄴㅏ", 2.0);

        let line = round.results()[0];
        assert_eq!((line.presses, line.wrong), (5, 1));
        assert_eq!(line.accuracy(), 0.8);
        assert_eq!(round.misses(), [('ㅏ', 1)]);
    }

    /// 틀린 채로 계속 치면 누를 때마다 틀린 것으로 세지만, 놓친 글쇠는 처음 한 번만 적는다.
    #[test]
    fn typing_on_after_a_mistake_keeps_counting_wrong_presses() {
        let mut round = round(&[(Kind::English, "cat")]);
        press_all(&mut round, "cot", 0.0);
        assert_eq!((round.presses, round.wrong), (3, 2));
        assert_eq!(round.misses(), [('a', 1)]);
    }

    /// 틀린 줄 모르고 다음 문장까지 쳐 버려도, 친 글은 문장 길이에서 멈춘다.
    #[test]
    fn typing_past_the_end_of_a_wrong_line_adds_nothing() {
        let mut round = round(&[(Kind::English, "cat"), (Kind::English, "dog")]);
        press_all(&mut round, "cotdog", 0.0);
        assert_eq!(round.typed().text(), "cot");
        assert_eq!((round.presses, round.wrong), (6, 5));
        assert!(round.results().is_empty());

        // 지우고 고치면 그대로 이어서 칠 수 있다.
        round.backspace();
        round.backspace();
        press_all(&mut round, "at", 6.0);
        assert_eq!(round.results().len(), 1);
    }

    /// 한글은 조합 중인 글자가 바뀌는 동안에는 길이가 늘지 않으므로 그대로 받는다.
    #[test]
    fn the_length_limit_counts_characters_not_keys() {
        let mut round = round(&[(Kind::Hangul, "가나")]);
        press_all(&mut round, "ㄱㅓㄴㅏㅁ", 0.0);
        assert_eq!(round.typed().text(), "거남");
        press_all(&mut round, "ㅏ", 5.0);
        assert_eq!(round.typed().text(), "거남", "'거나마'는 문장보다 길다");
    }

    /// 틀린 자리는 글쇠가 아니라 글자 번호로 남는다. 같은 자리에서 여러 번 틀려도 한 번만 적는다.
    #[test]
    fn slips_remember_which_characters_went_wrong() {
        let mut round = round(&[(Kind::Hangul, "가나다"), (Kind::English, "cat")]);
        // '나'의 홀소리를 두 번 틀리고, '다'의 닿소리를 한 번 틀린다.
        press_all(&mut round, "ㄱㅏㄴㅓ", 0.0);
        round.backspace();
        press_all(&mut round, "ㅗ", 1.0);
        round.backspace();
        press_all(&mut round, "ㅏㅁ", 2.0);
        round.backspace();
        press_all(&mut round, "ㄷㅏ", 3.0);
        // 둘째 문장은 틀리지 않는다.
        press_all(&mut round, "cat", 4.0);

        let slips: Vec<Vec<usize>> = round
            .slips()
            .iter()
            .map(|line| line.iter().copied().collect())
            .collect();
        assert_eq!(slips, [vec![1, 2], vec![]]);
        assert!(round.is_finished());
    }

    #[test]
    fn clearing_the_line_keeps_what_was_pressed() {
        let mut round = round(&[(Kind::English, "ab")]);
        press_all(&mut round, "x", 0.0);
        round.clear_line();
        press_all(&mut round, "ab", 1.0);
        let line = round.results()[0];
        assert_eq!((line.presses, line.wrong, line.seconds), (3, 1, 2.0));
    }

    #[test]
    fn so_far_includes_the_line_in_progress() {
        let mut round = round(&[(Kind::English, "ab"), (Kind::English, "cd")]);
        assert_eq!(round.so_far(5.0), Tally::default());
        press_all(&mut round, "ab", 0.0);
        press_all(&mut round, "c", 10.0);
        let now = round.so_far(12.0);
        assert_eq!((now.needed, now.presses, now.seconds), (3, 3, 3.0));
        assert_eq!(round.progress(), 0.75);
    }

    #[test]
    fn keys_after_the_round_is_over_are_ignored() {
        let mut round = round(&[(Kind::English, "a")]);
        press_all(&mut round, "ab", 0.0);
        assert!(round.is_finished());
        assert_eq!(round.results().len(), 1);
        assert_eq!(round.so_far(9.0).presses, 1);
    }

    #[test]
    fn misses_are_listed_most_frequent_first() {
        let mut round = round(&[(Kind::English, "abab")]);
        for wrong_then_right in ["xa", "b", "xa", "xb"] {
            for unit in wrong_then_right.chars() {
                if unit == 'x' {
                    press_all(&mut round, "x", 0.0);
                    round.backspace();
                } else {
                    press_all(&mut round, &unit.to_string(), 0.0);
                }
            }
        }
        assert_eq!(round.misses(), [('a', 2), ('b', 1)]);
    }
}
