#!/bin/sh
# 브라우저용 묶음을 target/web-dist에 만든다.
#
# 묶는 도구(wasm-bindgen)는 Cargo.lock에 적힌 wasm-bindgen과 버전이 같아야 하므로,
# 없거나 버전이 다르면 여기서 맞는 버전을 설치한다.
set -eu
cd "$(dirname "$0")/.."

version=$(cargo pkgid wasm-bindgen | sed 's/.*[@#]//')
if [ "$(wasm-bindgen --version 2>/dev/null || true)" != "wasm-bindgen $version" ]; then
  cargo install wasm-bindgen-cli --locked --force --version "$version"
fi

cargo build --profile web --target wasm32-unknown-unknown
rm -rf target/web-dist
# 모듈을 쓰지 않는 스크립트로 낸다. 페이지를 가둬 두는 틀 가운데에는 모듈 스크립트를 받지 못하는 것이 있다.
wasm-bindgen --target no-modules --no-typescript \
  --remove-name-section --remove-producers-section \
  --out-dir target/web-dist \
  target/wasm32-unknown-unknown/web/keyboard_battle.wasm

# 어느 빌드가 떠 있는지 페이지가 알 수 있게, 커밋과 만든 때, 프로그램 파일의 크기를 적어 넣는다.
build="$(git rev-parse --short HEAD 2>/dev/null || echo 커밋-모름) $(date -u +%Y-%m-%dT%H:%MZ)"
bytes=$(wc -c <target/web-dist/keyboard_battle_bg.wasm | tr -d ' ')
sed -e "s/__BUILD__/$build/" -e "s/__WASM_BYTES__/$bytes/" web/index.html >target/web-dist/index.html
cp web/boot.js target/web-dist/

echo "target/web-dist 에 만들었습니다."
echo "열어 보려면: python3 -m http.server --directory target/web-dist"
