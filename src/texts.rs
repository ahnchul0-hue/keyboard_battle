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

    /// 실린 문장들.
    fn pool(self) -> &'static [&'static str] {
        match self {
            Self::Hangul => &HANGUL,
            Self::English => &ENGLISH,
            Self::Rust => &RUST,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sentence {
    pub kind: Kind,
    pub text: &'static str,
}

/// 처음 두 문장은 닿소리 열네 개와 겹낱자·쌍자음을 고루 쳐 보게 한다.
/// 둘러보기와 시연은 이 두 문장으로 시작한다.
const HANGUL: [&str; 60] = [
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
    "가랑비에 옷 젖는 줄 모른다.",
    "등잔 밑이 어둡다.",
    "벼는 익을수록 고개를 숙인다.",
    "호랑이도 제 말 하면 온다.",
    "우물을 파도 한 우물을 파라.",
    "꿩 먹고 알 먹는다.",
    "누워서 떡 먹기.",
    "뛰는 놈 위에 나는 놈 있다.",
    "급할수록 돌아가라.",
    "시작이 반이다.",
    "바늘 도둑이 소도둑 된다.",
    "믿는 도끼에 발등 찍힌다.",
    "개구리 올챙이 적 생각 못 한다.",
    "고생 끝에 낙이 온다.",
    "공든 탑이 무너지랴.",
    "금강산도 식후경.",
    "까마귀 날자 배 떨어진다.",
    "낫 놓고 기역 자도 모른다.",
    "될성부른 나무는 떡잎부터 알아본다.",
    "사공이 많으면 배가 산으로 간다.",
    "열 번 찍어 안 넘어가는 나무 없다.",
    "지렁이도 밟으면 꿈틀한다.",
    "호랑이 굴에 가야 호랑이 새끼를 잡는다.",
    "빈 수레가 요란하다.",
    "작은 고추가 더 맵다.",
    "쇠뿔도 단김에 빼라.",
    "닭 쫓던 개 지붕 쳐다본다.",
    "물이 깊어야 고기가 모인다.",
    "배보다 배꼽이 더 크다.",
    "종로에서 뺨 맞고 한강에서 눈 흘긴다.",
    "팔이 안으로 굽는다.",
    "하나를 보면 열을 안다.",
    "가지 많은 나무에 바람 잘 날 없다.",
    "굴러온 돌이 박힌 돌 뺀다.",
    "달면 삼키고 쓰면 뱉는다.",
    "마른하늘에 날벼락.",
    "무소식이 희소식.",
    "비 온 뒤에 땅이 굳어진다.",
    "산 넘어 산이다.",
    "옷깃만 스쳐도 인연이다.",
];

const ENGLISH: [&str; 30] = [
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
    "A stitch in time saves nine.",
    "The early bird catches the worm.",
    "Better late than never.",
    "Look before you leap.",
    "Many hands make light work.",
    "Rome was not built in a day.",
    "Where there is a will, there is a way.",
    "Two heads are better than one.",
    "A rolling stone gathers no moss.",
    "Strike while the iron is hot.",
    "The pen is mightier than the sword.",
    "Haste makes waste.",
    "Honesty is the best policy.",
    "Sphinx of black quartz, judge my vow.",
    "The five boxing wizards jump quickly.",
    "Still waters run deep.",
    "No pain, no gain.",
    "Time and tide wait for no man.",
    "Fortune favors the bold.",
    "When in doubt, leave it out.",
];

const RUST: [&str; 30] = [
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
    "impl Default for Config { fn default() -> Self { Self::new() } }",
    "let mut seen: HashMap<String, usize> = HashMap::new();",
    "while let Some(top) = stack.pop() { visit(top); }",
    "let evens: Vec<_> = (1..=10).filter(|n| n % 2 == 0).collect();",
    "pub fn len(&self) -> usize { self.items.len() }",
    "let Some(first) = words.first() else { return None; };",
    "enum Shape { Circle(f64), Rect { w: f64, h: f64 } }",
    "let names = users.iter().map(|u| u.name.trim()).collect::<Vec<_>>();",
    "use std::collections::{BTreeMap, HashSet};",
    "assert_eq!(parse(\"42\"), Ok(42), \"plain number\");",
    "let area = |w: f64, h: f64| -> f64 { w * h };",
    "impl<T> Stack<T> { fn is_empty(&self) -> bool { self.len == 0 } }",
    "let tail = &bytes[start..end.min(bytes.len())];",
    "const MAX_RETRIES: u32 = 3;",
    "let n: i64 = line.trim().parse().unwrap_or(-1);",
    "*counts.entry(word).or_insert(0) += 1;",
    "let flipped: Vec<u8> = data.iter().map(|b| b ^ 0xff).collect();",
    "#[test] fn it_adds() { assert_eq!(add(2, 2), 4); }",
    "loop { if tries >= 3 { break; } tries += 1; }",
    "let key = format!(\"{}/{}\", user.id, path.display());",
];

/// 과정 하나에 내는 문장: 종류와 그 수. 이 순서대로 친다.
const HAND: [(Kind, usize); 3] = [(Kind::Hangul, 2), (Kind::English, 1), (Kind::Rust, 1)];

/// 실린 문장 모두.
pub fn all() -> impl Iterator<Item = Sentence> {
    HAND.into_iter().flat_map(|(kind, _)| {
        let sentence = move |&text| Sentence { kind, text };
        kind.pool().iter().map(sentence)
    })
}

/// 늘 같은 첫 과정. 둘러보기처럼 정해진 문장이 필요할 때 쓴다.
pub fn opening() -> Vec<Sentence> {
    Deck::in_order().deal()
}

/// 문장을 섞는 데 쓰는 작은 난수 발생기 (SplitMix64).
/// 같은 씨앗에서는 같은 순서가 나오므로 테스트할 수 있다.
#[derive(Clone, Debug)]
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// 0 이상 `bound` 미만의 수.
    fn below(&mut self, bound: usize) -> usize {
        ((u128::from(self.next()) * bound as u128) >> 64) as usize
    }

    /// 순서를 고르게 섞는다 (피셔-예이츠).
    fn shuffle<T>(&mut self, items: &mut [T]) {
        for last in (1..items.len()).rev() {
            items.swap(last, self.below(last + 1));
        }
    }
}

/// 한 종류의 문장 묶음. 한 바퀴를 다 돌 때까지 같은 문장을 다시 내지 않는다.
#[derive(Clone, Debug)]
struct Bag {
    kind: Kind,
    /// 문장을 낼 순서. 값은 그 종류의 문장 번호다.
    order: Vec<usize>,
    /// 다음에 낼 차례.
    next: usize,
}

impl Bag {
    fn new(kind: Kind) -> Self {
        Self {
            kind,
            order: (0..kind.pool().len()).collect(),
            next: 0,
        }
    }

    /// 다음 문장. 한 바퀴를 다 돌았으면 `rng`로 다시 섞되,
    /// 방금 낸 `recent`개가 새 바퀴의 맨 앞에 또 나오지는 않게 한다.
    fn draw(&mut self, rng: Option<&mut Rng>, recent: usize) -> Sentence {
        if self.next == self.order.len() {
            self.next = 0;
            if let Some(rng) = rng {
                let fresh = self.order.len().saturating_sub(recent);
                let just_dealt = self.order[fresh..].to_vec();
                rng.shuffle(&mut self.order);
                for front in 0..recent.min(fresh) {
                    let is_new = |&at: &usize| !just_dealt.contains(&self.order[at]);
                    if !is_new(&front)
                        && let Some(other) = (recent..self.order.len()).find(is_new)
                    {
                        self.order.swap(front, other);
                    }
                }
            }
        }
        let text = self.kind.pool()[self.order[self.next]];
        self.next += 1;
        Sentence {
            kind: self.kind,
            text,
        }
    }
}

/// 과정마다 문장을 골라 주는 곳.
///
/// 종류마다 문장을 섞어 두고 차례로 내므로, 실린 문장을 다 칠 때까지 같은 문장이 다시 나오지 않는다.
#[derive(Clone, Debug)]
pub struct Deck {
    bags: [Bag; 3],
    /// 섞는 데 쓰는 난수. 없으면 실린 순서대로 낸다.
    rng: Option<Rng>,
}

impl Deck {
    /// `seed`로 섞은 묶음. 씨앗이 다르면 나오는 순서가 다르다.
    pub fn shuffled(seed: u64) -> Self {
        let mut deck = Self::in_order();
        let mut rng = Rng(seed);
        for bag in &mut deck.bags {
            rng.shuffle(&mut bag.order);
        }
        deck.rng = Some(rng);
        deck
    }

    /// 섞지 않고 실린 순서대로 내는 묶음. 시연이나 확인처럼 늘 같은 문장이 필요할 때 쓴다.
    pub fn in_order() -> Self {
        Self {
            bags: HAND.map(|(kind, _)| Bag::new(kind)),
            rng: None,
        }
    }

    /// 다음 과정의 문장들: 한글 둘, 영문 하나, 러스트 하나.
    pub fn deal(&mut self) -> Vec<Sentence> {
        let mut lines = Vec::new();
        for (bag, (_, count)) in self.bags.iter_mut().zip(HAND) {
            for _ in 0..count {
                lines.push(bag.draw(self.rng.as_mut(), count));
            }
        }
        lines
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::keymap::keystroke_for;
    use crate::measure::key_units;
    use crate::typed::TypedText;

    /// 실린 문장을 한 번씩 다 치는 데 걸리는 과정 수.
    const CYCLE: usize = 30;

    fn texts(lines: &[Sentence]) -> Vec<&'static str> {
        lines.iter().map(|line| line.text).collect()
    }

    /// 실린 문장은 모두 이 프로그램의 자판으로 끝까지 칠 수 있어야 한다.
    #[test]
    fn every_sentence_can_be_typed() {
        for sentence in all() {
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
    fn no_sentence_is_listed_twice() {
        let listed: Vec<&str> = all().map(|sentence| sentence.text).collect();
        assert_eq!(listed.len(), 4 * CYCLE);
        let distinct: BTreeSet<&str> = listed.iter().copied().collect();
        assert_eq!(distinct.len(), listed.len(), "같은 문장이 두 번 실려 있다");
    }

    #[test]
    fn sentences_are_not_too_long() {
        for sentence in all() {
            let cells: usize = sentence
                .text
                .chars()
                .map(|c| if c.is_ascii() { 1 } else { 2 })
                .sum();
            assert!(cells <= MAX_CELLS, "{} ({cells}칸)", sentence.text);
        }
    }

    #[test]
    fn a_round_is_two_hangul_one_english_one_rust() {
        let kinds = |lines: Vec<Sentence>| lines.iter().map(|s| s.kind).collect::<Vec<_>>();
        let expected = [Kind::Hangul, Kind::Hangul, Kind::English, Kind::Rust];
        assert_eq!(kinds(Deck::in_order().deal()), expected);
        assert_eq!(kinds(Deck::shuffled(7).deal()), expected);
    }

    /// 섞지 않으면 실린 순서대로 나오고, 다 내고 나면 처음으로 돌아간다.
    #[test]
    fn an_unshuffled_deck_deals_in_the_listed_order() {
        let mut deck = Deck::in_order();
        let first = deck.deal();
        assert_eq!(
            texts(&first),
            [HANGUL[0], HANGUL[1], ENGLISH[0], RUST[0]],
            "둘러보기와 시연이 기대하는 첫 과정"
        );
        assert_eq!(first, opening());
        assert_eq!(
            texts(&deck.deal()),
            [HANGUL[2], HANGUL[3], ENGLISH[1], RUST[1]]
        );
        for _ in 2..CYCLE {
            deck.deal();
        }
        assert_eq!(deck.deal(), first);
    }

    /// 섞은 묶음은 한 바퀴 동안 실린 문장을 빠짐없이 한 번씩 낸다.
    #[test]
    fn a_shuffled_deck_deals_every_sentence_once_per_cycle() {
        let mut deck = Deck::shuffled(2026);
        for _ in 0..3 {
            let dealt: Vec<&str> = (0..CYCLE).flat_map(|_| texts(&deck.deal())).collect();
            let distinct: BTreeSet<&str> = dealt.iter().copied().collect();
            assert_eq!((dealt.len(), distinct.len()), (4 * CYCLE, 4 * CYCLE));
        }
    }

    #[test]
    fn the_same_seed_deals_the_same_rounds() {
        let (mut a, mut b) = (Deck::shuffled(42), Deck::shuffled(42));
        for _ in 0..2 * CYCLE + 3 {
            assert_eq!(a.deal(), b.deal());
        }
    }

    /// 켤 때마다 씨앗이 다르면, 처음 보는 문장도 그때마다 다르다.
    #[test]
    fn different_seeds_start_with_different_sentences() {
        let firsts: BTreeSet<&str> = (0..40)
            .map(|seed| Deck::shuffled(seed).deal()[0].text)
            .collect();
        assert!(firsts.len() >= 20, "40번 켜서 {}가지뿐이다", firsts.len());
        // 어느 한 문장으로 쏠리지도 않는다.
        let rounds: BTreeSet<Vec<&str>> = (0..40)
            .map(|seed| texts(&Deck::shuffled(seed).deal()))
            .collect();
        assert_eq!(rounds.len(), 40);
    }

    /// 한 바퀴를 다 돌고 다시 섞을 때, 방금 친 문장이 바로 다음 과정에 또 나오지 않는다.
    #[test]
    fn a_new_cycle_does_not_repeat_the_round_just_typed() {
        for seed in 0..50 {
            let mut deck = Deck::shuffled(seed);
            let mut previous = deck.deal();
            for _ in 0..3 * CYCLE {
                let next = deck.deal();
                for line in &next {
                    assert!(!previous.contains(line), "씨앗 {seed}: {}", line.text);
                }
                previous = next;
            }
        }
    }

    #[test]
    fn shuffling_keeps_every_item_and_spreads_them_evenly() {
        let mut rng = Rng(1);
        // 넷을 섞는 스물네 가지 순서가 모두 나오고, 어느 것도 유난히 잦지 않다.
        let mut seen = std::collections::BTreeMap::new();
        for _ in 0..24_000 {
            let mut items = [0, 1, 2, 3];
            rng.shuffle(&mut items);
            *seen.entry(items).or_insert(0) += 1;
        }
        assert_eq!(seen.len(), 24);
        for (order, count) in seen {
            assert!((850..1150).contains(&count), "{order:?}: {count}번");
        }
    }
}
