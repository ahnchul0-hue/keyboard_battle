//! 둘러보기: 프로그램이 스스로 첫 글을 쳐 보이며 화면의 각 부분을 설명한다.

use crate::keymap::keystroke_for;
use crate::measure::key_units;
use crate::player::{Player, Step};
use crate::round::{Action, Round};
use crate::texts;

/// 글쇠 사이의 간격(초). 1분에 350타쯤 되는 빠르기다.
const KEY_GAP: f64 = 0.17;

/// 둘러보기의 대본을 써 나가는 곳.
struct Script {
    /// 다음 일을 할 때 (시작부터 흐른 시간, 초).
    at: f64,
    /// 과정에 남길 시각(초). 설명을 읽는 동안에는 흐르지 않아서, 보여 주는 속도가 치는 빠르기 그대로 나온다.
    typing: f64,
    steps: Vec<(f64, Step)>,
}

impl Script {
    /// 설명을 띄우고, 읽을 시간을 `read`초 준다.
    fn say(&mut self, caption: &'static str, read: f64) {
        self.steps.push((self.at, Step::Say(caption)));
        self.at += read;
    }

    /// 일 하나를 하고 `gap`초 쉰다.
    fn act(&mut self, action: Action, gap: f64) {
        self.steps.push((self.at, Step::Do(action, self.typing)));
        self.at += gap;
        self.typing += gap;
    }

    /// `text`를 친다.
    fn keys(&mut self, text: &str) {
        for unit in key_units(text) {
            let (pos, shift) = keystroke_for(unit).expect("둘러보기는 칠 수 있는 글만 친다");
            self.act(Action::Press(pos, shift), KEY_GAP);
        }
    }
}

/// 둘러보기를 시작한다. `now`는 시작하는 시각(초)이다.
///
/// 늘 같은 처음 두 쪽의 글을 쓰고, 실제로 치는 연습과는 따로 논다.
pub fn tour(now: f64) -> Player {
    let mut script = Script {
        at: 0.0,
        typing: 0.0,
        steps: Vec::new(),
    };
    script.say("지금 칠 글은 테두리가 굵은 블럭에 있습니다.", 2.0);
    script.keys("다람쥐");
    script.say(
        "한/영 키는 누르지 않습니다. 문장에 맞춰 한글과 영문이 저절로 바뀝니다.",
        2.4,
    );
    script.keys(" 헌 ");
    script.say("틀리면 그 자리가 빨갛게 표시됩니다.", 0.8);
    // '쳇'의 첫소리를 일부러 틀린다.
    script.keys("제");
    script.at += 1.2;
    script.say("Backspace로 지우고 다시 칩니다.", 0.8);
    script.act(Action::Backspace, 0.4);
    script.act(Action::Backspace, 0.5);
    script.keys("쳇바퀴에 타고파");
    script.say(
        "글은 한 문단씩 나옵니다. 문단을 다 치면 아래의 다음 글이 올라옵니다.",
        2.2,
    );
    script.keys(" 얘, 옛");
    let last = "정한 시간이 다 되면 결과 창이 뜹니다. 이제 직접 쳐 보세요.";
    script.say(last, 2.4);
    // 마지막 설명을 읽을 시간을 벌어 둔다.
    script.say(last, 0.0);
    let [first, next] = texts::opening();
    let mut round = Round::new(first);
    // 다음에 칠 글이 아래에 미리 보이는 모습까지 보여 준다.
    round.extend(next);
    Player::new(round, script.steps, "", now)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 대본대로 끝까지 가면, 첫 글을 한 번 틀렸다가 고치고 둘째 문장의 앞머리까지 친다.
    #[test]
    fn the_tour_types_into_the_first_page_with_one_slip() {
        let mut player = tour(0.0);
        player.advance(1000.0);
        let round = player.round();
        assert_eq!(round.typed().text(), "다람쥐 헌 쳇바퀴에 타고파 얘, 옛");
        assert!(round.results().is_empty(), "문단을 다 치지는 않는다");
        assert_eq!(round.page_of(0).end, 1);
        assert_eq!(round.lines().len(), 2, "미리 보일 다음 쪽이 있다");
        let so_far = round.so_far(player.clock());
        assert_eq!((so_far.wrong, round.slips()[0].len()), (2, 1));
        // 설명을 읽는 시간은 속도에 들어가지 않는다. 틀리고 지운 만큼만 느려진다.
        let flawless = 60.0 / KEY_GAP;
        assert!(so_far.per_minute() > flawless * 0.7 && so_far.per_minute() < flawless);
    }

    #[test]
    fn the_tour_takes_about_twenty_seconds_and_always_explains_something() {
        let mut player = tour(100.0);
        let mut captions = Vec::new();
        let mut ended_at = None;
        for tenth in 0..300 {
            let now = 100.0 + f64::from(tenth) / 10.0;
            player.advance(now);
            assert!(
                !player.caption().is_empty(),
                "설명 없이 치기만 하는 때가 없다"
            );
            if captions.last() != Some(&player.caption()) {
                captions.push(player.caption());
            }
            if player.is_over(now) && ended_at.is_none() {
                ended_at = Some(now - 100.0);
            }
        }
        assert_eq!(captions.len(), 6);
        let seconds = ended_at.expect("둘러보기는 끝난다");
        assert!((17.0..23.0).contains(&seconds), "{seconds}초");
    }
}
