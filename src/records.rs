//! 끝까지 한 연습의 기록과, 새 결과를 지난 기록과 견주는 일.
//!
//! 기록 하나는 글 한 줄이다. `이름=값`을 빈칸으로 이어 적으므로, 나중에 항목이 늘어도
//! 예전에 적은 기록을 그대로 읽을 수 있다. 읽지 못하는 줄은 건너뛴다.

use crate::measure::Tally;
use crate::texts::Kind;

/// 평균을 낼 때 보는 최근 기록 수.
pub const RECENT: usize = 10;
/// 흐름을 그릴 때 보는 최근 기록 수.
pub const TRAIL: usize = 20;

/// 끝까지 한 연습 하나의 기록.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Record {
    /// 연습을 끝낸 때. 1970년 1월 1일 0시(UTC)부터 센 초다.
    pub at: u64,
    /// 연습 시간(분).
    pub minutes: u32,
    /// 연습 전체의 기록.
    pub total: Tally,
    /// 한글, 영문, 러스트를 따로 본 기록. [`Kind::ALL`]의 순서다.
    pub kinds: [Tally; 3],
}

/// 종류마다 기록에 적는 이름.
const KIND_KEYS: [&str; 3] = ["hangul", "english", "rust"];

fn tally_text(tally: &Tally) -> String {
    let Tally {
        needed,
        chars,
        presses,
        wrong,
        timed,
        seconds,
    } = tally;
    format!("{needed},{chars},{presses},{wrong},{timed},{seconds}")
}

fn tally_of(text: &str) -> Option<Tally> {
    let mut parts = text.split(',');
    let mut count = || parts.next()?.parse::<u32>().ok();
    let tally = Tally {
        needed: count()?,
        chars: count()?,
        presses: count()?,
        wrong: count()?,
        timed: count()?,
        seconds: 0.0,
    };
    let seconds: f64 = parts.next()?.parse().ok()?;
    (seconds.is_finite() && seconds >= 0.0).then_some(Tally { seconds, ..tally })
}

impl Record {
    pub fn speed(&self) -> f64 {
        self.total.per_minute()
    }

    pub fn accuracy(&self) -> f64 {
        self.total.accuracy()
    }

    /// 이 기록을 적은 한 줄. 줄바꿈은 들어 있지 않다.
    pub fn to_line(&self) -> String {
        let mut line = format!(
            "at={} minutes={} total={}",
            self.at,
            self.minutes,
            tally_text(&self.total)
        );
        for (key, tally) in KIND_KEYS.iter().zip(&self.kinds) {
            line += &format!(" {key}={}", tally_text(tally));
        }
        line
    }

    /// 한 줄에서 기록을 읽는다. 끝낸 때·연습 시간·전체 기록이 없거나 깨졌으면 `None`이다.
    /// 모르는 항목은 지나치고, 종류별 기록이 없으면 비워 둔다.
    pub fn parse(line: &str) -> Option<Self> {
        let (mut at, mut minutes, mut total) = (None, None, None);
        let mut kinds = [Tally::default(); 3];
        for part in line.split_whitespace() {
            let Some((key, value)) = part.split_once('=') else {
                continue;
            };
            match key {
                "at" => at = value.parse().ok(),
                "minutes" => minutes = value.parse().ok().filter(|&minutes: &u32| minutes > 0),
                "total" => total = tally_of(value),
                _ => {
                    if let Some(kind) = KIND_KEYS.iter().position(|&name| name == key) {
                        kinds[kind] = tally_of(value)?;
                    }
                }
            }
        }
        Some(Self {
            at: at?,
            minutes: minutes?,
            total: total?,
            kinds,
        })
    }
}

/// 여러 줄에서 읽을 수 있는 기록만 차례대로 골라낸다.
pub fn parse_all(text: &str) -> Vec<Record> {
    text.lines().filter_map(Record::parse).collect()
}

/// 기록 몇 개의 속도와 정확도를 평균한 것.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mean {
    /// 타/분.
    pub speed: f64,
    /// 0 ~ 1.
    pub accuracy: f64,
    /// 평균에 든 기록 수.
    pub count: usize,
}

/// 친 것이 있는 기록들의 평균. 하나도 없으면 `None`이다.
fn mean(tallies: impl Iterator<Item = Tally>) -> Option<Mean> {
    let (mut speed, mut accuracy, mut count) = (0.0, 0.0, 0usize);
    for tally in tallies.filter(|tally| tally.timed > 0) {
        speed += tally.per_minute();
        accuracy += tally.accuracy();
        count += 1;
    }
    (count > 0).then(|| Mean {
        speed: speed / count as f64,
        accuracy: accuracy / count as f64,
        count,
    })
}

/// 지난 기록들에서 뽑은, 새 결과와 견줄 것들.
///
/// 속도와 정확도는 연습 시간이 같은 기록끼리만 견준다. 연습은 늘 한글 문단으로 시작해 영문, 러스트로
/// 넘어가므로, 시간이 다르면 치는 글의 구성이 다르다. 1분 연습에는 느린 러스트가 아예 나오지 않는다.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Standing {
    /// 끝까지 한 모든 연습의 수.
    pub practices: usize,
    /// 그 연습들의 시간(분)을 더한 것.
    pub minutes: u64,
    /// 그 가운데 연습 시간이 같은 것의 수. 아래 다섯 가지는 이 기록들에서 뽑는다.
    pub alike: usize,
    /// 바로 앞의 기록.
    pub previous: Option<Record>,
    /// 최근 [`RECENT`]번의 평균.
    pub recent: Option<Mean>,
    /// 가장 빨랐던 기록.
    pub fastest: Option<Record>,
    /// 가장 정확했던 기록.
    pub cleanest: Option<Record>,
    /// 최근 [`TRAIL`]번의 속도(타/분)와 정확도(0 ~ 1). 오래된 것부터다.
    pub trail: Vec<(f64, f64)>,
    /// 종류마다 본 최근 [`RECENT`]번의 평균. 그 종류를 친 기록이면 연습 시간을 가리지 않고 센다.
    /// 한 종류만 따로 본 속도는 연습 시간이 달라도 견줄 수 있다.
    pub recent_kinds: [Option<Mean>; 3],
}

impl Standing {
    /// 기록이 하나도 없을 때의 것.
    pub const fn empty() -> Self {
        Self {
            practices: 0,
            minutes: 0,
            alike: 0,
            previous: None,
            recent: None,
            fastest: None,
            cleanest: None,
            trail: Vec::new(),
            recent_kinds: [None; 3],
        }
    }

    /// `minutes`분 연습의 결과와 견줄 것을 `records`에서 뽑는다. `records`는 오래된 것부터 차례대로다.
    pub fn of(records: &[Record], minutes: u32) -> Self {
        let alike: Vec<Record> = records
            .iter()
            .filter(|record| record.minutes == minutes)
            .copied()
            .collect();
        let latest = |count: usize| &alike[alike.len().saturating_sub(count)..];
        let by = |measure: fn(&Record) -> f64| {
            let order = |a: &&Record, b: &&Record| measure(a).total_cmp(&measure(b));
            // 같은 값이면 먼저 세운 기록을 든다.
            alike.iter().rev().max_by(order).copied()
        };
        Self {
            practices: records.len(),
            minutes: records.iter().map(|record| u64::from(record.minutes)).sum(),
            alike: alike.len(),
            previous: alike.last().copied(),
            recent: mean(latest(RECENT).iter().map(|record| record.total)),
            fastest: by(Record::speed),
            cleanest: by(Record::accuracy),
            trail: latest(TRAIL)
                .iter()
                .map(|record| (record.speed(), record.accuracy()))
                .collect(),
            recent_kinds: Kind::ALL.map(|kind| {
                let of_kind = records.iter().map(|record| record.kinds[kind as usize]);
                let typed: Vec<Tally> = of_kind.filter(|tally| tally.timed > 0).collect();
                mean(typed[typed.len().saturating_sub(RECENT)..].iter().copied())
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `speed`타/분으로 `minutes`분 동안, 백 번에 `wrong`번꼴로 틀리며 친 기록.
    fn tally(speed: u32, minutes: u32, wrong: u32) -> Tally {
        let timed = speed * minutes;
        Tally {
            needed: timed + 1,
            chars: timed / 2,
            presses: timed + 1,
            wrong: (timed + 1) * wrong / 100,
            timed,
            seconds: f64::from(minutes) * 60.0,
        }
    }

    fn record(at: u64, speed: u32, wrong: u32) -> Record {
        Record {
            at,
            minutes: 3,
            total: tally(speed, 3, wrong),
            kinds: [tally(speed, 1, wrong); 3],
        }
    }

    #[test]
    fn a_record_survives_being_written_and_read() {
        let mut written = record(1_800_000_000, 312, 3);
        written.total.seconds = 179.123_456_789;
        written.kinds[2] = Tally::default();
        let line = written.to_line();
        assert!(!line.contains('\n'));
        assert_eq!(Record::parse(&line), Some(written));
        assert_eq!(
            Record {
                at: 5,
                minutes: 1,
                total: Tally {
                    needed: 10,
                    chars: 4,
                    presses: 12,
                    wrong: 2,
                    timed: 9,
                    seconds: 1.5,
                },
                kinds: [Tally::default(); 3],
            }
            .to_line(),
            "at=5 minutes=1 total=10,4,12,2,9,1.5 \
             hangul=0,0,0,0,0,0 english=0,0,0,0,0,0 rust=0,0,0,0,0,0"
        );
    }

    /// 나중에 항목이 늘거나 순서가 바뀌어도 읽고, 종류별 기록이 없는 줄도 읽는다.
    #[test]
    fn unknown_and_missing_parts_do_not_stop_a_record_being_read() {
        let read = Record::parse("mood=good total=10,4,12,2,9,1.5 minutes=1  at=5 extra")
            .expect("읽을 수 있는 줄");
        assert_eq!((read.at, read.minutes, read.total.wrong), (5, 1, 2));
        assert_eq!(read.kinds, [Tally::default(); 3]);
    }

    #[test]
    fn broken_lines_are_skipped() {
        for line in [
            "",
            "# 주석",
            "at=5 minutes=1",
            "at=5 total=10,4,12,2,9,1.5",
            "at=five minutes=1 total=10,4,12,2,9,1.5",
            "at=5 minutes=0 total=10,4,12,2,9,1.5",
            "at=5 minutes=1 total=10,4,12,2,9",
            "at=5 minutes=1 total=10,4,12,2,9,NaN",
            "at=5 minutes=1 total=10,4,12,2,9,-3",
            "at=5 minutes=1 total=10,4,12,2,9,1.5 rust=1,2",
        ] {
            assert_eq!(Record::parse(line), None, "{line:?}");
        }
        let good = record(7, 300, 0).to_line();
        let text = format!("깨진 줄\n{good}\n\nat=1\n{good}\n");
        assert_eq!(parse_all(&text).len(), 2);
    }

    #[test]
    fn nothing_to_compare_with_before_the_first_record() {
        let standing = Standing::of(&[], 3);
        assert_eq!(standing, Standing::default());
        assert_eq!(standing, Standing::empty());
        assert!(standing.previous.is_none() && standing.recent.is_none());
    }

    #[test]
    fn a_standing_picks_the_previous_the_recent_mean_and_the_bests() {
        // 속도는 300, 310, …, 410으로 오르고, 정확도는 둘째 기록만 100%다.
        let records: Vec<Record> = (0..12)
            .map(|n| record(1000 + n, 300 + 10 * n as u32, u32::from(n != 1) * 2))
            .collect();
        let standing = Standing::of(&records, 3);
        assert_eq!((standing.practices, standing.minutes), (12, 36));
        assert_eq!(standing.alike, 12);
        assert_eq!(standing.previous, Some(records[11]));
        assert_eq!(standing.fastest, Some(records[11]));
        assert_eq!(standing.cleanest, Some(records[1]));

        // 평균은 최근 열 번(320 ~ 410)만 본다.
        let recent = standing.recent.expect("기록이 있다");
        assert_eq!(recent.count, RECENT);
        assert!((recent.speed - 365.0).abs() < 1e-9, "{}", recent.speed);
        assert!((recent.accuracy - 0.98).abs() < 0.002);

        assert_eq!(standing.trail.len(), 12);
        assert!((standing.trail[0].0 - 300.0).abs() < 1e-9);
        assert!((standing.trail[11].0 - 410.0).abs() < 1e-9);
    }

    #[test]
    fn the_trail_keeps_only_the_latest_records() {
        let records: Vec<Record> = (0..50).map(|n| record(n, 200 + n as u32, 0)).collect();
        let standing = Standing::of(&records, 3);
        assert_eq!(standing.trail.len(), TRAIL);
        assert!((standing.trail[0].0 - 230.0).abs() < 1e-9);
    }

    /// 같은 값이면 먼저 세운 기록이 최고 기록이다.
    #[test]
    fn the_first_of_equal_bests_is_kept() {
        let records = [record(1, 300, 0), record(2, 300, 0)];
        let standing = Standing::of(&records, 3);
        assert_eq!(standing.fastest.map(|record| record.at), Some(1));
        assert_eq!(standing.cleanest.map(|record| record.at), Some(1));
    }

    /// 종류별 평균은 그 종류를 친 기록만 센다.
    #[test]
    fn kinds_are_averaged_over_the_records_that_have_them() {
        let mut records = vec![record(1, 300, 0), record(2, 400, 0), record(3, 500, 0)];
        // 둘째 연습에서는 러스트 차례가 오기 전에 시간이 다 됐다.
        records[1].kinds[Kind::Rust as usize] = Tally::default();
        let standing = Standing::of(&records, 3);
        let [hangul, _, rust] = standing.recent_kinds;
        let (hangul, rust) = (hangul.expect("한글"), rust.expect("러스트"));
        assert_eq!((hangul.count, rust.count), (3, 2));
        assert!((hangul.speed - 400.0).abs() < 1e-9);
        assert!((rust.speed - 400.0).abs() < 1e-9);

        let none = Standing::of(
            &[Record {
                kinds: [Tally::default(); 3],
                ..record(1, 300, 0)
            }],
            3,
        );
        assert_eq!(none.recent_kinds, [None; 3]);
    }

    /// 속도와 정확도는 연습 시간이 같은 기록끼리만 견준다. 1분 연습은 한글이 대부분이라 빠르다.
    #[test]
    fn only_practices_of_the_same_length_are_compared() {
        let short = |at, speed| Record {
            minutes: 1,
            ..record(at, speed, 0)
        };
        let records = [
            record(1, 300, 2),
            short(2, 420),
            record(3, 310, 1),
            short(4, 400),
        ];
        let three = Standing::of(&records, 3);
        assert_eq!((three.practices, three.minutes, three.alike), (4, 8, 2));
        assert_eq!(three.previous, Some(records[2]));
        assert_eq!(three.fastest, Some(records[2]));
        assert_eq!(three.trail.len(), 2);
        let mean = three.recent.expect("3분 연습이 둘 있다");
        assert!((mean.speed - 305.0).abs() < 1e-9);

        let one = Standing::of(&records, 1);
        assert_eq!(one.alike, 2);
        assert_eq!(one.previous, Some(records[3]));
        assert_eq!(one.fastest, Some(records[1]));

        // 한 번도 해 보지 않은 시간이면 견줄 기록이 없다. 그래도 지금까지 한 연습 수는 안다.
        let five = Standing::of(&records, 5);
        assert_eq!((five.practices, five.alike), (4, 0));
        assert!(five.previous.is_none() && five.recent.is_none() && five.trail.is_empty());
        // 종류별 평균은 연습 시간을 가리지 않는다.
        assert_eq!(five.recent_kinds, three.recent_kinds);
        assert_eq!(five.recent_kinds[0].map(|mean| mean.count), Some(4));
    }
}
