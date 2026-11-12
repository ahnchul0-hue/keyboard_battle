//! 친 글을 칠 문장 위에 겹쳐 보여 줄 때, 자리마다 어떤 글자를 어떤 모습으로 보일지 정한다.
//!
//! 한글은 '가나'를 치는 도중에 잠깐 '간'이 된다. 친 글을 그대로 겹치면 그 순간 문장이
//! '간나'처럼 보이므로, 어긋남 없이 치는 동안에는 친 글쇠를 칠 문장의 글자에 차례로 나눠 준다.
//! 그러면 같은 순간이 '가ㄴ'으로 보인다.

use crate::hangul::{Composer, strokes};
use crate::measure::{common_prefix, key_units};
use crate::typed::TypedText;

/// 한 자리의 모습.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// 맞게 친 글자.
    Right,
    /// 틀리게 친 자리. 쳐야 했던 글자를 보여 준다.
    Wrong,
    /// 치고 있는 글자. 지금까지 조합된 모습을 보여 준다.
    Composing,
    /// 아직 치지 않은 글자.
    Todo,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    pub state: State,
}

/// 글자 `c`를 치는 글쇠 가운데 앞의 `keys`개만 쳤을 때의 모습.
fn partly_typed(c: char, keys: usize) -> char {
    let mut composer = Composer::new();
    for jamo in strokes(c).into_iter().flatten().take(keys) {
        composer.push(jamo);
    }
    composer.preedit().unwrap_or(c)
}

fn units(c: char) -> Vec<char> {
    key_units(c.encode_utf8(&mut [0; 4]))
}

/// `typed`가 `target`을 치다 만 모습인지. 다 친 것은 치다 만 것이 아니다.
fn is_start_of(typed: char, target: char) -> bool {
    let (typed, target) = (units(typed), units(target));
    typed.len() < target.len() && target.starts_with(&typed)
}

/// 조합 중인 글자 `typed`가 `here`를 다 치고 `next`를 치기 시작한 모습이면,
/// `next`를 친 데까지의 모습을 돌려준다. 받침이 다음 글자로 넘어가기 전의 모습이 그렇다
/// ('나다'를 치는 중의 '낟'은 '나'와 'ㄷ').
fn carried_into(typed: char, here: char, next: Option<char>) -> Option<char> {
    let (typed, here) = (units(typed), units(here));
    let rest = typed
        .strip_prefix(here.as_slice())
        .filter(|rest| !rest.is_empty())?;
    let next = next.filter(|&next| units(next).starts_with(rest))?;
    Some(partly_typed(next, rest.len()))
}

/// 칠 문장 `target` 위에 친 글 `typed`를 겹친 모습. 칠 문장의 글자마다 한 자리씩이고,
/// 친 글이 문장보다 길면 넘친 글자가 뒤에 붙는다.
pub fn cells(target: &str, typed: &TypedText) -> Vec<Cell> {
    let cell = |ch, state| Cell { ch, state };
    let typed_units = key_units(&typed.text());
    let on_track = common_prefix(&typed_units, &key_units(target)) == typed_units.len();

    if on_track {
        // 친 글쇠를 칠 문장의 글자에 앞에서부터 나눠 준다.
        let mut left = typed_units.len();
        return target
            .chars()
            .map(|c| {
                let needed = strokes(c).map_or(1, |keys| keys.len());
                let used = left.min(needed);
                left -= used;
                match used {
                    0 => cell(c, State::Todo),
                    _ if used == needed => cell(c, State::Right),
                    _ => cell(partly_typed(c, used), State::Composing),
                }
            })
            .collect();
    }

    // 어긋났다. 친 글자를 같은 자리의 글자와 견주고, 틀린 자리에는 쳐야 했던 글자를 보인다.
    let mut expected = target.chars();
    let mut out = Vec::new();
    for c in typed.done().chars() {
        out.push(match expected.next() {
            Some(target) if target == c => cell(c, State::Right),
            Some(target) => cell(target, State::Wrong),
            None => cell(c, State::Wrong),
        });
    }
    if let Some(c) = typed.composing() {
        match expected.next() {
            Some(here) if here == c => out.push(cell(c, State::Right)),
            Some(here) if is_start_of(c, here) => out.push(cell(c, State::Composing)),
            Some(here) => match carried_into(c, here, expected.clone().next()) {
                Some(part) => {
                    expected.next();
                    out.push(cell(here, State::Right));
                    out.push(cell(part, State::Composing));
                }
                None => out.push(cell(here, State::Wrong)),
            },
            None => out.push(cell(c, State::Wrong)),
        }
    }
    out.extend(expected.map(|c| cell(c, State::Todo)));
    out
}

/// 조합 중인 글자가 이미 틀렸는지. 제대로 조합되어 가는 중이면 틀린 것이 아니다.
pub fn composing_is_wrong(target: &str, typed: &TypedText) -> bool {
    let at = typed.done().chars().count();
    typed.composing().is_some()
        && cells(target, typed)
            .get(at)
            .is_some_and(|cell| cell.state == State::Wrong)
}

#[cfg(test)]
mod tests {
    use super::State::{Composing, Right, Todo, Wrong};
    use super::*;
    use crate::keymap::keystroke_for;
    use crate::texts::{self, Kind};
    use crate::typed::Script;

    /// 낱자나 문자를 차례로 쳤을 때의 겹친 모습을 (글자들, 모습들)로 돌려준다.
    fn look(target: &str, keys: &str, script: Script) -> (String, Vec<State>) {
        let mut typed = TypedText::default();
        for unit in keys.chars() {
            let (pos, shift) = keystroke_for(unit).expect("글쇠 하나로 치는 문자");
            typed.press(pos, shift, script);
        }
        let cells = cells(target, &typed);
        let text = cells.iter().map(|cell| cell.ch).collect();
        (text, cells.iter().map(|cell| cell.state).collect())
    }

    fn hangul(target: &str, keys: &str) -> (String, Vec<State>) {
        look(target, keys, Script::Hangul)
    }

    #[test]
    fn nothing_typed_shows_the_sentence_as_it_is() {
        assert_eq!(hangul("가나", ""), ("가나".to_owned(), vec![Todo, Todo]));
    }

    #[test]
    fn a_syllable_in_progress_shows_what_is_composed_so_far() {
        assert_eq!(
            hangul("값이", "ㄱ"),
            ("ㄱ이".to_owned(), vec![Composing, Todo])
        );
        assert_eq!(
            hangul("값이", "ㄱㅏㅂ"),
            ("갑이".to_owned(), vec![Composing, Todo])
        );
        assert_eq!(
            hangul("값이", "ㄱㅏㅂㅅ"),
            ("값이".to_owned(), vec![Right, Todo])
        );
    }

    /// 받침이 다음 글자로 넘어가기 전에도 문장이 '간나'처럼 보이지 않는다.
    #[test]
    fn a_final_waiting_to_move_on_is_shown_at_the_next_syllable() {
        assert_eq!(
            hangul("가나", "ㄱㅏ"),
            ("가나".to_owned(), vec![Right, Todo])
        );
        assert_eq!(
            hangul("가나", "ㄱㅏㄴ"),
            ("가ㄴ".to_owned(), vec![Right, Composing])
        );
        // 겹받침에서 갈라져 넘어가는 낱자도 마찬가지다.
        assert_eq!(
            hangul("학생", "ㅎㅏㄱㅅ"),
            ("학ㅅ".to_owned(), vec![Right, Composing])
        );
    }

    #[test]
    fn a_wrong_position_shows_what_should_have_been_typed() {
        let latin = |keys| look("cat", keys, Script::Latin);
        assert_eq!(latin("ca"), ("cat".to_owned(), vec![Right, Right, Todo]));
        assert_eq!(latin("co"), ("cat".to_owned(), vec![Right, Wrong, Todo]));
        assert_eq!(latin("xat"), ("cat".to_owned(), vec![Wrong, Right, Right]));
        assert_eq!(
            hangul("가나다", "ㄱㅏㄴㅓ"),
            ("가나다".to_owned(), vec![Right, Wrong, Todo])
        );
    }

    /// 앞에서 틀렸더라도, 지금 치는 글자가 제대로 조합되고 있으면 틀렸다고 하지 않는다.
    #[test]
    fn a_syllable_typed_correctly_after_a_mistake_is_not_marked_wrong() {
        assert_eq!(
            hangul("가나다", "ㄱㅓㄴㅏㄷ"),
            ("가나ㄷ".to_owned(), vec![Wrong, Right, Composing])
        );
        assert_eq!(
            hangul("가나다", "ㄱㅓㄴㅏㄷㅏ"),
            ("가나다".to_owned(), vec![Wrong, Right, Right])
        );
    }

    #[test]
    fn a_composing_syllable_is_wrong_as_soon_as_it_leaves_the_sentence() {
        let wrong = |keys: &str| {
            let mut typed = TypedText::default();
            for unit in keys.chars() {
                let (pos, shift) = keystroke_for(unit).expect("글쇠 하나로 치는 문자");
                typed.press(pos, shift, Script::Hangul);
            }
            composing_is_wrong("가나다", &typed)
        };
        assert!(!wrong(""));
        assert!(!wrong("ㄱ") && !wrong("ㄱㅏ") && !wrong("ㄱㅏㄴ"));
        assert!(wrong("ㅈ"), "첫 낱자부터 틀렸다");
        assert!(wrong("ㄱㅏㄴㅓ"), "홀소리를 틀렸다");
        // 앞에서 틀렸어도 지금 글자가 제대로 조합되는 중이면 틀렸다고 하지 않는다.
        assert!(!wrong("ㄱㅓㄴㅏㄷ"));
    }

    #[test]
    fn text_longer_than_the_sentence_is_shown_past_its_end() {
        assert_eq!(
            look("ab", "xbc", Script::Latin),
            ("abc".to_owned(), vec![Wrong, Right, Wrong])
        );
    }

    /// 어긋남 없이 치는 동안에는 글쇠를 하나 칠 때마다 자리 수가 그대로이고,
    /// 치고 있는 자리 말고는 칠 문장의 글자가 그대로 보인다.
    #[test]
    fn typing_any_sentence_keeps_the_sentence_in_view() {
        for sentence in texts::all() {
            let script = sentence.kind.script();
            let length = sentence.text.chars().count();
            let mut typed = TypedText::default();
            for unit in key_units(sentence.text) {
                let (pos, shift) = keystroke_for(unit).expect("칠 수 있는 문자");
                typed.press(pos, shift, script);
                let cells = cells(sentence.text, &typed);
                assert_eq!(cells.len(), length, "{}", sentence.text);
                let composing = cells.iter().filter(|c| c.state == Composing).count();
                assert!(composing <= 1 && cells.iter().all(|c| c.state != Wrong));
                for (cell, target) in cells.iter().zip(sentence.text.chars()) {
                    assert!(cell.state == Composing || cell.ch == target);
                }
                if sentence.kind != Kind::Hangul {
                    assert_eq!(composing, 0, "한글이 아니면 조합 중인 자리가 없다");
                }
            }
            assert!(
                cells(sentence.text, &typed)
                    .iter()
                    .all(|c| c.state == Right)
            );
        }
    }
}
