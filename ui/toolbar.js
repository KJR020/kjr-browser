// ツールバー UI のロジック。
// タブバー・ナビゲーションボタン・アドレスバーの操作を Rust 側の
// IPC コマンド (src-tauri/src/commands.rs) に橋渡しし、Rust 側からの
// tabs-changed イベントを受けてタブバーを再描画する。
// withGlobalTauri: true により window.__TAURI__ 経由で API を使う (バンドラ不要)
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const address = document.getElementById("address");
const tabsEl = document.getElementById("tabs");

async function call(command, args) {
  try {
    await invoke(command, args);
  } catch (err) {
    console.error(`${command} failed:`, err);
  }
}

// タブチップに表示する短いラベル
function tabTitle(url) {
  if (!url || url === "about:blank") return "新しいタブ";
  try {
    const u = new URL(url);
    return u.host + (u.pathname !== "/" ? u.pathname : "");
  } catch {
    return url;
  }
}

function renderTabs(tabs) {
  tabsEl.textContent = "";
  for (const tab of tabs) {
    const chip = document.createElement("div");
    chip.className = "tab" + (tab.active ? " active" : "");
    chip.addEventListener("click", () => call("switch_tab", { label: tab.label }));

    const title = document.createElement("span");
    title.className = "title";
    title.textContent = tabTitle(tab.url);
    chip.appendChild(title);

    const close = document.createElement("button");
    close.className = "close";
    close.textContent = "×";
    close.title = "タブを閉じる";
    close.addEventListener("click", (e) => {
      e.stopPropagation();
      call("close_tab", { label: tab.label });
    });
    chip.appendChild(close);

    tabsEl.appendChild(chip);

    if (tab.active && document.activeElement !== address) {
      address.value = tab.url === "about:blank" ? "" : tab.url;
    }
  }
}

address.addEventListener("keydown", (e) => {
  if (e.key === "Enter" && address.value.trim() !== "") {
    call("navigate", { url: address.value });
    address.blur();
  }
});

document.getElementById("back").addEventListener("click", () => call("go_back"));
document.getElementById("forward").addEventListener("click", () => call("go_forward"));
document.getElementById("reload").addEventListener("click", () => call("reload"));
document.getElementById("new-tab").addEventListener("click", () => call("new_tab"));

// Rust 側からのタブ状態通知 (作成・切替・閉じる・ページ遷移完了で発火)
listen("tabs-changed", (event) => renderTabs(event.payload));

// 初期表示: listen 登録前に作られたタブ (起動時の最初のタブ) を取得して描画
invoke("list_tabs").then(renderTabs).catch(console.error);
