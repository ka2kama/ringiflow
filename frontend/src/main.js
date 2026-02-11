/**
 * RingiFlow JavaScript エントリポイント
 *
 * このファイルは Elm アプリケーションの初期化と、
 * JavaScript 側の設定を担当する。
 *
 * ## ファイル構成
 *
 * ```
 * index.html
 *    └── main.js (このファイル)
 *           └── Main.elm (Elm アプリケーション)
 * ```
 *
 * ## なぜ JavaScript エントリポイントが必要か
 *
 * 1. **Flags の注入**: Elm は純粋関数型なので、環境変数や
 *    現在時刻などの外部情報を直接取得できない。
 *    JavaScript 側で取得して Flags として渡す。
 *
 * 2. **Ports の設定**: Elm と JavaScript 間の通信チャネルを設定。
 *
 * 3. **DOM マウント**: Elm アプリケーションをどの DOM 要素に
 *    描画するかを指定。
 *
 * ## vite-plugin-elm について
 *
 * このプロジェクトでは vite-plugin-elm を使用。
 * `.elm` ファイルを直接 import でき、Vite が自動的に
 * Elm コンパイラを呼び出してコンパイルする。
 *
 * 代替案:
 * - elm-webpack-loader: Webpack 用（Vite より設定が複雑）
 * - parcel: ゼロ設定だが、Elm サポートが限定的
 */

import "./styles.css";
import { Elm } from "./Main.elm";

/**
 * 開発用認証バイパス（DevAuth）のセットアップ
 *
 * 開発環境でログイン画面なしに認証済み状態を実現する。
 * 詳細: docs/06_ナレッジベース/security/DevAuth.md
 *
 * ## 動作条件
 *
 * - 開発環境（import.meta.env.DEV が true）またはデモ環境（VITE_DEV_AUTH=true）で有効
 * - BFF 側で DEV_AUTH_ENABLED=true が設定されている必要あり
 */
if (import.meta.env.DEV || import.meta.env.VITE_DEV_AUTH === "true") {
  // 開発用セッション Cookie を設定
  // BFF の dev_auth.rs で定義されている DEV_SESSION_ID と一致させる
  document.cookie = "session_id=dev-session; path=/";
}

/**
 * Elm アプリケーションの初期化
 *
 * ## Elm.Main.init の引数
 *
 * - `node`: Elm がレンダリングする DOM 要素
 *   - null を渡すと document.body 全体を使用
 *   - 特定の要素を渡すとその中にレンダリング
 *
 * - `flags`: Elm の init 関数に渡される初期データ
 *   - 任意の JSON シリアライズ可能な値
 *   - Elm 側で Flags 型として受け取る
 *
 * ## 戻り値の app オブジェクト
 *
 * `app` オブジェクトは Ports へのアクセスを提供:
 * - `app.ports.sendMessage.subscribe()`: Elm からの送信を購読
 * - `app.ports.receiveMessage.send()`: Elm へメッセージを送信
 */
const app = Elm.Main.init({
  /**
   * マウント先 DOM 要素
   *
   * index.html の <div id="app"></div> を指定。
   * Elm はこの要素の中身を完全に制御する。
   */
  node: document.getElementById("app"),

  /**
   * Flags: JavaScript から Elm への初期データ
   *
   * ## 設計意図
   *
   * Flags は Elm アプリケーション起動時に一度だけ渡される。
   * ランタイム中に変更が必要なデータは Ports を使用する。
   *
   * ## フィールド説明
   *
   * - apiBaseUrl: API サーバーのベース URL
   *   - 開発環境: 空文字（Vite のプロキシ機能を使用）
   *   - 本番環境: 実際の API URL（例: "https://api.ringiflow.com"）
   *   - 環境変数 VITE_API_BASE_URL で設定
   *
   * - timestamp: アプリケーション起動時刻
   *   - キャッシュバスティング
   *   - セッション開始時刻の記録
   *   - デバッグ用タイムスタンプ
   *
   * ## Vite の環境変数について
   *
   * `import.meta.env.VITE_*` で環境変数にアクセス可能。
   * VITE_ プレフィックスがないと、セキュリティ上の理由で
   * クライアントに公開されない。
   *
   * 設定方法:
   * - .env ファイル（ローカル開発用、.gitignore に含める）
   * - CI/CD 環境変数（本番デプロイ用）
   */
  flags: {
    apiBaseUrl: import.meta.env.VITE_API_BASE_URL || "",
    timestamp: Date.now(),
    // JavaScript の getTimezoneOffset() は UTC - ローカル（分）を返す（JST なら -540）
    // Elm の Time.customZone はローカル - UTC（分）を期待するので符号を反転する
    timezoneOffsetMinutes: -new Date().getTimezoneOffset(),
  },
});

/**
 * Ports の設定（将来の拡張用）
 *
 * ## 使用例: Elm からの通知を受け取る
 *
 * ```javascript
 * app.ports.sendMessage.subscribe((data) => {
 *   console.log("Received from Elm:", data);
 *
 *   // メッセージタイプに基づいて処理を分岐
 *   switch (data.type) {
 *     case "SHOW_NOTIFICATION":
 *       showNotification(data.payload.message);
 *       break;
 *
 *     case "STORE_TOKEN":
 *       localStorage.setItem("auth_token", data.payload.token);
 *       break;
 *
 *     case "NAVIGATE_EXTERNAL":
 *       window.location.href = data.payload.url;
 *       break;
 *
 *     default:
 *       console.warn("Unknown message type:", data.type);
 *   }
 * });
 * ```
 *
 * ## 使用例: JavaScript から Elm へ通知を送る
 *
 * ```javascript
 * // ユーザー認証完了時
 * app.ports.receiveMessage.send({
 *   v: 1,
 *   type: "AUTH_SUCCESS",
 *   payload: { userId: "123", name: "Alice" },
 *   correlationId: crypto.randomUUID(),
 *   ts: Date.now()
 * });
 *
 * // WebSocket メッセージ受信時
 * websocket.onmessage = (event) => {
 *   app.ports.receiveMessage.send({
 *     v: 1,
 *     type: "WS_MESSAGE",
 *     payload: JSON.parse(event.data),
 *     correlationId: crypto.randomUUID(),
 *     ts: Date.now()
 *   });
 * };
 * ```
 *
 * ## エラーハンドリング
 *
 * Elm の Ports はエラーを伝播しない設計のため、
 * JavaScript 側で適切にエラーハンドリングを行う:
 *
 * ```javascript
 * app.ports.sendMessage.subscribe((data) => {
 *   try {
 *     // 処理
 *   } catch (error) {
 *     console.error("Port handler error:", error);
 *     // 必要に応じて Elm にエラーを通知
 *     app.ports.receiveMessage.send({
 *       v: 1,
 *       type: "JS_ERROR",
 *       payload: { message: error.message },
 *       correlationId: data.correlationId,
 *       ts: Date.now()
 *     });
 *   }
 * });
 * ```
 */

/**
 * beforeunload イベント制御（Ports: setBeforeUnloadEnabled）
 *
 * フォーム入力中にタブ閉じやリロードを行った場合に、
 * ブラウザの警告ダイアログを表示してデータ損失を防ぐ。
 *
 * Elm 側で isDirty 状態が変わるたびに呼ばれる:
 * - true: beforeunload リスナーを登録
 * - false: beforeunload リスナーを解除
 */
/**
 * モーダルダイアログ表示（Ports: showModalDialog）
 *
 * Elm が <dialog> 要素を DOM に追加した後、showModal() で
 * モーダル表示する。showModal() はフォーカストラップ、
 * ESC キー処理、::backdrop 表示をブラウザネイティブで提供する。
 *
 * requestAnimationFrame を使用する理由:
 * Elm の Virtual DOM 更新が完了してから showModal() を呼ぶ必要がある。
 * Port コマンドは DOM パッチ後に実行されるが、安全マージンとして
 * 次フレームまで遅延させる。
 */
if (app.ports.showModalDialog) {
  app.ports.showModalDialog.subscribe((dialogId) => {
    requestAnimationFrame(() => {
      const dialog = document.getElementById(dialogId);
      if (dialog && !dialog.open) {
        dialog.showModal();
      }
    });
  });
}

if (app.ports.setBeforeUnloadEnabled) {
  let beforeUnloadHandler = null;

  app.ports.setBeforeUnloadEnabled.subscribe((enabled) => {
    if (enabled && !beforeUnloadHandler) {
      beforeUnloadHandler = (e) => {
        e.preventDefault();
        // モダンブラウザでは returnValue の設定が必要
        e.returnValue = "";
      };
      window.addEventListener("beforeunload", beforeUnloadHandler);
    } else if (!enabled && beforeUnloadHandler) {
      window.removeEventListener("beforeunload", beforeUnloadHandler);
      beforeUnloadHandler = null;
    }
  });
}
