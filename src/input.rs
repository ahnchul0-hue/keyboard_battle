//! egui가 넘겨준 글쇠를 자판 위의 자리로 읽고, OS가 그 입력을 어떻게 넘겼는지 적어 둔다.

use std::collections::VecDeque;

use eframe::egui::Key;
use keyboard_battle::keymap::KeyPos;

const RECENT_EVENTS: usize = 16;

/// egui가 알려 준 글쇠를 자판 위의 자리로 바꾼다. 문자 글쇠가 아니면 `None`.
pub fn key_pos(key: Key) -> Option<KeyPos> {
    let legend = match key {
        Key::Backtick => '`',
        Key::Minus => '-',
        Key::Equals => '=',
        Key::OpenBracket => '[',
        Key::CloseBracket => ']',
        Key::Backslash => '\\',
        Key::Semicolon => ';',
        Key::Quote => '\'',
        Key::Comma => ',',
        Key::Period => '.',
        Key::Slash => '/',
        Key::Space => ' ',
        // 글자·숫자 글쇠는 이름이 곧 쿼티 자판에 새겨진 글자다 ("A", "7").
        _ => match *key.name().as_bytes() {
            [legend] => char::from(legend).to_ascii_lowercase(),
            _ => return None,
        },
    };
    KeyPos::from_us(legend)
}

/// OS가 글쇠 입력을 어떻게 넘겨줬는지에 대한 기록. 다른 PC에서 생긴 문제를 좁히는 데 쓴다.
#[derive(Default)]
pub struct InputStats {
    /// 눌린 문자 글쇠 수.
    pub keys: u32,
    /// 그중 OS가 문자를 함께 보내지 않은 글쇠 수. OS 입력기가 글쇠를 먼저 받았다는 뜻이다.
    pub taken_by_ime: u32,
    /// 자판 위의 자리를 알 수 없던 글쇠 수.
    pub without_position: u32,
    /// OS 입력기가 보낸 조합 이벤트 수.
    pub ime_events: u32,
    /// 가장 최근에 들어온 입력들.
    pub recent: VecDeque<String>,
}

impl InputStats {
    pub fn log(&mut self, entry: String) {
        if self.recent.len() == RECENT_EVENTS {
            self.recent.pop_front();
        }
        self.recent.push_back(entry);
    }

    pub fn verdict(&self) -> String {
        let (keys, taken_by_ime, unknown) = (self.keys, self.taken_by_ime, self.without_position);
        if unknown > 0 {
            format!(
                "자리를 알 수 없는 글쇠가 {unknown}개 있었습니다. 확인 결과를 복사해 알려 주세요."
            )
        } else if keys == 0 {
            "아직 입력이 없습니다.".to_owned()
        } else if cfg!(target_arch = "wasm32") {
            // 브라우저는 글쇠와 문자를 따로 알려 주지 않아, 아래 판정을 할 수 없다.
            format!(
                "글쇠 {keys}개를 받았습니다. 브라우저에서는 OS 입력기가 끼어들었는지 알 수 없습니다."
            )
        } else if taken_by_ime > 0 || self.ime_events > 0 {
            format!(
                "글쇠 {keys}개 중 {taken_by_ime}개는 OS 입력기가 먼저 받았지만, \
                 글쇠 자리로 읽어 그대로 처리했습니다."
            )
        } else {
            format!("글쇠 {keys}개 모두 OS 입력기를 거치지 않고 들어왔습니다.")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use keyboard_battle::keymap::qwerty;

    #[test]
    fn every_character_key_has_exactly_one_egui_key() {
        let mut legends: Vec<char> = Key::ALL
            .iter()
            .filter_map(|&key| key_pos(key))
            .map(|pos| qwerty(pos, false))
            .collect();
        legends.sort_unstable();
        let mut expected: Vec<char> = KeyPos::all().map(|pos| qwerty(pos, false)).collect();
        expected.sort_unstable();
        assert_eq!(legends, expected);
        assert_eq!(key_pos(Key::R), KeyPos::from_us('r'));
        assert_eq!(key_pos(Key::Num7), KeyPos::from_us('7'));
        assert_eq!(key_pos(Key::F1), None);
        assert_eq!(key_pos(Key::Enter), None);
    }
}
