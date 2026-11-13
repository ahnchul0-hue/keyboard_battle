//! 기록을 두는 곳.
//!
//! 창으로 여는 프로그램은 사용자 자료 폴더의 글 파일 하나에 기록을 한 줄씩 덧붙인다.
//! 브라우저에서는 그 브라우저의 저장소(localStorage)에 둔다. 어느 쪽이든 읽거나 적지 못해도
//! 연습은 그대로 할 수 있고, 못 한 까닭을 설정에서 보여 준다.

use keyboard_battle::records::Record;

/// 기록을 두는 폴더(창)와 저장소 항목(브라우저)의 이름.
#[cfg(not(target_arch = "wasm32"))]
const FOLDER: &str = "keyboard_battle";
#[cfg(not(target_arch = "wasm32"))]
const FILE: &str = "records.txt";
#[cfg(target_arch = "wasm32")]
const ITEM: &str = "keyboard_battle.records";
/// 브라우저에 남겨 두는 가장 많은 기록 수. 저장소의 크기에는 끝이 있다.
#[cfg(target_arch = "wasm32")]
const KEPT_IN_BROWSER: usize = 2000;

enum Place {
    /// 어디에도 적지 않는다.
    Nowhere,
    /// 이 파일에 덧붙인다. `gap`이면 파일이 줄바꿈 없이 끝나 있어서, 적기 전에 줄부터 바꾼다.
    #[cfg(not(target_arch = "wasm32"))]
    File { path: std::path::PathBuf, gap: bool },
    #[cfg(target_arch = "wasm32")]
    Browser,
}

pub struct Store {
    place: Place,
    /// 읽거나 적지 못했으면 그 까닭.
    trouble: Option<String>,
}

/// 지금 시각. 1970년 1월 1일 0시(UTC)부터 센 초다.
#[cfg(not(target_arch = "wasm32"))]
pub fn now() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let since_epoch = SystemTime::now().duration_since(UNIX_EPOCH);
    since_epoch.map_or(0, |time| time.as_secs())
}

#[cfg(target_arch = "wasm32")]
pub fn now() -> u64 {
    (eframe::web_sys::js_sys::Date::now() / 1000.0) as u64
}

/// OS마다 프로그램의 자료를 두라고 정해 준 사용자 폴더.
#[cfg(not(target_arch = "wasm32"))]
fn data_folder() -> Option<std::path::PathBuf> {
    use std::path::PathBuf;
    let folder = |name: &str| {
        let value = std::env::var_os(name).map(PathBuf::from);
        value.filter(|path| path.is_absolute())
    };
    if cfg!(target_os = "windows") {
        folder("APPDATA")
    } else if cfg!(target_os = "macos") {
        folder("HOME").map(|home| home.join("Library").join("Application Support"))
    } else {
        let home = || folder("HOME").map(|home| home.join(".local").join("share"));
        folder("XDG_DATA_HOME").or_else(home)
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn append(path: &std::path::Path, gap: bool, record: &Record) -> std::io::Result<()> {
    use std::io::Write as _;
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder)?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    let start = if gap { "\n" } else { "" };
    // 한 번에 적어서, 줄이 반만 적힌 채로 남는 일을 줄인다.
    file.write_all(format!("{start}{}\n", record.to_line()).as_bytes())
}

#[cfg(target_arch = "wasm32")]
fn browser_storage() -> Result<eframe::web_sys::Storage, String> {
    let window = eframe::web_sys::window().ok_or("창을 찾지 못했습니다")?;
    match window.local_storage() {
        Ok(Some(storage)) => Ok(storage),
        Ok(None) => Err("이 브라우저는 저장소를 내주지 않습니다".to_owned()),
        Err(error) => Err(format!("{error:?}")),
    }
}

impl Store {
    /// 어디에도 적지 않는 곳. 기록은 프로그램을 끌 때까지만 남는다.
    #[cfg(test)]
    pub fn nowhere() -> Self {
        Self {
            place: Place::Nowhere,
            trouble: None,
        }
    }

    /// 기록을 두는 곳을 찾아, 거기에 있던 기록과 함께 돌려준다. 기록은 오래된 것부터다.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn open() -> (Self, Vec<Record>) {
        match data_folder() {
            Some(folder) => Self::open_file(folder.join(FOLDER).join(FILE)),
            None => {
                let trouble = "기록을 둘 사용자 폴더를 찾지 못했습니다.".to_owned();
                let store = Self {
                    place: Place::Nowhere,
                    trouble: Some(trouble),
                };
                (store, Vec::new())
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn open_file(path: std::path::PathBuf) -> (Self, Vec<Record>) {
        use keyboard_battle::records::parse_all;
        // 글자가 깨진 데가 있어도 읽을 수 있는 줄은 살린다.
        let (text, trouble) = match std::fs::read(&path) {
            Ok(bytes) => (String::from_utf8_lossy(&bytes).into_owned(), None),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (String::new(), None),
            Err(error) => (
                String::new(),
                Some(format!("기록을 읽지 못했습니다: {error}")),
            ),
        };
        let gap = !text.is_empty() && !text.ends_with('\n');
        let store = Self {
            place: Place::File { path, gap },
            trouble,
        };
        (store, parse_all(&text))
    }

    #[cfg(target_arch = "wasm32")]
    pub fn open() -> (Self, Vec<Record>) {
        use keyboard_battle::records::parse_all;
        let read = browser_storage().and_then(|storage| {
            let item = storage.get_item(ITEM);
            item.map_err(|error| format!("{error:?}"))
        });
        match read {
            Ok(text) => {
                let store = Self {
                    place: Place::Browser,
                    trouble: None,
                };
                (store, parse_all(&text.unwrap_or_default()))
            }
            Err(error) => {
                let trouble = format!("이 창에서는 기록을 저장할 수 없습니다: {error}");
                let store = Self {
                    place: Place::Nowhere,
                    trouble: Some(trouble),
                };
                (store, Vec::new())
            }
        }
    }

    /// 새 기록 `record`를 적는다. `all`은 그 기록까지 든 모든 기록이다.
    pub fn save(&mut self, record: &Record, all: &[Record]) {
        let saved: Result<(), String> = match &mut self.place {
            Place::Nowhere => return,
            #[cfg(not(target_arch = "wasm32"))]
            Place::File { path, gap } => {
                let _ = all;
                let saved = append(path, *gap, record);
                *gap &= saved.is_err();
                saved.map_err(|error| error.to_string())
            }
            #[cfg(target_arch = "wasm32")]
            Place::Browser => {
                let _ = record;
                let kept = &all[all.len().saturating_sub(KEPT_IN_BROWSER)..];
                let lines: Vec<String> = kept.iter().map(Record::to_line).collect();
                browser_storage().and_then(|storage| {
                    let set = storage.set_item(ITEM, &lines.join("\n"));
                    set.map_err(|error| format!("{error:?}"))
                })
            }
        };
        self.trouble = saved
            .err()
            .map(|error| format!("기록을 저장하지 못했습니다: {error}"));
    }

    /// 기록을 어디에 두는지 알려 주는 말.
    pub fn place(&self) -> String {
        match &self.place {
            Place::Nowhere => "기록은 프로그램을 끌 때까지만 남습니다.".to_owned(),
            #[cfg(not(target_arch = "wasm32"))]
            Place::File { path, .. } => format!("기록 파일: {}", path.display()),
            #[cfg(target_arch = "wasm32")]
            Place::Browser => {
                "기록은 이 브라우저에 남습니다. 다른 브라우저나 기기에서는 보이지 않습니다."
                    .to_owned()
            }
        }
    }

    /// 기록을 읽거나 적지 못했으면 그 까닭.
    pub fn trouble(&self) -> Option<&str> {
        self.trouble.as_deref()
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use keyboard_battle::measure::Tally;

    fn record(at: u64) -> Record {
        let total = Tally {
            needed: 900,
            chars: 400,
            presses: 910,
            wrong: 10,
            timed: 899,
            seconds: 180.0,
        };
        Record {
            at,
            minutes: 3,
            total,
            kinds: [total, Tally::default(), Tally::default()],
        }
    }

    /// 테스트마다 따로 쓰는 빈 폴더 속의 기록 파일 자리.
    fn scratch(name: &str) -> std::path::PathBuf {
        let folder = std::env::temp_dir().join(format!(
            "keyboard_battle_test_{}_{name}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&folder);
        folder.join("deep").join(FILE)
    }

    #[test]
    fn records_are_appended_and_read_back_in_order() {
        let path = scratch("append");
        let (mut store, records) = Store::open_file(path.clone());
        assert!(records.is_empty() && store.trouble().is_none());
        assert!(
            !path.exists(),
            "적을 것이 생기기 전에는 파일을 만들지 않는다"
        );

        let mut all = Vec::new();
        for at in [10, 20, 30] {
            all.push(record(at));
            store.save(&record(at), &all);
            assert_eq!(store.trouble(), None);
        }
        let (store, read) = Store::open_file(path.clone());
        assert_eq!(read, all);
        assert!(store.place().ends_with(FILE));
        std::fs::remove_dir_all(path.parent().unwrap().parent().unwrap()).unwrap();
    }

    /// 손으로 고치다 줄바꿈 없이 끝난 파일에도 새 기록은 제 줄에 적힌다. 깨진 줄은 건너뛴다.
    #[test]
    fn a_file_edited_by_hand_still_takes_new_records() {
        let path = scratch("edited");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut bytes = b"\xff\xfe broken\n".to_vec();
        bytes.extend(record(10).to_line().as_bytes());
        std::fs::write(&path, bytes).unwrap();

        let (mut store, mut all) = Store::open_file(path.clone());
        assert_eq!(all, [record(10)]);
        for at in [20, 30] {
            all.push(record(at));
            store.save(&record(at), &all);
        }
        let (_, read) = Store::open_file(path.clone());
        assert_eq!(read, all);
        std::fs::remove_dir_all(path.parent().unwrap().parent().unwrap()).unwrap();
    }

    /// 적을 수 없는 곳이어도 프로그램은 멈추지 않고, 못 적은 까닭을 알려 준다.
    #[test]
    fn a_place_that_cannot_be_written_reports_trouble() {
        let path = scratch("blocked");
        // 폴더가 있어야 할 자리에 파일을 두어, 그 아래에 아무것도 만들 수 없게 한다.
        let folder = path.parent().unwrap();
        std::fs::create_dir_all(folder.parent().unwrap()).unwrap();
        std::fs::write(folder, "폴더가 아니다").unwrap();

        let (mut store, records) = Store::open_file(path.clone());
        assert!(records.is_empty());
        store.save(&record(10), &[record(10)]);
        let trouble = store.trouble().expect("저장하지 못한 까닭");
        assert!(
            trouble.starts_with("기록을 저장하지 못했습니다"),
            "{trouble}"
        );
        std::fs::remove_dir_all(folder.parent().unwrap()).unwrap();
    }

    #[test]
    fn nowhere_keeps_nothing_and_has_no_trouble() {
        let mut store = Store::nowhere();
        store.save(&record(10), &[record(10)]);
        assert_eq!(store.trouble(), None);
        assert!(store.place().contains("끌 때까지"));
    }
}
