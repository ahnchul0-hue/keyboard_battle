#!/bin/bash
# README에 넣는 시연(docs/demo.gif)을 다시 찍는다.
#
# 가상 화면에 프로그램을 띄우고, 글쇠를 일정한 간격으로 보내며 화면을 녹화한다.
# 사람이 친 것이 아니므로, 화면에 나오는 속도와 정확도는 이 대본의 것이다.
#
# 리눅스에서 Xvfb, xdotool, ffmpeg, gifsicle이 있어야 한다.
# 사용법: tools/record_demo.sh
set -eu
cd "$(dirname "$0")/.."

cargo build --release
work=target/demo
mkdir -p "$work" docs

# 쿼티 자판에서 누르는 글쇠를 프로그램에서 받아 온다. 섞지 않은 묶음의 첫 한글 문단과 첫 영문 문단이다.
# 한글도 두벌식 자리대로 영문 글쇠로 적혀 있다.
mapfile -t page < <(cargo run --quiet --release --example keys 2)
hangul=${page[0]}
english=${page[1]}

# 시연은 쓰는 사람의 기록을 건드리지 않도록 따로 둔 폴더에 기록한다.
# 결과 창이 지난 기록과 견주는 모습을 보이려고, 지어낸 1분 연습 기록 열두 개를 미리 넣어 둔다.
# 견주기는 연습 시간이 같은 기록끼리 하므로, 시연에서 하는 연습과 같은 1분이어야 한다.
export XDG_DATA_HOME="$PWD/$work/data"
mkdir -p "$XDG_DATA_HOME/keyboard_battle"
now=$(date +%s)
# 타수 `$1`타/분으로 `$2`초 동안, 천 번에 `$3`번꼴로 틀리며 친 기록.
tally() {
  local timed=$(($1 * $2 / 60))
  echo "$((timed + 1)),$((timed * 10 / 21)),$((timed + 1 + timed * $3 / 500)),$((timed * $3 / 1000)),$timed,$2"
}
n=0
for past in "300 52" "296 44" "311 47" "318 38" "309 55" "325 41" "331 36" "322 49" "338 33" "335 40" "347 31" "341 37"; do
  set -- $past
  at=$((now - (12 - n) * 70000))
  echo "at=$at minutes=1 total=$(tally "$1" 60 "$2")" \
    "hangul=$(tally $(($1 * 102 / 100)) 46 "$2") english=$(tally $(($1 * 94 / 100)) 14 "$2")" \
    "rust=0,0,0,0,0,0"
  n=$((n + 1))
done >"$XDG_DATA_HOME/keyboard_battle/records.txt"

export DISPLAY=:94
Xvfb "$DISPLAY" -screen 0 960x700x24 >/dev/null 2>&1 &
xvfb=$!
sleep 1
# 대본이 치는 글이 나오도록, 문장을 섞지 않고 실린 순서대로 내게 한다.
target/release/keyboard_battle --in-order >"$work/app.log" 2>&1 &
app=$!
trap 'kill "$app" "$xvfb" 2>/dev/null || true' EXIT

# 창이 화면에 나타나면 글쇠 입력을 그 창으로 보낸다.
window=""
for _ in $(seq 1 40); do
  window=$(xdotool search --onlyvisible --class keyboard_battle 2>/dev/null | head -1 || true)
  [ -n "$window" ] && xdotool windowfocus "$window" 2>/dev/null && break
  sleep 0.5
done
[ -n "$window" ] || { echo "창이 뜨지 않았습니다. $work/app.log 를 보세요." >&2; exit 1; }
sleep 2
xdotool mousemove 600 680 click 1
sleep 0.5

ffmpeg -y -loglevel error -f x11grab -draw_mouse 0 -framerate 20 -video_size 960x700 -i "$DISPLAY" \
  -c:v libx264 -preset ultrafast -pix_fmt yuv444p -crf 10 "$work/screen.mp4" &
recorder=$!
sleep 1.6

# 재 보면 글쇠 사이가 적은 값의 절반쯤이다: 0.15초쯤, 1분에 400타쯤.
keys() { printf '%s' "$1" | xdotool type --delay 300 --file -; }
erase() {
  for _ in $(seq 1 "$1"); do
    xdotool key BackSpace
    sleep 0.2
  done
}
# `rest`의 앞에서 `$1`번째 빈칸까지를 친다. 빈칸 뒤에서 끊으므로 한글 조합이 깨지지 않는다.
keys_to_space() {
  local count=$1 part=""
  while [ "$count" -gt 0 ]; do
    part+="${rest%% *} "
    rest="${rest#* }"
    count=$((count - 1))
  done
  keys "$part"
}

# 연습 시간을 묻는 화면에서 1분을 넣는다. 그냥 Enter를 누르면 3분이지만 시연으로는 길다.
xdotool key 1
sleep 0.8
xdotool key Return
sleep 0.8

# 한글 문단. '쳇'의 첫소리를 틀렸다가 고친다. 조합 중인 글자는 낱자씩 지워진다.
rest=$hangul
keys_to_space 2
keys 'wp'
sleep 0.9
erase 2
sleep 0.3
keys_to_space 7
# 틀린 줄 모르고 몇 글쇠를 더 쳤다가, Esc로 틀린 데까지 한 번에 지운다.
keys 'ekfrrhcl, Wkq'
sleep 0.9
xdotool key Escape
sleep 0.5
# 문단의 나머지. 맞게 친 '닭'(ekfr)은 남아 있으므로 그다음부터 친다.
# 다 치면 아래에 미리 보이던 영문 문단이 올라온다.
keys "${rest#ekfr}"
sleep 0.4

# 영문 문단. 치는 도중에 1분이 다 된다. 그 뒤에 이어서 친 글쇠는 프로그램이 받지 않는다.
rest=$english
keys_to_space 20

# 결과 창을 보고, 다시 보기를 잠깐 보다가 그만둔다.
sleep 5.0
xdotool key v
sleep 4.0
xdotool key Escape
sleep 2.0

# 시간 바꾸기로 돌아가서, 오른쪽 위의 "밝게"를 눌러 밝은 화면으로 바꾼다.
xdotool key t
sleep 1.2
xdotool mousemove 657 22 click 1
sleep 0.2
xdotool mousemove 600 680
sleep 2.6

kill -INT "$recorder"
wait "$recorder" 2>/dev/null || true

# 어두운 화면과 밝은 화면이 다 나오므로, 색은 녹화 전체에서 고른다.
ffmpeg -y -loglevel error -i "$work/screen.mp4" \
  -vf "fps=10,palettegen=max_colors=256:stats_mode=full:reserve_transparent=0" "$work/palette.png"
ffmpeg -y -loglevel error -i "$work/screen.mp4" -i "$work/palette.png" \
  -lavfi "fps=10[x];[x][1:v]paletteuse=dither=none:diff_mode=rectangle" "$work/demo.gif"
gifsicle -O3 "$work/demo.gif" -o docs/demo.gif
ls -l docs/demo.gif
