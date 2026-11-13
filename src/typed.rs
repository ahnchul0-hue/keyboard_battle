//! 지금까지 친 글.

use crate::hangul::Composer;
use crate::keymap::{KeyPos, dubeolsik, qwerty};

/// 글쇠를 어느 배열로 읽을지. 한/영 키가 아니라 칠 문장이 정한다.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Script {
    Hangul,
    Latin,
}

/// 친 글. 조합이 끝난 부분과 조합 중인 한 글자로 이루어진다.
#[derive(Clone, Debug, Default)]
pub struct TypedText {
    done: String,
    composer: Composer,
}

impl TypedText {
    /// 문자 글쇠 하나를 누른다.
    pub fn press(&mut self, pos: KeyPos, shift: bool, script: Script) {
        let jamo = match script {
            Script::Hangul => dubeolsik(pos, shift),
            Script::Latin => None,
        };
        match jamo {
            Some(jamo) => self.done.extend(self.composer.push(jamo)),
            None => {
                self.done.extend(self.composer.flush());
                self.done.push(qwerty(pos, shift));
            }
        }
    }

    /// 조합 중이면 낱자 하나를, 아니면 글자 하나를 지운다.
    pub fn backspace(&mut self) {
        if !self.composer.backspace() {
            self.done.pop();
        }
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// 조합이 끝난 글의 앞 `chars`글자만 남긴다. 그 뒤의 글자와 조합 중인 글자는 지운다.
    pub fn keep(&mut self, chars: usize) {
        let end = self.done.char_indices().nth(chars);
        self.done
            .truncate(end.map_or(self.done.len(), |(at, _)| at));
        self.composer = Composer::default();
    }

    /// 조합이 끝난 글.
    pub fn done(&self) -> &str {
        &self.done
    }

    /// 조합 중인 글자.
    pub fn composing(&self) -> Option<char> {
        self.composer.preedit()
    }

    /// 조합 중인 글자까지 포함한 전체 글.
    pub fn text(&self) -> String {
        self.done.chars().chain(self.composing()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 쿼티 자판에서 `keys`를 치는 손놀림 그대로 글쇠를 누른다.
    fn press_all(typed: &mut TypedText, keys: &str, script: Script) {
        for c in keys.chars() {
            let (pos, shift) = KeyPos::all()
                .flat_map(|pos| [(pos, false), (pos, true)])
                .find(|&(pos, shift)| qwerty(pos, shift) == c)
                .expect("쿼티 자판으로 칠 수 있는 문자");
            typed.press(pos, shift, script);
        }
    }

    fn hangul(keys: &str) -> String {
        let mut typed = TypedText::default();
        press_all(&mut typed, keys, Script::Hangul);
        typed.text()
    }

    #[test]
    fn keeping_the_front_drops_the_rest_and_the_syllable_in_progress() {
        let mut typed = TypedText::default();
        press_all(&mut typed, "rksk ekf", Script::Hangul);
        assert_eq!((typed.done(), typed.composing()), ("가나 ", Some('달')));
        typed.keep(2);
        assert_eq!((typed.done(), typed.composing()), ("가나", None));
        // 가진 것보다 많이 남기라고 하면 조합 중인 글자만 지운다.
        press_all(&mut typed, "e", Script::Hangul);
        typed.keep(10);
        assert_eq!(typed.text(), "가나");
        typed.keep(0);
        assert_eq!(typed.text(), "");
    }

    #[test]
    fn hangul_keys_compose() {
        assert_eq!(hangul("gksrmf"), "한글");
        assert_eq!(
            hangul("ekfkawnl gjs cptqkznldp xkrhvk"),
            "다람쥐 헌 쳇바퀴에 타고파"
        );
        assert_eq!(hangul("RkRk Tkd"), "까까 쌍");
    }

    #[test]
    fn non_hangul_keys_end_composition() {
        assert_eq!(hangul("dkssud, 1qjs!"), "안녕, 1번!");
        assert_eq!(hangul("rk sk"), "가 나");
    }

    #[test]
    fn latin_keys_type_as_printed() {
        let mut typed = TypedText::default();
        press_all(&mut typed, "let v: Vec<u8> = vec![1, 2];", Script::Latin);
        assert_eq!(typed.text(), "let v: Vec<u8> = vec![1, 2];");
    }

    #[test]
    fn script_can_change_mid_line() {
        let mut typed = TypedText::default();
        press_all(&mut typed, "gks", Script::Hangul);
        press_all(&mut typed, "ok", Script::Latin);
        press_all(&mut typed, "rmf", Script::Hangul);
        assert_eq!(typed.text(), "한ok글");
    }

    #[test]
    fn backspace_undoes_jamo_then_whole_characters() {
        let mut typed = TypedText::default();
        press_all(&mut typed, "rk gks", Script::Hangul);
        assert_eq!((typed.done(), typed.composing()), ("가 ", Some('한')));
        let mut seen = Vec::new();
        for _ in 0..6 {
            typed.backspace();
            seen.push(typed.text());
        }
        assert_eq!(seen, ["가 하", "가 ㅎ", "가 ", "가", "", ""]);
    }
}
