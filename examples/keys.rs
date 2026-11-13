//! 섞지 않은 묶음(`--in-order`)의 앞쪽 글을 치려면 쿼티 자판에서 어떤 글쇠를 누르는지 적어 준다.
//! 글 하나에 한 줄씩이다. 시연을 찍는 스크립트가 이것을 받아 글쇠를 보낸다.
//!
//! 사용법: cargo run --example keys [쪽 수] [한 쪽의 글쇠 수]

use keyboard_battle::keymap::{keystroke_for, qwerty};
use keyboard_battle::measure::key_units;
use keyboard_battle::texts::{Deck, PAGE_KEYS};

fn main() {
    let number = |nth| std::env::args().nth(nth).and_then(|text| text.parse().ok());
    let (pages, length) = (number(1).unwrap_or(3), number(2).unwrap_or(PAGE_KEYS));
    let mut deck = Deck::in_order();
    for _ in 0..pages {
        for line in deck.page(length) {
            let key = |unit| {
                let (pos, shift) = keystroke_for(unit).expect("실린 글은 모두 칠 수 있다");
                qwerty(pos, shift)
            };
            let keys: String = key_units(&line.text).into_iter().map(key).collect();
            println!("{keys}");
        }
    }
}
