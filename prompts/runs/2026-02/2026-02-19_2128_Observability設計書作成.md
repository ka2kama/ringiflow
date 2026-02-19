# Observability 設計書作成

## 概要

Epic #648（Observability 基盤の設計と段階的実装）の Story 5（#655）として、Observability の詳細設計書を新規作成した。Story 1〜4, 6 で実装済みの設計判断を一元化し、Phase 4（OpenTelemetry + Datadog）への移行パスを文書化した。

## 実施内容

1. Issue #655 の精査
   - 全 Story（1-4, 6）が完了済みであることを確認
   - Issue の Want が有効であり、精査結果「続行」と判定

2. 調査
   - 運用設計書 9.1〜9.4, 8.5, 8.6 の MUST/MUST NOT 要件を抽出
   - 既存の詳細設計書（`07_認証機能設計.md`, `08_AuthService設計.md`）のフォーマットパターンを確認
   - 実装コード（`observability.rs`, `event_log.rs`, `request_id.rs`, `macros.rs`）を確認
   - 実装解説ドキュメント、ナレッジベース（`log-schema.md`）を確認

3. 設計書作成
   - `docs/03_詳細設計書/14_Observability設計.md` を作成
   - 章構成: 概要 → 設計原則 → アーキテクチャ → ログ設計 → 計装設計 → ビジネスイベントログ → メトリクス設計 → Phase 4 移行パス → MUST 要件対応表 → 変更履歴
   - 実装済みセクションはマーカーなし、Phase 4 セクションは実装状態マーカーあり

## 判断ログ

- 特筆すべき判断なし（既存実装の文書化であり、新しい設計判断は発生しなかった）

## 成果物

- `docs/03_詳細設計書/14_Observability設計.md`（新規作成）
- `prompts/plans/655_observability-design-doc.md`（計画ファイル）
- PR: #695
