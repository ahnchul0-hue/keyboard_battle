//! 글쇠 위치와 자판 배열.
//!
//! 입력은 "어떤 문자가 들어왔나"가 아니라 "어느 자리의 글쇠가 눌렸나"로 받는다.
//! 그래야 OS 입력기가 한글이든 영문이든 같은 결과가 나오고, 어느 손가락을 썼는지도 알 수 있다.

use crate::hangul::Jamo;

/// 미국식 쿼티 자판의 문자 글쇠 48개 (사이띄개 포함).
const PLAIN: &[u8; 48] = b"`1234567890-=qwertyuiop[]\\asdfghjkl;'zxcvbnm,./ ";
/// 같은 글쇠를 Shift와 함께 눌렀을 때.
const SHIFTED: &[u8; 48] = b"~!@#$%^&*()_+QWERTYUIOP{}|ASDFGHJKL:\"ZXCVBNM<>? ";

/// 두벌식에서 한글 낱자를 내는 글쇠와 그 낱자.
const HANGUL_KEYS: &str = "qwertyuiopasdfghjklzxcvbnm";
const HANGUL_PLAIN: &str = "ㅂㅈㄷㄱㅅㅛㅕㅑㅐㅔㅁㄴㅇㄹㅎㅗㅓㅏㅣㅋㅌㅊㅍㅠㅜㅡ";
const HANGUL_SHIFTED: &str = "ㅃㅉㄸㄲㅆㅛㅕㅑㅒㅖㅁㄴㅇㄹㅎㅗㅓㅏㅣㅋㅌㅊㅍㅠㅜㅡ";

/// 손.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hand {
    Left,
    Right,
}

impl Hand {
    pub fn name(self) -> &'static str {
        match self {
            Self::Left => "왼손",
            Self::Right => "오른손",
        }
    }
}

/// 손가락.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Finger {
    Pinky,
    Ring,
    Middle,
    Index,
    Thumb,
}

impl Finger {
    pub fn name(self) -> &'static str {
        match self {
            Self::Pinky => "새끼손가락",
            Self::Ring => "약손가락",
            Self::Middle => "가운뎃손가락",
            Self::Index => "집게손가락",
            Self::Thumb => "엄지손가락",
        }
    }
}

/// 자판의 줄.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Row {
    Number,
    Top,
    Home,
    Bottom,
    Space,
}

impl Row {
    pub fn name(self) -> &'static str {
        match self {
            Self::Number => "숫자 줄",
            Self::Top => "윗줄",
            Self::Home => "가운데 줄",
            Self::Bottom => "아랫줄",
            Self::Space => "맨 아랫줄",
        }
    }
}

/// 표준 운지법에서 손가락마다 맡는 글쇠. 왼손 새끼손가락부터 오른손 새끼손가락까지다.
const REACH: [(Hand, Finger, &str); 8] = [
    (Hand::Left, Finger::Pinky, "`1qaz"),
    (Hand::Left, Finger::Ring, "2wsx"),
    (Hand::Left, Finger::Middle, "3edc"),
    (Hand::Left, Finger::Index, "45rtfgvb"),
    (Hand::Right, Finger::Index, "67yuhjnm"),
    (Hand::Right, Finger::Middle, "8ik,"),
    (Hand::Right, Finger::Ring, "9ol."),
    (Hand::Right, Finger::Pinky, "0-=p[]\\;'/"),
];

/// 글쇠 위치. 값은 [`PLAIN`] 안에서의 순서다.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KeyPos(u8);

impl KeyPos {
    /// 미국식 쿼티 자판에서 Shift 없이 `legend`를 내는 글쇠.
    pub fn from_us(legend: char) -> Option<Self> {
        let index = PLAIN.iter().position(|&b| char::from(b) == legend)?;
        Some(Self(index as u8))
    }

    /// 모든 문자 글쇠.
    pub fn all() -> impl Iterator<Item = Self> {
        (0..PLAIN.len() as u8).map(Self)
    }
}

/// 표준 운지법에서 이 글쇠를 치는 손과 손가락. 사이띄개는 어느 쪽 엄지로든 치므로 손이 없다.
pub fn finger(pos: KeyPos) -> (Option<Hand>, Finger) {
    let legend = qwerty(pos, false);
    let reach = REACH.iter().find(|(_, _, keys)| keys.contains(legend));
    reach.map_or((None, Finger::Thumb), |&(hand, finger, _)| {
        (Some(hand), finger)
    })
}

/// 이 글쇠가 놓인 줄.
pub fn row(pos: KeyPos) -> Row {
    match pos.0 {
        0..=12 => Row::Number,
        13..=25 => Row::Top,
        26..=36 => Row::Home,
        37..=46 => Row::Bottom,
        _ => Row::Space,
    }
}

/// 쿼티 배열에서 이 글쇠가 내는 문자.
pub fn qwerty(pos: KeyPos, shift: bool) -> char {
    let row = if shift { SHIFTED } else { PLAIN };
    char::from(row[usize::from(pos.0)])
}

/// 두벌식 배열에서 이 글쇠가 내는 낱자. 숫자·기호·사이띄개처럼 한글 글쇠가 아니면 `None`.
pub fn dubeolsik(pos: KeyPos, shift: bool) -> Option<Jamo> {
    let index = HANGUL_KEYS.find(qwerty(pos, false))?;
    let row = if shift { HANGUL_SHIFTED } else { HANGUL_PLAIN };
    row.chars().nth(index).and_then(Jamo::new)
}

/// 낱자나 문자 하나를 치려면 누르는 글쇠와 Shift 여부. 칠 수 없는 문자면 `None`.
///
/// Shift 없이도 나오는 낱자는 Shift 없는 쪽을 고른다.
pub fn keystroke_for(unit: char) -> Option<(KeyPos, bool)> {
    [false, true]
        .into_iter()
        .flat_map(|shift| KeyPos::all().map(move |pos| (pos, shift)))
        .find(|&(pos, shift)| {
            dubeolsik(pos, shift).map(Jamo::as_char) == Some(unit) || qwerty(pos, shift) == unit
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(legend: char) -> KeyPos {
        KeyPos::from_us(legend).expect("문자 글쇠")
    }

    #[test]
    fn qwerty_matches_the_us_layout() {
        assert_eq!(KeyPos::all().count(), 48);
        for pos in KeyPos::all() {
            let (plain, shifted) = (qwerty(pos, false), qwerty(pos, true));
            assert_eq!(KeyPos::from_us(plain), Some(pos));
            if plain.is_ascii_lowercase() {
                assert_eq!(shifted, plain.to_ascii_uppercase());
            }
        }
        let pairs = [('1', '!'), ('7', '&'), ('-', '_'), ('[', '{'), ('\\', '|')];
        let more = [(';', ':'), ('\'', '"'), (',', '<'), ('/', '?'), ('`', '~')];
        for (plain, shifted) in pairs.into_iter().chain(more) {
            assert_eq!(qwerty(key(plain), true), shifted);
        }
        assert_eq!(qwerty(key(' '), true), ' ');
        assert_eq!(KeyPos::from_us('A'), None);
        assert_eq!(KeyPos::from_us('한'), None);
    }

    /// 문자 글쇠는 저마다 손가락 하나가 맡고, 사이띄개만 엄지가 맡는다.
    #[test]
    fn every_key_belongs_to_one_finger() {
        let assigned: String = REACH.iter().map(|(_, _, keys)| *keys).collect();
        let mut sorted: Vec<char> = assigned.chars().collect();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(
            sorted.len(),
            assigned.chars().count(),
            "두 손가락이 맡은 글쇠"
        );
        assert_eq!(sorted.len(), 47);

        assert_eq!(finger(key('f')), (Some(Hand::Left), Finger::Index));
        assert_eq!(finger(key('j')), (Some(Hand::Right), Finger::Index));
        assert_eq!(finger(key('a')), (Some(Hand::Left), Finger::Pinky));
        assert_eq!(finger(key(';')), (Some(Hand::Right), Finger::Pinky));
        assert_eq!(finger(key('c')), (Some(Hand::Left), Finger::Middle));
        assert_eq!(finger(key('\\')), (Some(Hand::Right), Finger::Pinky));
        assert_eq!(finger(key(' ')), (None, Finger::Thumb));
        for pos in KeyPos::all().filter(|&pos| pos != key(' ')) {
            assert!(finger(pos).0.is_some(), "{:?}", qwerty(pos, false));
        }
    }

    #[test]
    fn rows_follow_the_keyboard() {
        let rows = [
            ("`1234567890-=", Row::Number),
            ("qwertyuiop[]\\", Row::Top),
            ("asdfghjkl;'", Row::Home),
            ("zxcvbnm,./", Row::Bottom),
            (" ", Row::Space),
        ];
        for (keys, expected) in rows {
            for legend in keys.chars() {
                assert_eq!(row(key(legend)), expected, "{legend:?}");
            }
        }
    }

    #[test]
    fn dubeolsik_matches_the_standard_layout() {
        let jamo = |legend, shift| dubeolsik(key(legend), shift).map(Jamo::as_char);
        assert_eq!(jamo('r', false), Some('ㄱ'));
        assert_eq!(jamo('r', true), Some('ㄲ'));
        assert_eq!(jamo('o', false), Some('ㅐ'));
        assert_eq!(jamo('o', true), Some('ㅒ'));
        assert_eq!(jamo('p', true), Some('ㅖ'));
        assert_eq!(jamo('a', true), Some('ㅁ'));
        assert_eq!(jamo('m', false), Some('ㅡ'));
        for legend in ['1', ';', ' ', '/'] {
            assert_eq!(jamo(legend, false), None);
            assert_eq!(jamo(legend, true), None);
        }
    }

    #[test]
    fn every_letter_key_gives_a_distinct_jamo() {
        let mut plain: Vec<char> = Vec::new();
        for legend in 'a'..='z' {
            for shift in [false, true] {
                assert!(dubeolsik(key(legend), shift).is_some(), "{legend}");
            }
            plain.extend(dubeolsik(key(legend), false).map(Jamo::as_char));
        }
        plain.sort_unstable();
        plain.dedup();
        assert_eq!(plain.len(), 26);
    }

    #[test]
    fn keystroke_for_finds_the_key_that_types_a_character() {
        assert_eq!(keystroke_for('ㄱ'), Some((key('r'), false)));
        assert_eq!(keystroke_for('ㄲ'), Some((key('r'), true)));
        // ㅁ은 Shift를 눌러도 나오지만 Shift 없는 쪽을 고른다.
        assert_eq!(keystroke_for('ㅁ'), Some((key('a'), false)));
        assert_eq!(keystroke_for('a'), Some((key('a'), false)));
        assert_eq!(keystroke_for('{'), Some((key('['), true)));
        assert_eq!(keystroke_for(' '), Some((key(' '), false)));
        // 겹낱자와 완성된 글자는 글쇠 하나로 칠 수 없다.
        for unit in ['ㅘ', 'ㄳ', '가', 'é', '\n'] {
            assert_eq!(keystroke_for(unit), None, "{unit:?}");
        }
    }
}
