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
wasm-bindgen --target web --no-typescript \
  --remove-name-section --remove-producers-section \
  --out-dir target/web-dist \
  target/wasm32-unknown-unknown/web/keyboard_battle.wasm
cp web/index.html target/web-dist/

echo "target/web-dist 에 만들었습니다."
echo "열어 보려면: python3 -m http.server --directory target/web-dist"
