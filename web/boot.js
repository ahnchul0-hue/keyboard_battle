// 키보드 배틀을 브라우저에 띄운다.
//
// 단계마다 무엇을 하는 중인지 화면에 적고, 막히면 어느 단계에서 왜 막혔는지를 적는다.
// 다른 사람의 브라우저나 페이지를 가둬 두는 틀 안처럼, 직접 열어 볼 수 없는 곳에서 생긴 문제를
// 찾을 수 있게 하려는 것이다. 모듈을 쓰지 않는 스크립트로 두어, 가장 까다로운 틀에서도 돌게 한다.
//
// 페이지는 이 스크립트보다 앞에 `window.KEYBOARD_BATTLE = { build, script, wasm, wasmBytes }`를 둘 수 있다.
// 페이지에는 `id="screen"`인 canvas와 `id="note"`인 안내 문구가 있어야 한다.
(function () {
  "use strict";

  var config = window.KEYBOARD_BATTLE || {};
  var note = document.getElementById("note");
  var began = Date.now();

  /** 문제를 알릴 때 함께 보낼 내용. 이 페이지가 어디까지 갔고 어떤 브라우저에서 열렸는지다. */
  var report = {
    build: config.build || "표시 없음",
    at: new Date().toISOString(),
    status: "loading",
    step: "",
    steps: [],
    error: "",
    seen: [],
  };

  function describe(error) {
    if (error && error.message) return (error.name || "Error") + ": " + error.message;
    return String(error);
  }

  /** 이 브라우저에서 무엇이 되는지 알아본다. 하나가 안 되더라도 나머지는 알아본다. */
  function lookAround() {
    var facts = {
      agent: navigator.userAgent,
      view: window.innerWidth + "x" + window.innerHeight + " @" + window.devicePixelRatio,
      webgpu: !!navigator.gpu,
    };
    try {
      facts.framed = window.top !== window.self;
    } catch (error) {
      facts.framed = true;
    }
    facts.host = !!(window.claude && window.claude.use);
    try {
      var gl = document.createElement("canvas").getContext("webgl2");
      facts.webgl2 = !!gl;
      if (gl) {
        var info = gl.getExtension("WEBGL_debug_renderer_info");
        facts.renderer = String(gl.getParameter(info ? info.UNMASKED_RENDERER_WEBGL : gl.RENDERER));
        var lose = gl.getExtension("WEBGL_lose_context");
        if (lose) lose.loseContext();
      }
    } catch (error) {
      facts.webgl2 = false;
      facts.renderer = "확인 실패: " + describe(error);
    }
    try {
      window.localStorage.setItem("keyboard_battle.probe", "1");
      window.localStorage.removeItem("keyboard_battle.probe");
      facts.storage = "됨";
    } catch (error) {
      facts.storage = "안 됨: " + describe(error);
    }
    // 페이지 규칙이 WebAssembly를 막는지는 가장 작은 모듈을 만들어 보면 안다.
    try {
      new WebAssembly.Module(new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0]));
      facts.wasm = "됨";
    } catch (error) {
      facts.wasm = "안 됨: " + describe(error);
    }
    return facts;
  }

  // ---- 화면의 안내 문구 ----

  var stepBegan = Date.now();
  var detail = "";

  function show(text, failed) {
    if (!note) return;
    note.hidden = false;
    note.classList.toggle("failed", !!failed);
    note.textContent = text;
  }

  function showStep() {
    var waited = Math.round((Date.now() - stepBegan) / 1000);
    var slow = waited >= 8 ? " (" + waited + "초째)" : "";
    show(report.step + "…" + (detail ? " " + detail : "") + slow, false);
  }

  function step(name) {
    report.step = name;
    report.steps.push(name + " +" + (Date.now() - began) + "ms");
    stepBegan = Date.now();
    detail = "";
    showStep();
    record();
  }

  // 한 단계가 오래 걸리면 몇 초째인지 적어, 멈춘 것인지 느린 것인지 알 수 있게 한다.
  var ticker = window.setInterval(function () {
    if (report.status === "loading") showStep();
  }, 1000);

  function reportText() {
    var lines = ["키보드 배틀 웹 버전 진단"];
    Object.keys(report).forEach(function (key) {
      var value = report[key];
      lines.push(key + ": " + (Array.isArray(value) ? value.join(" | ") : value));
    });
    return lines.join("\n");
  }

  /** 띄우지 못한 까닭을 적고, 그 내용을 복사할 단추를 둔다. */
  function showFailure(headline) {
    if (!note) return;
    note.hidden = false;
    note.classList.add("failed");
    note.textContent = "";
    var text = [headline, "막힌 곳: " + report.step, "까닭: " + report.error, "판: " + report.build];
    text.forEach(function (line) {
      var paragraph = document.createElement("span");
      paragraph.textContent = line;
      note.appendChild(paragraph);
      note.appendChild(document.createElement("br"));
    });
    var copy = document.createElement("button");
    copy.type = "button";
    copy.id = "copy-report";
    copy.textContent = "진단 내용 복사";
    copy.addEventListener("click", function () {
      var all = reportText();
      var done = function () {
        copy.textContent = "복사했습니다";
      };
      var fallback = function () {
        // 복사를 막는 틀에서는 내용을 펼쳐 보여 주고 골라 둔다.
        var box = document.createElement("textarea");
        box.id = "report-text";
        box.readOnly = true;
        box.rows = 8;
        box.value = all;
        note.appendChild(box);
        box.focus();
        box.select();
        copy.textContent = "아래 내용을 복사해 주세요";
      };
      try {
        navigator.clipboard.writeText(all).then(done, fallback);
      } catch (error) {
        fallback();
      }
    });
    note.appendChild(copy);
  }

  function fail(error) {
    if (report.status === "failed") return;
    report.status = "failed";
    report.error = describe(error);
    window.clearInterval(ticker);
    showFailure("이 창에서는 프로그램을 띄우지 못했습니다.");
    record();
  }

  // ---- 진단 내용 남기기 ----
  //
  // 페이지가 Claude의 아티팩트 보기 안에서 열렸고 그 저장소를 쓸 수 있으면, 진단 내용을 거기에 남긴다.
  // 그러면 화면의 문구를 옮겨 적지 않아도 무슨 일이 있었는지 알 수 있다. 그 밖의 곳에서는 아무 데도 보내지 않는다.

  var store;
  var entry = null;
  var writing = Promise.resolve();

  function hostStore() {
    if (!store) {
      var host = window.claude;
      store =
        host && typeof host.use === "function"
          ? Promise.resolve(host.use("db")).catch(function () {
              return null;
            })
          : Promise.resolve(null);
    }
    return store;
  }

  function record() {
    var copy = JSON.parse(JSON.stringify(report));
    writing = writing
      .then(hostStore)
      .then(function (db) {
        if (!db) return undefined;
        if (!entry) entry = db.collection("diagnostics").doc();
        return entry.set(copy);
      })
      .catch(function (error) {
        report.recorded = "못 남김: " + describe(error && error.code ? error.code : error);
      });
  }

  // ---- 프로그램 불러오기 ----

  function loadScript(source) {
    return new Promise(function (resolve, reject) {
      var script = document.createElement("script");
      script.src = source;
      script.onload = resolve;
      script.onerror = function () {
        reject(new Error(source + " 파일을 불러오지 못했습니다"));
      };
      document.head.appendChild(script);
    });
  }

  function megabytes(bytes) {
    return (bytes / 1048576).toFixed(1);
  }

  /** 프로그램 파일을 내려받는다. 얼마나 받았는지 화면에 적는다. */
  async function download(source, expected) {
    var response = await fetch(source);
    if (!response.ok) throw new Error(source + " 응답이 " + response.status + "입니다");
    var bytes;
    if (response.body && response.body.getReader) {
      var reader = response.body.getReader();
      var parts = [];
      var received = 0;
      for (;;) {
        var part = await reader.read();
        if (part.done) break;
        parts.push(part.value);
        received += part.value.length;
        detail = megabytes(received) + (expected ? " / " + megabytes(expected) : "") + "MB";
        showStep();
      }
      bytes = new Uint8Array(received);
      var at = 0;
      parts.forEach(function (chunk) {
        bytes.set(chunk, at);
        at += chunk.length;
      });
    } else {
      bytes = new Uint8Array(await response.arrayBuffer());
    }
    // 오류 페이지 같은 엉뚱한 것을 받았으면 여기서 알아차린다.
    var magic = [0, 97, 115, 109];
    var isProgram = magic.every(function (byte, index) {
      return bytes[index] === byte;
    });
    if (!isProgram) {
      var head = String.fromCharCode.apply(null, Array.prototype.slice.call(bytes, 0, 60));
      throw new Error("받은 파일이 프로그램이 아닙니다 (" + bytes.length + "바이트, 앞부분: " + head + ")");
    }
    report.wasmBytes = bytes.length;
    return bytes;
  }

  /** 프로그램이 화면을 열었는지, 못 열었는지 알려 올 때까지 기다린다. */
  var announce;
  var announced = new Promise(function (resolve) {
    announce = resolve;
  });

  window.keyboardBattleStatus = function (kind, text) {
    if (kind === "panic") {
      // 프로그램이 돌다가 멈췄다. 이미 떠 있던 화면 위에 까닭을 적는다.
      report.status = "panic";
      report.error = String(text);
      window.clearInterval(ticker);
      showFailure("프로그램이 멈췄습니다. 페이지를 새로 고치면 다시 시작합니다.");
      record();
    }
    announce({ kind: kind, text: String(text) });
  };

  function remember(text) {
    if (report.seen.length < 5) report.seen.push(String(text).slice(0, 300));
  }
  window.addEventListener("error", function (event) {
    remember(event.message || event.error);
  });
  window.addEventListener("unhandledrejection", function (event) {
    remember(describe(event.reason));
  });

  async function boot() {
    Object.assign(report, lookAround());
    step("브라우저 확인하는 중");
    if (report.wasm !== "됨") {
      throw new Error("이 창은 WebAssembly 실행을 막고 있습니다 (" + report.wasm + ")");
    }
    if (!report.webgl2) {
      throw new Error("이 브라우저에서 WebGL2를 쓸 수 없습니다 (" + (report.renderer || "지원 안 함") + ")");
    }

    step("스크립트 불러오는 중");
    await loadScript(config.script || "./keyboard_battle.js");
    if (typeof wasm_bindgen !== "function") {
      throw new Error("스크립트는 받았지만 프로그램을 여는 함수가 없습니다");
    }

    step("프로그램 내려받는 중");
    var bytes = await download(config.wasm || "./keyboard_battle_bg.wasm", config.wasmBytes);

    step("프로그램 준비하는 중");
    await wasm_bindgen({ module_or_path: bytes });

    step("화면 여는 중");
    var answer = await announced;
    if (answer.kind !== "started") {
      throw new Error(answer.text || "화면을 열지 못했습니다");
    }

    report.status = "started";
    report.step = "떴음";
    report.steps.push("떴음 +" + (Date.now() - began) + "ms");
    window.clearInterval(ticker);
    if (note) note.hidden = true;
    var canvas = document.getElementById("screen");
    if (canvas) canvas.focus();
    record();
  }

  boot().catch(fail);
})();
