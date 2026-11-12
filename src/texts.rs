//! 연습 문장.
//!
//! 예부터 전해 오는 속담과 글귀, 그리고 이 프로그램을 위해 새로 쓴 문장과 코드만 싣는다.

use crate::typed::Script;

/// 문장 하나의 가장 긴 길이(칸). 영문·숫자·기호는 한 칸, 한글은 두 칸을 차지한다.
/// 화면은 이 길이의 문장이 한 줄에 들어오도록 글자 크기를 정한다.
pub const MAX_CELLS: usize = 70;

/// 문장의 종류.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Hangul,
    English,
    Rust,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Hangul => "한글",
            Self::English => "영문",
            Self::Rust => "러스트",
        }
    }

    /// 이 종류의 문장을 칠 때 글쇠를 읽는 배열.
    pub fn script(self) -> Script {
        match self {
            Self::Hangul => Script::Hangul,
            Self::English | Self::Rust => Script::Latin,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sentence {
    pub kind: Kind,
    pub text: &'static str,
}

/// 한 과정에 두 문장씩 쓴다. 처음 두 문장은 닿소리 열네 개와 겹낱자·쌍자음을 고루 쳐 보게 한다.
const HANGUL: [&str; 20] = [
    "다람쥐 헌 쳇바퀴에 타고파",
    "얘, 옛 학생은 값싼 떡볶이와 닭꼬치, 짭짤한 빵을 좋아했다.",
    "가는 말이 고와야 오는 말이 곱다.",
    "낮말은 새가 듣고 밤말은 쥐가 듣는다.",
    "세 살 버릇 여든까지 간다.",
    "천 리 길도 한 걸음부터.",
    "백지장도 맞들면 낫다.",
    "고래 싸움에 새우 등 터진다.",
    "소 잃고 외양간 고친다.",
    "원숭이도 나무에서 떨어진다.",
    "하늘이 무너져도 솟아날 구멍이 있다.",
    "콩 심은 데 콩 나고 팥 심은 데 팥 난다.",
    "돌다리도 두들겨 보고 건너라.",
    "발 없는 말이 천 리 간다.",
    "윗물이 맑아야 아랫물이 맑다.",
    "구슬이 서 말이라도 꿰어야 보배.",
    "말 한마디에 천 냥 빚도 갚는다.",
    "서당 개 삼 년에 풍월을 읊는다.",
    "아니 땐 굴뚝에 연기 날까.",
    "티끌 모아 태산.",
];

const ENGLISH: [&str; 10] = [
    "The quick brown fox jumps over the lazy dog.",
    "A journey of a thousand miles begins with a single step.",
    "Actions speak louder than words.",
    "Pack my box with five dozen liquor jugs.",
    "Slow and steady wins the race.",
    "Well begun is half done.",
    "How vexingly quick daft zebras jump!",
    "Measure twice and cut once.",
    "Every expert was once a beginner.",
    "Practice makes progress, not perfection.",
];

const RUST: [&str; 10] = [
    "let v: Vec<u8> = vec![1, 2]; println!(\"{:?}\", &v);",
    "fn add(a: i32, b: i32) -> i32 { a + b }",
    "let name = String::from(\"ferris\");",
    "match value { Some(n) if n > 0 => n * 2, _ => 0 }",
    "let total: u64 = nums.iter().map(|n| n * n).sum();",
    "fn peek(&self) -> Option<&T> { self.items.last() }",
    "#[derive(Debug, Clone, PartialEq)] struct Point { x: f64, y: f64 }",
    "let text = std::fs::read_to_string(path)?;",
    "if let Err(e) = run() { eprintln!(\"error: {e}\"); }",
    "for (i, x) in items.iter().enumerate() { println!(\"{i}: {x}\"); }",
];

/// `number`번째 과정(0부터)의 문장들: 한글 둘, 영문 하나, 러스트 하나.
///
/// 과정마다 새 문장이 나오고, 모두 쓰고 나면 처음으로 돌아간다.
pub fn round(number: usize) -> Vec<Sentence> {
    let pick = |kind, pool: &[&'static str], index: usize| Sentence {
        kind,
        text: pool[index % pool.len()],
    };
    vec![
        pick(Kind::Hangul, &HANGUL, 2 * number),
        pick(Kind::Hangul, &HANGUL, 2 * number + 1),
        pick(Kind::English, &ENGLISH, number),
        pick(Kind::Rust, &RUST, number),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap::keystroke_for;
    use crate::measure::key_units;
    use crate::typed::TypedText;

    /// 한 바퀴를 도는 데 걸리는 과정 수.
    const CYCLE: usize = 10;

    fn all_sentences() -> Vec<Sentence> {
        (0..CYCLE).flat_map(round).collect()
    }

    /// 실린 문장은 모두 이 프로그램의 자판으로 끝까지 칠 수 있어야 한다.
    #[test]
    fn every_sentence_can_be_typed() {
        for sentence in all_sentences() {
            let mut typed = TypedText::default();
            for unit in key_units(sentence.text) {
                let (pos, shift) = keystroke_for(unit)
                    .unwrap_or_else(|| panic!("칠 수 없는 문자 {unit:?}: {}", sentence.text));
                typed.press(pos, shift, sentence.kind.script());
            }
            assert_eq!(typed.text(), sentence.text);
        }
    }

    #[test]
    fn rounds_bring_new_sentences_until_the_pool_runs_out() {
        let mut texts: Vec<&str> = all_sentences().iter().map(|s| s.text).collect();
        assert_eq!(texts.len(), HANGUL.len() + ENGLISH.len() + RUST.len());
        texts.sort_unstable();
        texts.dedup();
        assert_eq!(texts.len(), 4 * CYCLE, "같은 문장이 두 번 실려 있다");
        assert_eq!(round(CYCLE), round(0));
    }

    #[test]
    fn a_round_is_two_hangul_one_english_one_rust() {
        let kinds: Vec<Kind> = round(3).iter().map(|s| s.kind).collect();
        assert_eq!(
            kinds,
            [Kind::Hangul, Kind::Hangul, Kind::English, Kind::Rust]
        );
    }

    #[test]
    fn sentences_are_not_too_long() {
        for sentence in all_sentences() {
            let cells: usize = sentence
                .text
                .chars()
                .map(|c| if c.is_ascii() { 1 } else { 2 })
                .sum();
            assert!(cells <= MAX_CELLS, "{} ({cells}칸)", sentence.text);
        }
    }
}
