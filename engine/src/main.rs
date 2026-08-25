//! kjr-engine のエントリーポイント: ウィンドウとイベントループ。
//!
//! GUI アプリケーションの正体は「イベントを待つ → 処理する → 必要なら描き直す」
//! を繰り返す無限ループ (イベントループ)。ブラウザも例外ではなく、
//! Chrome のメッセージループも構造はこれと同じ。
//!
//! Phase 2 までのパイプライン:
//!
//!   SAMPLE_HTML (テキスト)
//!        ↓ html::parse()        … パース (文字列 → 構造)
//!   DOM ツリー
//!        ↓ render::render()     … UA スタイル + 縦積みレイアウト (Phase 3/4 で分離予定)
//!   ディスプレイリスト
//!        ↓ paint::paint()       … ラスタライズ (図形 → ピクセル)
//!   ピクセルバッファ
//!        ↓ softbuffer           … OS への転送
//!   画面
//!
//! モジュール構成 (レンダリングパイプラインの工程名に対応):
//! - html.rs         … HTML パーサ (テキスト → DOM)
//! - dom.rs          … DOM ツリーの定義
//! - render.rs       … DOM → ディスプレイリスト (Phase 2 の仮実装)
//! - display_list.rs … 描画コマンドの定義 (パイプラインの中間表現)
//! - paint.rs        … ラスタライズ (図形 → ピクセル)
//! - text.rs         … フォント処理 (文字 → グリフ画像 → ピクセル)

mod display_list;
mod dom;
mod html;
mod paint;
mod render;
mod text;

use std::num::NonZeroU32;
use std::rc::Rc;

use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

use dom::Node;
use text::FontStack;

/// Phase 2 のデモページ。この文字列がパース → DOM → 描画コマンド → ピクセル
/// と姿を変えて画面に届く。書き換えて cargo run すればそのまま反映される
const SAMPLE_HTML: &str = r#"<!DOCTYPE html>
<html>
<body>
  <h1>kjr-engine Phase 2</h1>
  <!-- この行はコメントなので画面に出ない -->
  <p>この画面は <strong>HTML 文字列</strong> をパースして描画している。</p>
  <hr>
  <h2>できるようになったこと</h2>
  <ul>
    <li>タグと属性のパース (再帰下降)</li>
    <li>DOM ツリーの構築 (起動ログにダンプが出る)</li>
    <li>DOM からディスプレイリストへの変換</li>
  </ul>
  <hr>
  <p>次の Phase 3 では CSS を当てて、この見た目を変えられるようにする。</p>
</body>
</html>"#;

/// アプリの状態。ウィンドウや描画先はイベントループ開始後に
/// 作られる (resumed で初期化される) ため Option で持つ
struct App {
    window: Option<Rc<Window>>,
    surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
    fonts: FontStack,
    /// 表示中のページの DOM。リサイズのたびにここから描画し直す
    dom: Node,
}

impl App {
    /// 画面を描き直す。「毎回まっさらなピクセル配列を作って丸ごと転送する」
    /// という最も素朴な方式 (本物のブラウザは差分だけ再描画して省力化している)
    fn redraw(&mut self) {
        let (Some(window), Some(surface)) = (&self.window, &mut self.surface) else {
            return;
        };
        let size = window.inner_size();
        let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
        else {
            return; // 最小化などで大きさが 0 のときは描かない
        };

        // 1. DOM からディスプレイリストを作る (ウィンドウ幅に合わせて毎回作り直す)
        let commands = render::render(&self.dom, size.width as f32, size.height as f32);

        // 2. ディスプレイリストをラスタライズしてピクセル画像を得る
        let pixmap = paint::paint(&commands, &self.fonts, size.width, size.height);

        // 3. ウィンドウのピクセルバッファへ転送する
        surface.resize(w, h).expect("surface resize");
        let mut buffer = surface.buffer_mut().expect("get frame buffer");
        paint::copy_to_buffer(&pixmap, &mut buffer);
        buffer.present().expect("present frame");
    }
}

impl ApplicationHandler for App {
    /// イベントループが始まった (またはモバイルで復帰した) ときに 1 度呼ばれる。
    /// ウィンドウはここで作るのが winit 0.30 の流儀
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Rc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("kjr-engine")
                        .with_inner_size(LogicalSize::new(900.0, 600.0)),
                )
                .expect("create window"),
        );
        // softbuffer: GPU を使わずウィンドウにピクセルを書き込むための道具。
        // Context がディスプレイへの接続、Surface がウィンドウ 1 枚ぶんの描画先
        let context = softbuffer::Context::new(window.clone()).expect("softbuffer context");
        let surface =
            softbuffer::Surface::new(&context, window.clone()).expect("softbuffer surface");
        self.window = Some(window);
        self.surface = Some(surface);
    }

    /// ウィンドウに何かが起きるたびに呼ばれる (クリック、リサイズ、再描画要求…)。
    /// ここが「イベントを待つ → 処理する」の本体
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            // 閉じるボタン → ループを抜けてアプリ終了
            WindowEvent::CloseRequested => event_loop.exit(),
            // リサイズ → OS に「描き直したい」と伝える (直接は描かない)
            WindowEvent::Resized(_) => {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            // OS からの「今描いてよい」の合図。描画は必ずここで行う
            WindowEvent::RedrawRequested => self.redraw(),
            _ => {}
        }
    }
}

fn main() {
    // パース結果の DOM ツリーを標準出力にダンプする。
    // HTML がどう木になったかを目で確認できる (Phase 2 の学習ポイント)
    let dom = html::parse(SAMPLE_HTML);
    println!("--- DOM ツリー ---\n{}", dom.dump(0));

    let fonts = FontStack::load_system_fonts().expect("load fonts");
    let event_loop = EventLoop::new().expect("create event loop");
    // Wait = イベントが来るまで眠る (アニメーションしないなら CPU を使わない)
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App { window: None, surface: None, fonts, dom };
    event_loop.run_app(&mut app).expect("run event loop");
}
