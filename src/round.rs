//! 과정 하나: 문장을 차례로 치고 기록을 남긴다.
//!
//! 문장 수가 정해진 과정도 있고(둘러보기), 시간이 다 될 때까지 문장을 이어 붙이는 과정도 있다(연습).
//! 문장은 끝까지 정확히 쳐야 다음으로 넘어간다. 틀린 글쇠는 지우고 다시 쳐야 하며,
//! 그렇게 누른 글쇠는 모두 정확도에 반영된다. 지우개(Backspace)는 세지 않는다.

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

use crate::keymap::KeyPos;
use crate::measure::{Tally, char_of_unit, common_prefix, key_units};
use crate::texts::Sentence;
use crate::typed::TypedText;

/// 과정을 치는 동안 일어나는 일 하나.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// 문자 글쇠를 누른다. 뒤의 값은 Shift를 함께 눌렀는지다.
    Press(KeyPos, bool),
    /// 조합 중이면 낱자 하나를, 아니면 글자 하나를 지운다.
    Backspace,
    /// 친 글에서 처음 틀린 데부터 뒤를 지운다. 틀린 데가 없으면 아무 일도 없다.
    /// 누른 글쇠 수와 시간은 그대로 남는다.
    Rewind,
}

#[derive(Clone, Debug)]
pub struct Round {
    lines: Vec<Sentence>,
    /// 쪽(한 블럭에 함께 보이는 글들)이 시작하는 글 번호. 늘 0으로 시작한다.
    pages: Vec<usize>,
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
    /// 일어난 일과 그 시각(초). 그대로 다시 해 보이면 같은 과정이 된다.
    log: Vec<(f64, Action)>,
}

impl Round {
    /// `lines`를 한 쪽으로 삼아 시작한다.
    pub fn new(lines: Vec<Sentence>) -> Self {
        let units = |line: &Sentence| key_units(&line.text);
        Self {
            pages: vec![0],
            needed: lines.iter().map(|line| units(line).len() as u32).sum(),
            target: lines.first().map(units).unwrap_or_default(),
            done: Vec::new(),
            typed: TypedText::default(),
            presses: 0,
            wrong: 0,
            started_at: None,
            misses: BTreeMap::new(),
            slips: vec![BTreeSet::new(); lines.len()],
            log: Vec::new(),
            lines,
        }
    }

    pub fn lines(&self) -> &[Sentence] {
        &self.lines
    }

    /// 칠 글을 새 쪽으로 뒤에 더 붙인다. 시간을 정해 치는 연습에서 글이 떨어지지 않게 하는 데 쓴다.
    pub fn extend(&mut self, lines: Vec<Sentence>) {
        if lines.is_empty() {
            return;
        }
        let was_finished = self.is_finished();
        let keys = |line: &Sentence| key_units(&line.text).len() as u32;
        self.needed += lines.iter().map(keys).sum::<u32>();
        self.slips
            .resize(self.slips.len() + lines.len(), BTreeSet::new());
        if !self.lines.is_empty() {
            self.pages.push(self.lines.len());
        }
        self.lines.extend(lines);
        if was_finished {
            let next = self.current().map(|line| key_units(&line.text));
            self.target = next.unwrap_or_default();
        }
    }

    /// `line`번째 글이 든 쪽의 범위(글 번호). 마지막 글보다 뒤면 마지막 쪽이다.
    pub fn page_of(&self, line: usize) -> Range<usize> {
        let page = self.pages.partition_point(|&start| start <= line) - 1;
        let end = self.pages.get(page + 1).copied();
        self.pages[page]..end.unwrap_or(self.lines.len())
    }

    /// `line`번째 글이 든 쪽의 번호 (0부터).
    pub fn page_number(&self, line: usize) -> usize {
        self.pages.partition_point(|&start| start <= line) - 1
    }

    /// 끝까지 다 친 쪽의 수.
    pub fn pages_done(&self) -> usize {
        let done = self.done.len();
        let ends = self.pages.iter().skip(1).copied();
        let ends = ends.chain(std::iter::once(self.lines.len()));
        ends.filter(|&end| end > 0 && end <= done).count()
    }

    /// 틀린 데가 있는 낱말들과, 그 낱말 안에서 틀리기 시작한 자리(글자 번호)들. 친 순서대로다.
    pub fn slipped_words(&self) -> Vec<(String, Vec<usize>)> {
        let mut words: Vec<(String, Vec<usize>)> = Vec::new();
        for (line, slips) in self.lines.iter().zip(&self.slips) {
            let chars: Vec<char> = line.text.chars().collect();
            let mut last_start = None;
            for &slip in slips {
                // 틀린 글자가 든 낱말의 처음과 끝. 빈칸에서 틀렸으면 그 앞 낱말로 친다.
                let at = (0..=slip.min(chars.len().saturating_sub(1)))
                    .rev()
                    .find(|&index| chars[index] != ' ')
                    .unwrap_or(0);
                let is_space = |&index: &usize| chars[index] == ' ';
                let start = (0..at).rev().find(is_space).map_or(0, |space| space + 1);
                let end = (at..chars.len()).find(is_space).unwrap_or(chars.len());
                if last_start == Some(start) {
                    // 같은 낱말에서 또 틀렸다.
                    if let Some((_, slipped)) = words.last_mut() {
                        slipped.push(slip.min(end) - start);
                    }
                } else {
                    let word: String = chars[start..end].iter().collect();
                    words.push((word, vec![slip.min(end) - start]));
                    last_start = Some(start);
                }
            }
        }
        words
    }

    /// 같은 글을 같은 쪽 나눔으로 처음부터 다시 치는 과정.
    pub fn fresh(&self) -> Self {
        let mut fresh = Self::new(self.lines.clone());
        fresh.pages.clone_from(&self.pages);
        fresh
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

    /// 지금 문장에서 어긋나지 않게 친 데까지의 글쇠들.
    pub fn reached(&self) -> &[char] {
        &self.target[..self.matched()]
    }

    /// 일 하나를 한다. `now`는 그 일이 일어난 시각(초)이다. 과정이 끝난 뒤의 일은 버린다.
    pub fn apply(&mut self, action: Action, now: f64) {
        if self.is_finished() {
            return;
        }
        self.log.push((now, action));
        match action {
            Action::Press(pos, shift) => self.press_key(pos, shift, now),
            Action::Backspace => self.typed.backspace(),
            Action::Rewind => self.rewind_typed(),
        }
    }

    /// 친 글을 틀리기 바로 앞까지 되돌린다. 틀린 줄 모르고 한참 더 쳤을 때, 지우개를 여러 번
    /// 누르지 않고 한 번에 돌아가게 한다. 맞게 친 글자는 하나도 지우지 않는다.
    fn rewind_typed(&mut self) {
        let typed = key_units(&self.typed.text());
        if common_prefix(&typed, &self.target) == typed.len() {
            return;
        }
        let Some(line) = self.current() else {
            return;
        };
        let same = |(typed, wanted): &(char, char)| typed == wanted;
        let pairs = self.typed.done().chars().zip(line.text.chars());
        let right = pairs.take_while(same).count();
        self.typed.keep(right);
    }

    /// 문자 글쇠 하나를 누른다.
    pub fn press(&mut self, pos: KeyPos, shift: bool, now: f64) {
        self.apply(Action::Press(pos, shift), now);
    }

    /// 조합 중이면 낱자 하나를, 아니면 글자 하나를 지운다.
    pub fn backspace(&mut self, now: f64) {
        self.apply(Action::Backspace, now);
    }

    /// 친 글에서 처음 틀린 데부터 뒤를 지운다. 누른 글쇠 수와 시간은 그대로 남는다.
    pub fn rewind(&mut self, now: f64) {
        self.apply(Action::Rewind, now);
    }

    /// 지금까지 일어난 일과 그 시각(초).
    pub fn log(&self) -> &[(f64, Action)] {
        &self.log
    }

    /// 마지막 일이 일어난 시각(초). 아직 아무 일도 없었으면 `None`.
    pub fn last_action_at(&self) -> Option<f64> {
        self.log.last().map(|&(time, _)| time)
    }

    fn press_key(&mut self, pos: KeyPos, shift: bool, now: f64) {
        let number = self.done.len();
        let Some(line) = self.lines.get(number) else {
            return;
        };
        let (script, length) = (line.kind.script(), line.text.chars().count());
        let position = self.matched();
        let expected = self.target.get(position).copied();
        let was_on_track = position == key_units(&self.typed.text()).len();

        self.started_at.get_or_insert(now);
        self.presses += 1;
        let before = self.typed.clone();
        self.typed.press(pos, shift, script);

        if self.matched() < key_units(&self.typed.text()).len() {
            self.wrong += 1;
            // 잘 치다가 처음 어긋난 순간에만, 눌러야 했던 글쇠와 그 자리를 적어 둔다.
            if let Some(expected) = expected.filter(|_| was_on_track) {
                *self.misses.entry(expected).or_default() += 1;
                let slipped = char_of_unit(&self.lines[number].text, position);
                self.slips[number].extend(slipped);
            }
            // 틀린 줄 모르고 계속 쳐도 글이 문장보다 길어지지는 않게 한다.
            // 누른 것은 세되 글에는 넣지 않는다.
            if self.typed.text().chars().count() > length {
                self.typed = before;
            }
        }
        if self.typed.text() == self.lines[number].text {
            self.finish_line(length, now);
        }
    }

    /// 지금 글을 끝낸 것으로 적는다. `chars`는 그 글의 글자 수다.
    fn finish_line(&mut self, chars: usize, now: f64) {
        let needed = self.target.len() as u32;
        self.done.push(Tally {
            needed,
            chars: chars as u32,
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
            .map(|next| key_units(&next.text))
            .unwrap_or_default();
    }

    /// 지금 문장의 시간을 재고 있는지. 문장의 첫 글쇠를 누른 때부터 그 문장을 끝낼 때까지다.
    pub fn is_timing(&self) -> bool {
        self.started_at.is_some()
    }

    /// 치고 있는 문장의 지금까지 기록. 어긋나지 않게 친 데까지만 넣는다.
    /// 그 문장의 첫 글쇠를 아직 누르지 않았으면 `None`.
    pub fn in_progress(&self, now: f64) -> Option<Tally> {
        let started_at = self.started_at?;
        let line = self.current()?;
        let matched = self.matched();
        // 다 친 글자 수는, 다음에 칠 글쇠가 든 글자의 번호와 같다.
        let chars = char_of_unit(&line.text, matched).unwrap_or(line.text.chars().count());
        Some(Tally {
            needed: matched as u32,
            chars: chars as u32,
            presses: self.presses,
            wrong: self.wrong,
            timed: (matched as u32).saturating_sub(1),
            seconds: now - started_at,
        })
    }

    /// 지금까지의 기록: 끝낸 문장들에 치고 있는 문장을 더한 것.
    pub fn so_far(&self, now: f64) -> Tally {
        let finished: Tally = self.done.iter().copied().sum();
        finished + self.in_progress(now).unwrap_or_default()
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
        let lines = lines.iter().map(|&(kind, text)| Sentence::new(kind, text));
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
        assert_eq!(round.current().map(|line| line.text.as_str()), Some("ok"));
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
        round.backspace(0.0);
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
        round.backspace(0.0);
        round.backspace(0.0);
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
        round.backspace(0.0);
        press_all(&mut round, "ㅗ", 1.0);
        round.backspace(0.0);
        press_all(&mut round, "ㅏㅁ", 2.0);
        round.backspace(0.0);
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

    /// 적어 둔 일을 새 과정에 그대로 다시 하면 같은 기록이 나온다.
    #[test]
    fn replaying_the_log_gives_the_same_round() {
        let mut round = round(&[(Kind::Hangul, "가나"), (Kind::English, "ok")]);
        press_all(&mut round, "ㄱㅓ", 0.0);
        round.backspace(2.5);
        press_all(&mut round, "ㅏㄴㅏ", 3.0);
        press_all(&mut round, "x", 7.0);
        round.rewind(8.0);
        press_all(&mut round, "ok", 9.0);
        press_all(&mut round, "z", 20.0);
        assert!(round.is_finished());
        assert_eq!(
            round.log().len(),
            10,
            "과정이 끝난 뒤에 누른 것은 적지 않는다"
        );

        let mut again = Round::new(round.lines().to_vec());
        for &(time, action) in round.log() {
            again.apply(action, time);
        }
        assert_eq!(again.results(), round.results());
        assert_eq!(again.misses(), round.misses());
        assert_eq!(again.slips(), round.slips());
    }

    #[test]
    fn rewinding_keeps_what_was_pressed() {
        let mut round = round(&[(Kind::English, "ab")]);
        press_all(&mut round, "x", 0.0);
        round.rewind(0.0);
        press_all(&mut round, "ab", 1.0);
        let line = round.results()[0];
        assert_eq!((line.presses, line.wrong, line.seconds), (3, 1, 2.0));
    }

    /// 되돌리면 틀리기 바로 앞까지만 지워진다. 긴 문단에서 맞게 친 앞부분을 잃지 않는다.
    #[test]
    fn rewinding_goes_back_only_as_far_as_the_slip() {
        let mut round = round(&[(Kind::English, "the quick fox")]);
        press_all(&mut round, "the quack f", 0.0);
        round.rewind(20.0);
        assert_eq!(round.typed().text(), "the qu");
        // 틀린 데가 없으면 그대로다.
        round.rewind(21.0);
        assert_eq!(round.typed().text(), "the qu");
        press_all(&mut round, "ick fox", 22.0);
        assert!(round.is_finished());

        // 한글은 글자 단위로 돌아간다. 조합 중인 글자가 틀렸으면 그 글자만 지운다.
        let mut round = self::round(&[(Kind::Hangul, "가는 말이 곱다")]);
        press_all(&mut round, "ㄱㅏㄴㅡㄴ ㅁㅏㄹㅇㅣ ㄱㅗㄱ", 0.0);
        assert_eq!(round.typed().text(), "가는 말이 곡");
        round.rewind(30.0);
        assert_eq!(round.typed().text(), "가는 말이 ");
        // 앞 글자의 홀소리를 틀리고 더 쳤으면 그 글자부터 지운다.
        press_all(&mut round, "ㄱㅜㅂㄷㅏ", 31.0);
        assert_eq!(round.typed().text(), "가는 말이 굽다");
        round.rewind(40.0);
        assert_eq!(round.typed().text(), "가는 말이 ");
        // 받침이 다음 글자로 넘어가기를 기다리는 것은 틀린 것이 아니다.
        press_all(&mut round, "ㄱㅗㅂㄷ", 41.0);
        assert_eq!(round.typed().text(), "가는 말이 곱ㄷ");
        round.rewind(50.0);
        assert_eq!(round.typed().text(), "가는 말이 곱ㄷ");
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

    /// 문장을 이어 붙이면 과정이 끝나지 않고 계속된다. 이미 끝난 과정에 붙여도 이어서 칠 수 있다.
    #[test]
    fn more_lines_can_be_added_while_typing() {
        let mut round = round(&[(Kind::English, "ab")]);
        press_all(&mut round, "a", 0.0);
        let more = |text: &str| Sentence::new(Kind::English, text);
        round.extend(vec![more("cd")]);
        assert_eq!(round.lines().len(), 2);
        assert_eq!(round.progress(), 0.25);
        press_all(&mut round, "bcd", 1.0);
        assert!(round.is_finished());

        round.extend(vec![more("xy")]);
        assert!(!round.is_finished());
        // 붙인 문장에서 틀린 자리도 적힌다.
        press_all(&mut round, "xz", 10.0);
        assert_eq!(round.slips().len(), 3);
        assert_eq!(round.slips()[2].iter().copied().collect::<Vec<_>>(), [1]);
        assert_eq!(round.results().len(), 2);
    }

    /// 치다 만 문장의 기록에는 어긋나지 않게 친 데까지만 든다.
    #[test]
    fn the_line_in_progress_counts_what_was_typed_right() {
        let mut round = round(&[(Kind::Hangul, "가나다")]);
        assert_eq!(round.in_progress(0.0), None, "첫 글쇠를 누르기 전");
        // '가'와 '나'의 닿소리까지 치고, 홀소리를 틀린다.
        press_all(&mut round, "ㄱㅏㄴㅓ", 0.0);
        let partial = round.in_progress(5.0).expect("치는 중");
        assert_eq!((partial.needed, partial.presses, partial.wrong), (3, 4, 1));
        assert_eq!(partial.chars, 1, "다 친 글자는 '가' 하나다");
        assert_eq!((partial.timed, partial.seconds), (2, 5.0));
        assert_eq!(round.reached(), ['ㄱ', 'ㅏ', 'ㄴ']);
        assert_eq!(round.so_far(5.0), partial);
    }

    /// 처음에 준 글들이 첫 쪽이고, 이어 붙일 때마다 새 쪽이 된다.
    #[test]
    fn lines_added_later_form_new_pages() {
        let more = |texts: &[&str]| -> Vec<Sentence> {
            let line = |text: &&str| Sentence::new(Kind::English, *text);
            texts.iter().map(line).collect()
        };
        let mut round = Round::new(more(&["ab", "cd"]));
        assert_eq!(round.page_of(0), 0..2);
        round.extend(more(&["ef"]));
        round.extend(more(&["gh", "ij", "kl"]));
        let pages: Vec<_> = (0..6).map(|line| round.page_of(line)).collect();
        assert_eq!(pages, [0..2, 0..2, 2..3, 3..6, 3..6, 3..6]);
        assert_eq!(round.page_of(9), 3..6, "마지막 글보다 뒤면 마지막 쪽");

        // 빈 과정에 붙인 글은 첫 쪽이 된다.
        let mut round = Round::new(Vec::new());
        round.extend(more(&["ab"]));
        round.extend(more(&["cd"]));
        assert_eq!((round.page_of(0), round.page_of(1)), (0..1, 1..2));

        // 쪽 번호와 끝낸 쪽 수.
        assert_eq!((round.page_number(0), round.page_number(1)), (0, 1));
        assert_eq!(round.pages_done(), 0);
        press_all(&mut round, "ab", 0.0);
        assert_eq!(round.pages_done(), 1);

        // 같은 글을 처음부터 다시 칠 때도 쪽 나눔은 그대로다.
        let fresh = round.fresh();
        assert_eq!(fresh.lines(), round.lines());
        assert_eq!((fresh.page_of(0), fresh.page_of(1)), (0..1, 1..2));
        assert!(fresh.results().is_empty() && fresh.log().is_empty());
    }

    /// 틀린 자리는 낱말로 묶어 돌려준다. 긴 글을 통째로 보이지 않고 틀린 낱말만 보이려는 것이다.
    #[test]
    fn slips_are_gathered_by_word() {
        let mut round = round(&[(Kind::English, "the cat sat"), (Kind::Hangul, "가나 다라")]);
        // 'cat'의 a와 t를 한 번씩 틀리고, 'sat' 앞의 빈칸을 틀린다.
        press_all(&mut round, "the cx", 0.0);
        round.backspace(0.0);
        press_all(&mut round, "ax", 1.0);
        round.backspace(1.0);
        press_all(&mut round, "tx", 2.0);
        round.backspace(2.0);
        press_all(&mut round, " sat", 3.0);
        // '다'의 홀소리를 틀린다.
        press_all(&mut round, "ㄱㅏㄴㅏ ㄷㅓ", 10.0);
        let words = round.slipped_words();
        let expected = [
            ("cat".to_owned(), vec![1, 2, 3]),
            ("다라".to_owned(), vec![0]),
        ];
        assert_eq!(words, expected);
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
                    round.backspace(0.0);
                } else {
                    press_all(&mut round, &unit.to_string(), 0.0);
                }
            }
        }
        assert_eq!(round.misses(), [('a', 2), ('b', 1)]);
    }
}
