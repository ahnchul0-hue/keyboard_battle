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

export DISPLAY=:94
Xvfb "$DISPLAY" -screen 0 960x700x24 >/dev/null 2>&1 &
xvfb=$!
sleep 1
# 대본이 치는 문장이 나오도록, 문장을 섞지 않고 실린 순서대로 내게 한다.
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
sleep 1.2

# 쿼티 자판에서 누르는 글쇠를 적는다. 한글 문장도 두벌식 자리대로 영문 글쇠로 적는다.
# 재 보면 글쇠 사이가 적은 값의 절반쯤이다: 0.15초쯤, 1분에 400타쯤.
keys() { xdotool type --delay 300 "$1"; }
erase() {
  for _ in $(seq 1 "$1"); do
    xdotool key BackSpace
    sleep 0.2
  done
}

# 다람쥐 헌 쳇바퀴에 타고파 — '쳇'의 첫소리를 틀렸다가 고친다. 조합 중인 글자는 낱자씩 지워진다.
keys 'ekfkawnl gjs '
keys 'wp'
sleep 0.9
erase 2
sleep 0.3
keys 'cptqkznldp xkrhvk'
sleep 0.5
# 얘, 옛 학생은 값싼 떡볶이와 닭꼬치, 짭짤한 빵을 좋아했다.
keys 'dO, dPt gkrtoddms rkqtTks EjrqhRdldhk ekfrRhcl, WkqWkfgks Qkddmf whgdkgoTek.'
sleep 0.3
# 영문 — 낱말 하나를 틀렸다가 고친다.
keys 'The quick brpwn'
sleep 0.5
erase 3
sleep 0.1
keys 'own fox jumps over the lazy dog.'
sleep 0.3
keys 'let v: Vec<u8> = vec![1, 2]; println!("{:?}", &v);'

# 결과 창을 보고, 다시 보기를 잠깐 보다가 그만둔다.
sleep 4.2
xdotool key v
sleep 4.6
xdotool key Escape
sleep 1.4

# 다음 과정: 가는 말이 고와야 오…
xdotool key Return
sleep 1.0
keys 'rksms akfdl rhdhkdi dh'
sleep 0.5

# 오른쪽 위의 "밝게"를 눌러 밝은 화면으로 바꾼다.
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
