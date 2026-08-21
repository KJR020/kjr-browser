// withGlobalTauri: true により window.__TAURI__ 経由で API を使う (バンドラ不要)
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const address = document.getElementById("address");

async function call(command, args) {
  try {
    await invoke(command, args);
  } catch (err) {
    console.error(`${command} failed:`, err);
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

// コンテンツ側のページ遷移が完了したらアドレスバーを同期する
listen("url-changed", (event) => {
  if (document.activeElement !== address) {
    address.value = event.payload;
  }
});
