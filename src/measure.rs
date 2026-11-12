//! 타수와 정확도.
//!
//! 타수는 글을 치는 데 누르는 글쇠 수다. 한글은 낱자마다, 그 밖의 글자는 글자마다 1타로 세고
//! Shift는 세지 않는다 ('값'은 ㄱㅏㅂㅅ 4타, 'A'는 1타).

use std::iter::Sum;
use std::ops::Add;

use crate::hangul::{Jamo, strokes};

/// 글을 치는 글쇠 순서. 한글은 낱자로 풀고, 그 밖의 글자는 그대로 둔다.
pub fn key_units(text: &str) -> Vec<char> {
    let mut units = Vec::with_capacity(text.len());
    for c in text.chars() {
        match strokes(c) {
            Some(keys) => units.extend(keys.into_iter().map(Jamo::as_char)),
            None => units.push(c),
        }
    }
    units
}

/// `text`를 치는 글쇠 가운데 `unit`번째(0부터)가 몇 번째 글자에 드는지.
/// 글쇠가 문장 밖이면 `None`.
pub fn char_of_unit(text: &str, unit: usize) -> Option<usize> {
    let mut keys = 0;
    text.chars().position(|c| {
        keys += strokes(c).map_or(1, |keys| keys.len());
        unit < keys
    })
}

/// 두 글쇠 순서가 앞에서부터 몇 개나 같은지.
pub fn common_prefix(a: &[char], b: &[char]) -> usize {
    a.iter().zip(b).take_while(|(x, y)| x == y).count()
}

/// 문장 하나의 기록, 또는 여러 문장의 기록을 더한 것.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Tally {
    /// 문장을 치는 데 필요한 글쇠 수.
    pub needed: u32,
    /// 문장의 글자 수. 영문 WPM을 낼 때 쓴다.
    pub chars: u32,
    /// 실제로 누른 문자 글쇠 수.
    pub presses: u32,
    /// 그중 누르고 난 글이 문장과 어긋나 있던 횟수.
    pub wrong: u32,
    /// 시간을 재는 동안 친 글쇠 수. 문장의 첫 글쇠는 시계를 켜는 신호라서 세지 않는다.
    pub timed: u32,
    /// 문장의 첫 글쇠부터 마지막 글쇠까지 걸린 시간(초).
    pub seconds: f64,
}

impl Tally {
    /// 1분에 치는 글쇠 수 (타/분).
    ///
    /// 첫 글쇠를 누른 순간부터 재므로, 그 뒤에 친 글쇠 수를 걸린 시간으로 나눈다.
    /// 필요한 글쇠 수를 그대로 나누면 짧은 문장일수록 속도가 부풀려진다.
    pub fn per_minute(&self) -> f64 {
        if self.seconds > 0.0 {
            f64::from(self.timed) * 60.0 / self.seconds
        } else {
            0.0
        }
    }

    /// 1분에 치는 낱말 수. 다섯 글자를 한 낱말로 친다.
    pub fn wpm(&self) -> f64 {
        if self.needed == 0 {
            return 0.0;
        }
        self.per_minute() * f64::from(self.chars) / f64::from(self.needed) / 5.0
    }

    /// 누른 글쇠 가운데 맞게 누른 비율 (0 ~ 1).
    pub fn accuracy(&self) -> f64 {
        if self.presses == 0 {
            return 1.0;
        }
        1.0 - f64::from(self.wrong) / f64::from(self.presses)
    }
}

impl Add for Tally {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            needed: self.needed + other.needed,
            chars: self.chars + other.chars,
            presses: self.presses + other.presses,
            wrong: self.wrong + other.wrong,
            timed: self.timed + other.timed,
            seconds: self.seconds + other.seconds,
        }
    }
}

impl Sum for Tally {
    fn sum<I: Iterator<Item = Self>>(tallies: I) -> Self {
        tallies.fold(Self::default(), Add::add)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hangul_counts_one_key_per_jamo() {
        assert_eq!(key_units("값"), ['ㄱ', 'ㅏ', 'ㅂ', 'ㅅ']);
        assert_eq!(key_units("과자"), ['ㄱ', 'ㅗ', 'ㅏ', 'ㅈ', 'ㅏ']);
        // 쌍자음은 Shift와 함께 누르는 글쇠 하나다.
        assert_eq!(key_units("까"), ['ㄲ', 'ㅏ']);
    }

    #[test]
    fn other_characters_count_one_key_each() {
        assert_eq!(key_units("Ab 1!"), ['A', 'b', ' ', '1', '!']);
        assert_eq!(key_units("가 a").len(), 4);
        assert_eq!(key_units(""), []);
    }

    #[test]
    fn char_of_unit_finds_the_character_a_key_belongs_to() {
        let found: Vec<_> = (0..5).map(|unit| char_of_unit("가나", unit)).collect();
        assert_eq!(found, [Some(0), Some(0), Some(1), Some(1), None]);
        // 'a'는 1타, '값'은 4타다.
        let found: Vec<_> = (0..6).map(|unit| char_of_unit("a값", unit)).collect();
        assert_eq!(found, [Some(0), Some(1), Some(1), Some(1), Some(1), None]);
        assert_eq!(char_of_unit("", 0), None);
    }

    #[test]
    fn common_prefix_stops_at_the_first_difference() {
        let (a, b) = (key_units("가나다"), key_units("가너다"));
        assert_eq!(common_prefix(&a, &b), 3);
        assert_eq!(common_prefix(&a, &a), a.len());
        assert_eq!(common_prefix(&a, &[]), 0);
    }

    /// 0.2초에 한 글쇠씩 101개를 치면, 첫 글쇠 뒤로 100개를 20초에 친 것이다.
    #[test]
    fn speed_counts_the_keys_typed_after_the_clock_started() {
        let tally = Tally {
            needed: 101,
            chars: 101,
            presses: 160,
            wrong: 8,
            timed: 100,
            seconds: 20.0,
        };
        assert_eq!(tally.per_minute(), 300.0);
        assert_eq!(tally.wpm(), 60.0);
        assert_eq!(tally.accuracy(), 0.95);
    }

    /// 한글은 글자 하나에 글쇠가 여럿이므로, 같은 타수라도 낱말 수는 적다.
    #[test]
    fn wpm_follows_characters_not_keys() {
        let tally = Tally {
            needed: 250,
            chars: 100,
            timed: 250,
            seconds: 60.0,
            ..Tally::default()
        };
        assert_eq!(tally.per_minute(), 250.0);
        assert_eq!(tally.wpm(), 20.0);
    }

    #[test]
    fn empty_tally_does_not_divide_by_zero() {
        let tally = Tally::default();
        assert_eq!(tally.per_minute(), 0.0);
        assert_eq!(tally.wpm(), 0.0);
        assert_eq!(tally.accuracy(), 1.0);
    }

    #[test]
    fn tallies_add_up() {
        let line = Tally {
            needed: 10,
            chars: 5,
            presses: 12,
            wrong: 1,
            timed: 9,
            seconds: 2.0,
        };
        let total: Tally = [line, line, line].into_iter().sum();
        assert_eq!(total.needed, 30);
        assert_eq!(total.presses, 36);
        assert_eq!(total.seconds, 6.0);
        // 속도는 문장별 평균이 아니라 전체 글쇠 수를 전체 시간으로 나눈 값이다.
        assert_eq!(total.per_minute(), line.per_minute());
    }
}
