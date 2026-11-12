//! 적어 둔 일을 제때에 맞춰 다시 해 보인다. 다시 보기와 둘러보기에 쓴다.

use crate::round::{Action, Round};
use crate::texts::Sentence;

/// 다시 보기를 시작하고 첫 글쇠가 눌리기까지의 뜸(초).
const LEAD_IN: f64 = 0.6;
/// 다시 보기에서 글쇠 사이를 이보다(초) 오래 쉬지는 않는다. 생각하느라 멈춘 시간까지 지켜볼 까닭이 없다.
const LONGEST_PAUSE: f64 = 1.2;
/// 마지막 일을 하고 나서 끝나기까지의 뜸(초).
const TAIL: f64 = 1.0;

/// 정해진 때에 할 일.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Step {
    /// 과정에 일 하나를 한다. 뒤의 값은 과정에 남길 시각(초)이다.
    Do(Action, f64),
    /// 화면의 설명을 바꾼다.
    Say(&'static str),
}

#[derive(Clone, Debug)]
pub struct Player {
    round: Round,
    /// 시작한 뒤 흐른 시간(초)과 그때 할 일. 시간 순서로 늘어서 있다.
    steps: Vec<(f64, Step)>,
    /// 다음에 할 일의 번호.
    next: usize,
    started_at: f64,
    caption: &'static str,
    /// 과정에 마지막으로 남긴 시각(초).
    clock: f64,
}

impl Player {
    /// `lines`를 처음부터 치는 과정에 `steps`를 차례로 해 보인다. `now`는 시작하는 시각(초)이다.
    pub fn new(
        lines: Vec<Sentence>,
        steps: Vec<(f64, Step)>,
        caption: &'static str,
        now: f64,
    ) -> Self {
        Self {
            round: Round::new(lines),
            steps,
            next: 0,
            started_at: now,
            caption,
            clock: 0.0,
        }
    }

    /// `original`을 친 그대로 다시 보인다. 오래 멈춘 곳은 줄여서 보이되,
    /// 과정에는 본디 시각을 남기므로 속도와 정확도는 처음과 같게 나온다.
    pub fn replay(original: &Round, caption: &'static str, now: f64) -> Self {
        let mut at = LEAD_IN;
        let mut previous: Option<f64> = None;
        let steps = original.log().iter().map(|&(time, action)| {
            if let Some(previous) = previous {
                at += (time - previous).clamp(0.0, LONGEST_PAUSE);
            }
            previous = Some(time);
            (at, Step::Do(action, time))
        });
        Self::new(original.lines().to_vec(), steps.collect(), caption, now)
    }

    /// 다시 해 보이고 있는 과정.
    pub fn round(&self) -> &Round {
        &self.round
    }

    /// 화면에 띄울 설명.
    pub fn caption(&self) -> &'static str {
        self.caption
    }

    /// 과정에 마지막으로 남긴 시각(초). 과정의 지금까지 기록을 낼 때 쓴다.
    pub fn clock(&self) -> f64 {
        self.clock
    }

    /// `now`까지 할 일을 한다. 과정에 한 일이 있으면 `true`.
    pub fn advance(&mut self, now: f64) -> bool {
        let elapsed = now - self.started_at;
        let mut acted = false;
        while let Some(&(at, step)) = self.steps.get(self.next) {
            if at > elapsed {
                break;
            }
            match step {
                Step::Do(action, time) => {
                    self.round.apply(action, time);
                    self.clock = time;
                    acted = true;
                }
                Step::Say(caption) => self.caption = caption,
            }
            self.next += 1;
        }
        acted
    }

    /// 다음 일을 하기까지, 할 일이 더 없으면 끝나기까지 남은 시간(초).
    pub fn wait(&self, now: f64) -> f64 {
        let elapsed = now - self.started_at;
        let last = self.steps.last().map_or(0.0, |(at, _)| *at);
        let due = self.steps.get(self.next).map_or(last + TAIL, |(at, _)| *at);
        (due - elapsed).max(0.0)
    }

    /// 할 일을 다 하고 마지막 뜸까지 지났는지.
    pub fn is_over(&self, now: f64) -> bool {
        self.next == self.steps.len() && self.wait(now) == 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap::keystroke_for;
    use crate::texts::Kind;

    fn press(unit: char) -> Action {
        let (pos, shift) = keystroke_for(unit).expect("글쇠 하나로 치는 문자");
        Action::Press(pos, shift)
    }

    /// 'ok'를 치다 한 번 틀리고, 한참 쉬었다가 마저 친 과정.
    fn original() -> Round {
        let line = Sentence {
            kind: Kind::English,
            text: "ok",
        };
        let mut round = Round::new(vec![line]);
        round.apply(press('o'), 100.0);
        round.apply(press('x'), 100.2);
        round.apply(Action::Backspace, 100.5);
        round.apply(press('k'), 130.5);
        assert!(round.is_finished());
        round
    }

    #[test]
    fn replay_gives_the_same_record_as_the_original() {
        let original = original();
        let mut player = Player::replay(&original, "다시 보기", 0.0);
        assert!(player.round().results().is_empty());
        player.advance(1000.0);
        assert_eq!(player.round().results(), original.results());
        assert_eq!(player.round().slips(), original.slips());
        assert_eq!(player.clock(), 130.5);
    }

    #[test]
    fn replay_keeps_the_rhythm_but_shortens_long_pauses() {
        let mut player = Player::replay(&original(), "다시 보기", 50.0);
        let typed = |player: &Player| player.round().typed().text();

        assert!(
            !player.advance(50.0 + LEAD_IN - 0.01),
            "뜸을 들인 뒤에 시작한다"
        );
        assert!(player.advance(50.0 + LEAD_IN));
        assert_eq!(typed(&player), "o");
        assert!((player.wait(50.0 + LEAD_IN) - 0.2).abs() < 1e-9);

        player.advance(50.0 + LEAD_IN + 0.5);
        assert_eq!(typed(&player), "o", "틀린 글쇠를 지운 데까지");
        // 본디 30초를 쉬었지만 다시 보기에서는 1.2초만 쉰다.
        assert!(!player.advance(50.0 + LEAD_IN + 0.5 + LONGEST_PAUSE - 0.01));
        assert!(player.advance(50.0 + LEAD_IN + 0.5 + LONGEST_PAUSE));
        assert!(player.round().is_finished());

        let end = 50.0 + LEAD_IN + 0.5 + LONGEST_PAUSE;
        assert!(!player.is_over(end), "마지막 모습을 잠깐 보여 준다");
        assert!(player.is_over(end + TAIL));
    }

    #[test]
    fn captions_change_on_cue() {
        let line = Sentence {
            kind: Kind::English,
            text: "ok",
        };
        let steps = vec![
            (1.0, Step::Say("첫째")),
            (2.0, Step::Do(press('o'), 0.0)),
            (3.0, Step::Say("둘째")),
        ];
        let mut player = Player::new(vec![line], steps, "처음", 10.0);
        assert_eq!(player.caption(), "처음");
        assert!(!player.advance(11.0), "설명만 바뀌었고 과정에 한 일은 없다");
        assert_eq!(player.caption(), "첫째");
        assert!(player.advance(12.5));
        assert_eq!(
            (player.caption(), player.round().typed().text()),
            ("첫째", "o".to_owned())
        );
        player.advance(13.0);
        assert_eq!(player.caption(), "둘째");
        assert!(player.is_over(13.0 + TAIL));
    }
}
