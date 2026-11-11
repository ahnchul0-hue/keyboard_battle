//! 두벌식 한글 조합기.
//!
//! OS 입력기를 거치지 않고, 글쇠가 낸 낱자를 받아 직접 글자를 조합한다.
//! 낱자는 모두 한글 호환 자모(U+3131..=U+3163) 문자로 다룬다.
//!
//! 조합 규칙은 두벌식 표준을 따른다. 윈도우 기본 입력기와 다른 점은 하나다.
//! 가운뎃소리 없이 닿소리만 이어 치면 겹받침 낱자(ㄳ 등)로 합치지 않고 따로 둔다.

/// 첫 한글 음절 '가'의 코드.
const SYLLABLE_BASE: u32 = 0xAC00;
const SYLLABLE_COUNT: u32 = 19 * 21 * 28;

/// 첫소리. 순서는 유니코드 음절 배열과 같다.
const CHO: [char; 19] = [
    'ㄱ', 'ㄲ', 'ㄴ', 'ㄷ', 'ㄸ', 'ㄹ', 'ㅁ', 'ㅂ', 'ㅃ', 'ㅅ', 'ㅆ', 'ㅇ', 'ㅈ', 'ㅉ', 'ㅊ', 'ㅋ',
    'ㅌ', 'ㅍ', 'ㅎ',
];

/// 가운뎃소리. 순서는 유니코드 음절 배열과 같다.
const JUNG: [char; 21] = [
    'ㅏ', 'ㅐ', 'ㅑ', 'ㅒ', 'ㅓ', 'ㅔ', 'ㅕ', 'ㅖ', 'ㅗ', 'ㅘ', 'ㅙ', 'ㅚ', 'ㅛ', 'ㅜ', 'ㅝ', 'ㅞ',
    'ㅟ', 'ㅠ', 'ㅡ', 'ㅢ', 'ㅣ',
];

/// 받침. 유니코드 음절 배열에서 받침 번호는 여기 순서에 1을 더한 값이다 (0은 받침 없음).
const JONG: [char; 27] = [
    'ㄱ', 'ㄲ', 'ㄳ', 'ㄴ', 'ㄵ', 'ㄶ', 'ㄷ', 'ㄹ', 'ㄺ', 'ㄻ', 'ㄼ', 'ㄽ', 'ㄾ', 'ㄿ', 'ㅀ', 'ㅁ',
    'ㅂ', 'ㅄ', 'ㅅ', 'ㅆ', 'ㅇ', 'ㅈ', 'ㅊ', 'ㅋ', 'ㅌ', 'ㅍ', 'ㅎ',
];

/// 글쇠 하나로 치는 홀소리.
const KEY_VOWELS: [char; 14] = [
    'ㅏ', 'ㅐ', 'ㅑ', 'ㅒ', 'ㅓ', 'ㅔ', 'ㅕ', 'ㅖ', 'ㅗ', 'ㅛ', 'ㅜ', 'ㅠ', 'ㅡ', 'ㅣ',
];

/// 두 글쇠로 치는 겹홀소리: (먼저 치는 낱자, 나중에 치는 낱자, 합친 낱자).
const COMPOUND_VOWELS: [(char, char, char); 7] = [
    ('ㅗ', 'ㅏ', 'ㅘ'),
    ('ㅗ', 'ㅐ', 'ㅙ'),
    ('ㅗ', 'ㅣ', 'ㅚ'),
    ('ㅜ', 'ㅓ', 'ㅝ'),
    ('ㅜ', 'ㅔ', 'ㅞ'),
    ('ㅜ', 'ㅣ', 'ㅟ'),
    ('ㅡ', 'ㅣ', 'ㅢ'),
];

/// 두 글쇠로 치는 겹받침: (먼저 치는 낱자, 나중에 치는 낱자, 합친 낱자).
const COMPOUND_FINALS: [(char, char, char); 11] = [
    ('ㄱ', 'ㅅ', 'ㄳ'),
    ('ㄴ', 'ㅈ', 'ㄵ'),
    ('ㄴ', 'ㅎ', 'ㄶ'),
    ('ㄹ', 'ㄱ', 'ㄺ'),
    ('ㄹ', 'ㅁ', 'ㄻ'),
    ('ㄹ', 'ㅂ', 'ㄼ'),
    ('ㄹ', 'ㅅ', 'ㄽ'),
    ('ㄹ', 'ㅌ', 'ㄾ'),
    ('ㄹ', 'ㅍ', 'ㄿ'),
    ('ㄹ', 'ㅎ', 'ㅀ'),
    ('ㅂ', 'ㅅ', 'ㅄ'),
];

fn join(table: &[(char, char, char)], first: char, second: char) -> Option<char> {
    table
        .iter()
        .find(|&&(a, b, _)| a == first && b == second)
        .map(|&(_, _, joined)| joined)
}

fn split(table: &[(char, char, char)], joined: char) -> Option<(char, char)> {
    table
        .iter()
        .find(|&&(_, _, j)| j == joined)
        .map(|&(a, b, _)| (a, b))
}

fn syllable(cho: char, jung: char, jong: Option<char>) -> char {
    let index = |table: &[char], jamo: char| {
        let found = table.iter().position(|&c| c == jamo);
        found.expect("조합기가 쥐고 있는 낱자는 늘 표 안에 있다") as u32
    };
    let jong = jong.map_or(0, |jong| index(&JONG, jong) + 1);
    let code = SYLLABLE_BASE + (index(&CHO, cho) * 21 + index(&JUNG, jung)) * 28 + jong;
    char::from_u32(code).expect("한글 음절 범위 안의 코드다")
}

/// 두벌식 자판에서 글쇠 하나가 내는 낱자 (닿소리 19개, 홀소리 14개).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Jamo(char);

impl Jamo {
    /// 글쇠 하나로 칠 수 있는 낱자일 때만 만든다. 겹홀소리와 겹받침은 두 글쇠로 치므로 `None`이다.
    pub fn new(c: char) -> Option<Self> {
        (CHO.contains(&c) || KEY_VOWELS.contains(&c)).then_some(Self(c))
    }

    pub fn as_char(self) -> char {
        self.0
    }

    pub fn is_vowel(self) -> bool {
        KEY_VOWELS.contains(&self.0)
    }
}

/// 조합 중인 글자 하나.
///
/// 낱자를 [`push`](Self::push)로 넣으면, 그 낱자 때문에 완성되어 밀려난 글자를 돌려준다.
/// 아직 조합 중인 글자는 [`preedit`](Self::preedit)로 본다.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Composer {
    cho: Option<char>,
    /// 겹홀소리는 합친 낱자로 쥔다.
    jung: Option<char>,
    /// 겹받침은 합친 낱자로 쥔다. 첫소리와 가운뎃소리가 모두 있을 때만 값이 있다.
    jong: Option<char>,
}

impl Composer {
    pub fn new() -> Self {
        Self::default()
    }

    /// 조합 중인 글자. 없으면 `None`.
    pub fn preedit(&self) -> Option<char> {
        match (self.cho, self.jung) {
            (Some(cho), Some(jung)) => Some(syllable(cho, jung, self.jong)),
            (lone_cho, lone_jung) => lone_cho.or(lone_jung),
        }
    }

    /// 낱자 하나를 넣는다. 이 낱자 때문에 완성된 글자가 있으면 돌려준다.
    pub fn push(&mut self, jamo: Jamo) -> Option<char> {
        if jamo.is_vowel() {
            self.push_vowel(jamo.0)
        } else {
            self.push_consonant(jamo.0)
        }
    }

    fn push_vowel(&mut self, vowel: char) -> Option<char> {
        // 받침 뒤에 홀소리가 오면 받침(겹받침이면 뒤쪽 낱자)이 다음 글자의 첫소리로 넘어간다.
        if let Some(jong) = self.jong {
            let (stays, moves) = match split(&COMPOUND_FINALS, jong) {
                Some((first, second)) => (Some(first), second),
                None => (None, jong),
            };
            self.jong = stays;
            return self.start(Some(moves), Some(vowel));
        }
        match self.jung {
            None => {
                self.jung = Some(vowel);
                None
            }
            Some(jung) => match join(&COMPOUND_VOWELS, jung, vowel) {
                Some(joined) => {
                    self.jung = Some(joined);
                    None
                }
                None => self.start(None, Some(vowel)),
            },
        }
    }

    fn push_consonant(&mut self, consonant: char) -> Option<char> {
        let has_vowel = self.cho.is_some() && self.jung.is_some();
        let new_jong = match self.jong {
            None if has_vowel && JONG.contains(&consonant) => Some(consonant),
            Some(jong) => join(&COMPOUND_FINALS, jong, consonant),
            None => None,
        };
        match new_jong {
            Some(jong) => {
                self.jong = Some(jong);
                None
            }
            None => self.start(Some(consonant), None),
        }
    }

    /// 지금 글자를 끝내고 새 글자를 시작한다. 끝난 글자를 돌려준다.
    fn start(&mut self, cho: Option<char>, jung: Option<char>) -> Option<char> {
        let done = self.preedit();
        *self = Self {
            cho,
            jung,
            jong: None,
        };
        done
    }

    /// 조합 중인 글자에서 마지막에 친 낱자 하나를 지운다. 조합 중인 글자가 없으면 `false`.
    pub fn backspace(&mut self) -> bool {
        if let Some(jong) = self.jong {
            self.jong = split(&COMPOUND_FINALS, jong).map(|(first, _)| first);
        } else if let Some(jung) = self.jung {
            self.jung = split(&COMPOUND_VOWELS, jung).map(|(first, _)| first);
        } else if self.cho.is_some() {
            self.cho = None;
        } else {
            return false;
        }
        true
    }

    /// 조합을 끝내고, 조합 중이던 글자를 돌려준다.
    pub fn flush(&mut self) -> Option<char> {
        self.start(None, None)
    }
}

/// 글자 하나를 두벌식으로 칠 때 누르는 낱자 순서.
///
/// 이 순서대로 [`Composer`]에 넣으면 같은 글자가 나온다. 그렇게 칠 수 없는 글자면 `None`.
pub fn strokes(c: char) -> Option<Vec<Jamo>> {
    let parts = |table: &[(char, char, char)], jamo: char| match split(table, jamo) {
        Some((first, second)) => vec![Jamo(first), Jamo(second)],
        None => vec![Jamo(jamo)],
    };
    if Jamo::new(c).is_some() || split(&COMPOUND_VOWELS, c).is_some() {
        return Some(parts(&COMPOUND_VOWELS, c));
    }
    let offset = (c as u32)
        .checked_sub(SYLLABLE_BASE)
        .filter(|&offset| offset < SYLLABLE_COUNT)? as usize;
    let mut out = vec![Jamo(CHO[offset / (21 * 28)])];
    out.extend(parts(&COMPOUND_VOWELS, JUNG[offset / 28 % 21]));
    if let Some(jong) = (offset % 28).checked_sub(1) {
        out.extend(parts(&COMPOUND_FINALS, JONG[jong]));
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 낱자를 차례로 넣어 나온 글 전체.
    fn compose(jamo: impl IntoIterator<Item = Jamo>) -> String {
        let mut composer = Composer::new();
        let mut out = String::new();
        for jamo in jamo {
            out.extend(composer.push(jamo));
        }
        out.extend(composer.flush());
        out
    }

    fn typed(jamo: &str) -> String {
        compose(jamo.chars().map(|c| Jamo::new(c).expect("글쇠 낱자")))
    }

    fn all_syllables() -> impl Iterator<Item = char> {
        (0..SYLLABLE_COUNT).filter_map(|offset| char::from_u32(SYLLABLE_BASE + offset))
    }

    #[test]
    fn composes_plain_syllables() {
        assert_eq!(typed("ㅎㅏㄴㄱㅡㄹ"), "한글");
        assert_eq!(typed("ㄷㅏㄹㅏㅁㅈㅜㅣ"), "다람쥐");
    }

    #[test]
    fn final_moves_to_next_syllable_before_vowel() {
        assert_eq!(typed("ㄱㅏㄴㅏ"), "가나");
        assert_eq!(typed("ㅇㅏㄴㄴㅕㅇ"), "안녕");
    }

    #[test]
    fn compound_final_joins_and_splits() {
        assert_eq!(typed("ㄱㅏㅂㅅ"), "값");
        assert_eq!(typed("ㄱㅏㅂㅅㅣ"), "갑시");
        assert_eq!(typed("ㅎㅏㄱㅅㅐㅇ"), "학생");
        assert_eq!(typed("ㄷㅏㄹㄱㄱㅗㄱㅣ"), "닭고기");
    }

    #[test]
    fn compound_vowel_joins() {
        assert_eq!(typed("ㄱㅗㅏ"), "과");
        assert_eq!(typed("ㅇㅜㅔㄴ"), "웬");
        assert_eq!(typed("ㅇㅡㅣㅅㅏ"), "의사");
        // 두벌식 표준은 ㅘ 뒤의 ㅣ를 ㅙ로 합치지 않는다.
        assert_eq!(typed("ㅇㅗㅏㅣ"), "와ㅣ");
    }

    #[test]
    fn double_consonants_come_from_one_key() {
        assert_eq!(typed("ㅂㅗㄲㄷㅏ"), "볶다");
        assert_eq!(typed("ㅇㅣㅆㅇㅓ"), "있어");
        // 같은 닿소리를 두 번 쳐도 쌍자음으로 합치지 않는다.
        assert_eq!(typed("ㄱㅏㄱㄱㅏ"), "각가");
        // ㄸ·ㅃ·ㅉ은 받침이 될 수 없다.
        assert_eq!(typed("ㅇㅏㄸㅏ"), "아따");
        assert_eq!(typed("ㅇㅏㅃㅏ"), "아빠");
    }

    #[test]
    fn lone_jamo_stay_as_they_are() {
        assert_eq!(typed("ㅋㅋㅋ"), "ㅋㅋㅋ");
        assert_eq!(typed("ㅏㅏ"), "ㅏㅏ");
        assert_eq!(typed("ㅗㅏ"), "ㅘ");
        assert_eq!(typed("ㅏㄱ"), "ㅏㄱ");
        assert_eq!(typed("ㄱㅅㅏ"), "ㄱ사");
    }

    #[test]
    fn preedit_follows_each_keystroke() {
        let mut composer = Composer::new();
        assert_eq!(composer.preedit(), None);
        let mut seen = Vec::new();
        for c in "ㅎㅏㄴ".chars() {
            assert_eq!(composer.push(Jamo::new(c).expect("글쇠 낱자")), None);
            seen.extend(composer.preedit());
        }
        assert_eq!(seen, ['ㅎ', '하', '한']);
    }

    #[test]
    fn backspace_removes_one_keystroke_at_a_time() {
        let mut composer = Composer::new();
        for c in "ㄱㅗㅏㅂㅅ".chars() {
            composer.push(Jamo::new(c).expect("글쇠 낱자"));
        }
        let mut seen = vec![composer.preedit()];
        while composer.backspace() {
            seen.push(composer.preedit());
        }
        let expected = ['괎', '괍', '과', '고', 'ㄱ'].map(Some);
        assert_eq!(seen, [expected.as_slice(), &[None]].concat());
    }

    #[test]
    fn every_syllable_round_trips() {
        for c in all_syllables() {
            let keys = strokes(c).expect("모든 음절은 두벌식으로 칠 수 있다");
            assert_eq!(compose(keys), c.to_string(), "음절 {c}");
        }
    }

    /// 이웃한 두 글자가 서로 영향을 주는 조합은 (앞 글자의 받침) × (뒤 글자의 첫소리·가운뎃소리)
    /// 뿐이므로, 그 조합을 빠짐없이 쳐 본다.
    #[test]
    fn every_neighbouring_pair_round_trips() {
        let firsts: Vec<char> = all_syllables().take(28).collect(); // 가, 각, 갂, … 갛
        let seconds = all_syllables().step_by(28); // 받침 없는 모든 음절
        for second in seconds {
            for &first in &firsts {
                let text = String::from_iter([first, second]);
                let keys = text.chars().filter_map(strokes).flatten();
                assert_eq!(compose(keys), text);
            }
        }
    }

    #[test]
    fn strokes_exist_only_for_what_the_composer_can_produce() {
        for c in 'ㄱ'..='ㅣ' {
            if let Some(keys) = strokes(c) {
                assert_eq!(compose(keys), c.to_string(), "낱자 {c}");
            }
        }
        assert_eq!(strokes('ㄳ'), None);
        assert_eq!(strokes('a'), None);
        assert_eq!(strokes('뷁').map(|keys| keys.len()), Some(5));
    }
}
