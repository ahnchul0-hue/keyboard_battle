//! 연습 문장.
//!
//! 예부터 전해 오는 속담과 글귀, 그리고 이 프로그램을 위해 새로 쓴 문장과 코드만 싣는다.

use crate::measure::key_units;
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
    /// 모든 종류. 연습에서 돌아가며 나오는 순서이자, 기록에 적는 순서다.
    pub const ALL: [Self; 3] = [Self::Hangul, Self::English, Self::Rust];

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

/// 한 번에 이어서 치는 글. 속담 하나일 수도 있고, 여러 문장을 이은 문단일 수도 있다.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sentence {
    pub kind: Kind,
    pub text: String,
}

impl Sentence {
    pub fn new(kind: Kind, text: impl Into<String>) -> Self {
        let text = text.into();
        Self { kind, text }
    }
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

const ENGLISH: [&str; 50] = [
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
    "Birds of a feather flock together.",
    "A watched pot never boils.",
    "Do not count your chickens before they hatch.",
    "Every cloud has a silver lining.",
    "All that glitters is not gold.",
    "Practice what you preach.",
    "The squeaky wheel gets the grease.",
    "Do not put all your eggs in one basket.",
    "A picture is worth a thousand words.",
    "Necessity is the mother of invention.",
    "Out of sight, out of mind.",
    "Easy come, easy go.",
    "Great minds think alike.",
    "A penny saved is a penny earned.",
    "The best things in life are free.",
    "Jackdaws love my big sphinx of quartz.",
    "Waltz, bad nymph, for quick jigs vex.",
    "Curiosity killed the cat.",
    "Knowledge is power.",
    "Better safe than sorry.",
];

const RUST: [&str; 50] = [
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
    "let mut buf = Vec::with_capacity(1024);",
    "impl From<u8> for Level { fn from(n: u8) -> Self { Level(n) } }",
    "let words: Vec<&str> = line.split_whitespace().collect();",
    "if count % 2 == 0 && !done { continue; }",
    "let (left, right) = slice.split_at(mid);",
    "struct Stack<T> { items: Vec<T>, limit: usize }",
    "fn area(&self) -> f64 { self.w * self.h }",
    "let found = list.iter().position(|&x| x == key);",
    "pub trait Shape { fn area(&self) -> f64; }",
    "let label = if ok { \"pass\" } else { \"fail\" };",
    "let sum: i32 = (1..=100).sum();",
    "match rx.recv() { Ok(msg) => handle(msg), Err(_) => break }",
    "let upper = name.to_uppercase();",
    "let max = *scores.iter().max().unwrap_or(&0);",
    "type Result<T> = std::result::Result<T, Error>;",
    "let pairs: Vec<(usize, char)> = text.char_indices().collect();",
    "fn is_even(n: u32) -> bool { n % 2 == 0 }",
    "let path = dir.join(\"notes\").with_extension(\"txt\");",
    "self.count = self.count.saturating_add(1);",
    "let id: u32 = args.get(1)?.parse().ok()?;",
];

/// 쪽의 종류가 돌아가는 차례. 한 쪽은 한 종류의 글로만 채운다.
/// 문장마다 한글과 영문을 오가면 그때마다 손과 머리가 자판을 바꿔 타야 해서 흐름이 끊긴다.
const ROTATION: [Kind; 3] = Kind::ALL;

/// 한 쪽에 이어서 치는 글의 길이(글쇠 수). 1분에 350타로 45초쯤 걸린다.
/// 문장을 하나씩 내면 몇 초마다 끊기므로, 이만큼이 찰 때까지 문장을 잇는다.
pub const PAGE_KEYS: usize = 260;

/// 실린 문장 모두.
pub fn all() -> impl Iterator<Item = Sentence> {
    ROTATION.into_iter().flat_map(|kind| {
        let sentence = move |&text| Sentence::new(kind, text);
        kind.pool().iter().map(sentence)
    })
}

/// 늘 같은 처음 두 쪽. 둘러보기처럼 정해진 글이 필요할 때 쓴다.
pub fn opening() -> [Vec<Sentence>; 2] {
    let mut deck = Deck::in_order();
    [deck.page(PAGE_KEYS), deck.page(PAGE_KEYS)]
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
    /// 바로 앞 쪽과 지금 채우는 쪽에 낸 문장들. 다시 섞을 때 이들이 곧바로 또 나오지 않게 한다.
    recent: Vec<usize>,
}

impl Bag {
    fn new(kind: Kind) -> Self {
        Self {
            kind,
            order: (0..kind.pool().len()).collect(),
            next: 0,
            recent: Vec::new(),
        }
    }

    /// 다음 문장. 한 바퀴를 다 돌았으면 `rng`로 다시 섞되, 방금 낸 문장들은 뒤로 돌린다.
    fn draw(&mut self, rng: Option<&mut Rng>) -> &'static str {
        if self.next == self.order.len() {
            self.next = 0;
            if let Some(rng) = rng {
                rng.shuffle(&mut self.order);
                let recent = &self.recent;
                self.order.sort_by_key(|item| recent.contains(item));
            }
        }
        let item = self.order[self.next];
        self.next += 1;
        self.recent.push(item);
        self.kind.pool()[item]
    }

    /// 쪽 하나를 다 채웠음을 알린다. 그 쪽에 낸 `drawn`개만 방금 낸 것으로 남긴다.
    fn turn_page(&mut self, drawn: usize) {
        let older = self.recent.len().saturating_sub(drawn);
        self.recent.drain(..older);
    }
}

/// 쪽마다 칠 글을 골라 주는 곳.
///
/// 종류마다 문장을 섞어 두고 차례로 내므로, 실린 문장을 다 칠 때까지 같은 문장이 다시 나오지 않는다.
#[derive(Clone, Debug)]
pub struct Deck {
    bags: [Bag; 3],
    /// 섞는 데 쓰는 난수. 없으면 실린 순서대로 낸다.
    rng: Option<Rng>,
    /// 지금까지 낸 쪽 수. 다음 쪽의 종류를 정한다.
    dealt: usize,
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

    /// 섞지 않고 실린 순서대로 내는 묶음. 시연이나 확인처럼 늘 같은 글이 필요할 때 쓴다.
    pub fn in_order() -> Self {
        Self {
            bags: ROTATION.map(Bag::new),
            rng: None,
            dealt: 0,
        }
    }

    /// 쪽의 종류가 다시 처음(한글)부터 돌아가게 한다. 연습을 새로 시작할 때 부른다.
    /// 연습마다 글의 차례가 같아야 기록을 서로 견줄 수 있다. 문장은 앞에서 낸 데서 이어서 나온다.
    pub fn restart(&mut self) {
        self.dealt = 0;
    }

    /// 다음 쪽에 칠 글. 치는 글쇠가 `length`개를 채울 때까지 문장을 잇는다.
    /// 한글과 영문은 문장들을 한 문단으로 이은 글 하나이고, 러스트는 줄마다 따로 치는 코드 몇 줄이다.
    pub fn page(&mut self, length: usize) -> Vec<Sentence> {
        let turn = self.dealt % ROTATION.len();
        self.dealt += 1;
        let (kind, bag) = (ROTATION[turn], &mut self.bags[turn]);
        let mut texts = Vec::new();
        let mut keys = 0;
        while keys < length {
            let text = bag.draw(self.rng.as_mut());
            // 문장 사이에 치는 빈칸까지 센다.
            keys += key_units(text).len() + 1;
            texts.push(text);
        }
        bag.turn_page(texts.len());
        if kind == Kind::Rust {
            let line = |text| Sentence::new(kind, text);
            texts.into_iter().map(line).collect()
        } else {
            vec![Sentence::new(kind, texts.join(" "))]
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::keymap::keystroke_for;
    use crate::typed::TypedText;

    /// 쪽에 든 문장들. 문단으로 이은 글은 다시 문장으로 가른다.
    fn sentences(page: &[Sentence]) -> Vec<&'static str> {
        let kind = page[0].kind;
        let joined: Vec<&str> = page.iter().map(|line| line.text.as_str()).collect();
        let joined = joined.join(" ");
        // 긴 문장부터 찾아야, 다른 문장의 앞머리인 짧은 문장을 잘못 집지 않는다.
        let mut found: Vec<(usize, &'static str)> = kind
            .pool()
            .iter()
            .filter_map(|&text| joined.find(text).map(|at| (at, text)))
            .collect();
        found.sort_unstable();
        found.into_iter().map(|(_, text)| text).collect()
    }

    /// 실린 문장은 모두 이 프로그램의 자판으로 끝까지 칠 수 있어야 한다.
    #[test]
    fn every_sentence_can_be_typed() {
        for sentence in all() {
            let mut typed = TypedText::default();
            for unit in key_units(&sentence.text) {
                let (pos, shift) = keystroke_for(unit)
                    .unwrap_or_else(|| panic!("칠 수 없는 문자 {unit:?}: {}", sentence.text));
                typed.press(pos, shift, sentence.kind.script());
            }
            assert_eq!(typed.text(), sentence.text);
        }
    }

    #[test]
    fn no_sentence_is_listed_twice() {
        let listed: Vec<String> = all().map(|sentence| sentence.text).collect();
        assert_eq!(listed.len(), HANGUL.len() + ENGLISH.len() + RUST.len());
        let distinct: BTreeSet<&String> = listed.iter().collect();
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

    /// 쪽은 한글, 영문, 러스트의 차례로 돌아가고, 한 쪽은 한 종류로만 채운다.
    #[test]
    fn pages_take_turns_by_kind() {
        for mut deck in [Deck::in_order(), Deck::shuffled(7)] {
            for turn in 0..9 {
                let page = deck.page(PAGE_KEYS);
                let kind = ROTATION[turn % 3];
                assert!(page.iter().all(|line| line.kind == kind));
                // 한글과 영문은 문단 하나로 잇고, 러스트는 줄마다 따로 친다.
                assert_eq!(page.len() == 1, kind != Kind::Rust, "{kind:?}");
            }
        }
    }

    /// 차례를 처음으로 돌리면 다시 한글부터 나오되, 이미 낸 문장은 다시 내지 않는다.
    #[test]
    fn restarting_the_turns_begins_with_hangul_again_but_with_new_sentences() {
        let mut deck = Deck::in_order();
        let first = deck.page(PAGE_KEYS);
        let english = deck.page(PAGE_KEYS);
        assert_eq!(
            (first[0].kind, english[0].kind),
            (Kind::Hangul, Kind::English)
        );
        deck.restart();
        let again = deck.page(PAGE_KEYS);
        assert_eq!(again[0].kind, Kind::Hangul);
        let seen = sentences(&first);
        assert!(sentences(&again).iter().all(|text| !seen.contains(text)));
        // 그다음은 영문이고, 앞의 영문 쪽이 끝난 데서 이어진다.
        let next = deck.page(PAGE_KEYS);
        assert_eq!(next[0].kind, Kind::English);
        assert!(next[0].text.starts_with(ENGLISH[sentences(&english).len()]));
    }

    /// 한 쪽은 정한 길이가 찰 때까지 문장을 잇는다. 마지막 문장만큼만 넘친다.
    #[test]
    fn a_page_is_long_enough_to_type_without_a_break() {
        let mut deck = Deck::shuffled(3);
        for _ in 0..30 {
            let page = deck.page(PAGE_KEYS);
            let keys: usize = page.iter().map(|line| key_units(&line.text).len()).sum();
            assert!(keys >= PAGE_KEYS - sentences(&page).len(), "{keys}타");
            assert!(keys < PAGE_KEYS + 80, "{keys}타");
            assert!(sentences(&page).len() >= 3);
        }
    }

    /// 섞지 않으면 실린 순서대로 나오고, 다 내고 나면 처음으로 돌아간다.
    #[test]
    fn an_unshuffled_deck_deals_in_the_listed_order() {
        let mut deck = Deck::in_order();
        let first = deck.page(PAGE_KEYS);
        let listed = sentences(&first);
        assert_eq!(listed, HANGUL[..listed.len()]);
        assert!(
            first[0]
                .text
                .starts_with("다람쥐 헌 쳇바퀴에 타고파 얘, 옛"),
            "둘러보기와 시연이 기대하는 첫 글"
        );
        let english = deck.page(PAGE_KEYS);
        assert_eq!([first, english.clone()], opening());
        assert!(english[0].text.starts_with(ENGLISH[0]));
        let rust: Vec<String> = deck
            .page(PAGE_KEYS)
            .into_iter()
            .map(|line| line.text)
            .collect();
        assert_eq!(rust, RUST[..rust.len()]);
        // 다음 한글 쪽은 앞 쪽이 끝난 데서 이어진다.
        assert!(
            deck.page(PAGE_KEYS)[0]
                .text
                .starts_with(HANGUL[listed.len()])
        );
    }

    /// 섞은 묶음은 한 바퀴 동안 실린 문장을 한 번씩만 낸다.
    #[test]
    fn a_shuffled_deck_does_not_repeat_until_the_pool_runs_out() {
        let mut deck = Deck::shuffled(2026);
        let mut seen: Vec<Vec<&str>> = vec![Vec::new(); 3];
        for turn in 0..12 {
            seen[turn % 3].extend(sentences(&deck.page(PAGE_KEYS)));
        }
        for (kind, seen) in ROTATION.iter().zip(&seen) {
            let pool = kind.pool().len();
            let first_cycle = &seen[..seen.len().min(pool)];
            let distinct: BTreeSet<&&str> = first_cycle.iter().collect();
            assert_eq!(distinct.len(), first_cycle.len(), "{kind:?}");
        }
    }

    #[test]
    fn the_same_seed_deals_the_same_pages() {
        let (mut a, mut b) = (Deck::shuffled(42), Deck::shuffled(42));
        for _ in 0..40 {
            assert_eq!(a.page(PAGE_KEYS), b.page(PAGE_KEYS));
        }
    }

    /// 켤 때마다 씨앗이 다르면, 처음 보는 글도 그때마다 다르다.
    #[test]
    fn different_seeds_start_with_different_pages() {
        let firsts: BTreeSet<String> = (0..40)
            .map(|seed| Deck::shuffled(seed).page(PAGE_KEYS)[0].text.clone())
            .collect();
        assert_eq!(firsts.len(), 40);
    }

    /// 한 바퀴를 다 돌고 다시 섞을 때, 방금 친 문장이 같은 쪽이나 바로 다음 쪽에 또 나오지 않는다.
    #[test]
    fn a_new_cycle_does_not_repeat_what_was_just_typed() {
        for seed in 0..30 {
            let mut deck = Deck::shuffled(seed);
            let mut previous: Vec<Vec<&str>> = vec![Vec::new(); 3];
            for turn in 0..60 {
                let page = sentences(&deck.page(PAGE_KEYS));
                let distinct: BTreeSet<&&str> = page.iter().collect();
                assert_eq!(
                    distinct.len(),
                    page.len(),
                    "씨앗 {seed}: 한 쪽 안에서 겹친다"
                );
                for text in &page {
                    assert!(!previous[turn % 3].contains(text), "씨앗 {seed}: {text}");
                }
                previous[turn % 3] = page;
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
