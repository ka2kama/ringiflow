# Ghostty + Wayland で mprocs のキー入力がフリーズする問題を解決する

## いつ使うか

- Ghostty（Wayland）で `mprocs`（`just dev-all`）を起動すると、キー入力が一切効かなくなるとき
- プロセスの描画は更新されるが、Tab 切替・q 終了などの操作が不能なとき
- WezTerm や JetBrains IDE のターミナルでは再現しないとき

## 環境（確認時点: 2026-02-18）

| 項目 | バージョン |
|------|-----------|
| Ghostty | 1.2.3-1.fc43 (GTK, Wayland, io_uring) |
| mprocs | 0.8.3 |
| OS | Fedora 43, kernel 6.18.9 / 6.18.10 |
| DE | KDE Plasma 6 |

## 原因

Ghostty の GTK Wayland バックエンドにおけるキー入力転送のバグ。

mprocs は crossterm ライブラリを使用しており、crossterm が Kitty keyboard protocol のネゴシエーション（`supports_keyboard_enhancement` クエリ）を行う。Ghostty の Wayland 実装では、このネゴシエーションのレスポンスが正しく PTY に転送されず、crossterm のイベントループがブロックされる。

## ワークアラウンド

Ghostty（Wayland）から `just dev-all-x11` を実行する。これは別の Ghostty ウィンドウを X11 で起動し、その中で mprocs を動かす:

```bash
# justfile で定義済み
just dev-all-x11
# → GDK_BACKEND=x11 ghostty -e just dev-all
```

ワークフロー:
1. Desktop ランチャーから Ghostty を通常起動（Wayland）
2. Ghostty 内で `just dev-all-x11` を実行
3. 新しい X11 Ghostty ウィンドウが開き、mprocs が正常動作する

制約:
- **Ghostty から実行する必要がある**。WezTerm 等の別ターミナルからでは動作しない（親プロセスの環境変数の差異が影響する）
- Desktop ランチャーから直接 X11 で Ghostty を起動する方法（`.desktop` 変更）は、KDE Plasma 6 環境で GLib-CRITICAL エラーが発生し、mprocs のフリーズも解消しないため不採用

## 試行した対策と結果

| # | 対策 | 結果 | 備考 |
|---|------|------|------|
| 1 | `GDK_BACKEND=x11` で Desktop Ghostty 起動（`.desktop` 変更） | NG | GLib-CRITICAL エラー多発、mprocs フリーズ解消せず |
| 2 | `DBusActivatable=false`（`.desktop` 変更） | — | D-Bus 起動をバイパスするために必要だったが、#1 自体が不採用 |
| 3 | `GDK_DPI_SCALE=1.5`（`.desktop` 変更） | — | X11 での HiDPI 対策だが、#1 自体が不採用 |
| 4 | `async-backend = epoll`（Ghostty config） | NG | 単体でも #1 との組合せでも効果なし |
| 5 | `modify-other-keys = false`（Ghostty config） | NG | — |
| 6 | `TERM=xterm-256color`（環境変数） | NG | — |
| 7 | `term = xterm-256color`（Ghostty config） | NG | — |
| 8 | `GDK_BACKEND=x11 ghostty -e just dev-all`（Ghostty から） | **OK** | 唯一の動作するワークアラウンド |
| 9 | `GDK_BACKEND=x11 ghostty -e just dev-all`（WezTerm から） | NG | Ghostty 固有の環境変数が不足 |

### Desktop `.desktop` 変更が不採用な理由

Desktop ランチャーから X11 で Ghostty を起動しようとすると、2つの問題が発生する:

1. `DBusActivatable=true`（デフォルト）のまま `.desktop` の `Exec` を変更しても、KDE/GNOME は D-Bus 経由で起動するため `Exec` 行が無視される
2. `DBusActivatable=false` に変更して X11 起動に成功しても、KDE Plasma 6 との D-Bus 通信で `GLib-CRITICAL: g_variant_iter_loop: assertion 'g_variant_is_of_type' failed` が大量発生し、mprocs のフリーズも解消しない

### WezTerm から `dev-all-x11` が動かない理由

環境変数の差分が原因。Ghostty は子プロセスに以下を設定するが、WezTerm にはこれらがない:

| 変数 | Ghostty | WezTerm |
|------|---------|---------|
| `GHOSTTY_RESOURCES_DIR` | `/usr/share/ghostty` | なし |
| `GHOSTTY_SHELL_FEATURES` | `cursor,path,title` | なし |
| `TERM` | `xterm-ghostty` | `xterm-256color` |
| `TERMINFO` | `/usr/share/terminfo` | なし |

`GHOSTTY_RESOURCES_DIR` と `TERMINFO` を手動で設定しても解消しなかったため、これら以外の間接的な要因（プロセス起動経路、GTK 初期化順序等）が影響していると考えられる。

## なぜこの方法か

Wayland と X11 では GTK の入力処理パスが根本的に異なる:

- Wayland: Compositor → `wl_keyboard` プロトコル → GTK4 `GtkIMContext` → PTY
- X11（XWayland）: XKB キーイベント → GTK4 → PTY

`GDK_BACKEND=x11` は XWayland 経由で動作する。スケーリングやパフォーマンスで若干の差があるが、開発用途では実用上問題ない。普段のターミナル使用は Wayland のままで、mprocs を使うウィンドウだけ X11 にする運用が推奨。

## Ghostty への報告用テンプレート

```
Title: TUI app (mprocs/crossterm) keyboard input blocked under Wayland

Environment:
- Ghostty 1.2.3-1.fc43 (GTK, Wayland, io_uring)
- Fedora 43, KDE Plasma 6, kernel 6.18.10
- mprocs 0.8.3 (crossterm-based TUI)

Symptoms:
- mprocs TUI renders correctly, processes run normally
- All keyboard input is completely ignored
- Mouse input: untested

Reproduction:
1. Launch Ghostty on Wayland (default)
2. Run `mprocs` with any configuration
3. Keyboard input is completely unresponsive

Workarounds tried (none worked):
- modify-other-keys = false
- async-backend = epoll
- TERM=xterm-256color
- term = xterm-256color
- GDK_BACKEND=x11 via .desktop entry (GLib-CRITICAL errors on KDE Plasma 6)

Working workaround:
- GDK_BACKEND=x11 ghostty -e <command> (launched from within a Ghostty terminal)

Not reproducible in:
- WezTerm
- JetBrains IDE terminal
```

## 関連

- [Ghostty Discussion #9513 - Shift+key not transmitting to mprocs](https://github.com/ghostty-org/ghostty/discussions/9513)
- [Ghostty Discussion #8437 - TUI apps cause issues under Wayland](https://github.com/ghostty-org/ghostty/discussions/8437)
- [Helix PR #6438 - Handle keyboard enhancement check failure](https://github.com/helix-editor/helix/pull/6438)
- [ADR-026 開発サーバー一括起動ツールの選定](../../../docs/70_ADR/026_開発サーバー一括起動ツールの選定.md)
