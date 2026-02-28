# Issue #198: 人間向け表示用 ID の導入 — 設計計画

## 目的

UUID のみの ID 体系に「人間向け表示用 ID」（例: `WF-123`）を追加する設計を行う。
このセッションでは **設計ドキュメントの作成のみ** を行い、コードは書かない。

## 成果物

| 成果物 | ファイルパス |
|--------|-----------|
| ADR | `docs/70_ADR/029_人間向け表示用IDの導入.md` |
| 詳細設計書 | `docs/40_詳細設計書/12_表示用ID設計.md` |
| ID 設計規約の更新 | `docs/40_詳細設計書/04_ID設計規約.md` |
| Issue 更新 | GitHub Issue #198 に実装計画を追記 |

## 設計の要約

### 推奨案

| 検討項目 | 決定 | 理由 |
|---------|------|------|
| スコープ | テナント単位連番 | マルチテナント SaaS として自然。テナント間の情報漏洩防止 |
| 採番方式 | カウンターテーブル + `SELECT FOR UPDATE` | KISS。DDL 不要、テナント追加が INSERT のみ |
| プレフィックス | 固定（`WF-`, `TASK-`） | MVP に最適。将来カスタマイズ可能な余地あり |
| DB 保存形式 | `display_number` (BIGINT) | プレフィックスはアプリ層で結合。変更に強い |
| 対象エンティティ | Phase A: WorkflowInstance、Phase B: WorkflowStep | 段階的導入 |
| URL ルーティング | UUID 維持 | 一意性保証、テナント ID 不要 |

### ADR-001 との関係

ADR-001 が却下した「BIGSERIAL+UUID」は主キー構造の議論。
今回の表示用 ID は UUID 主キーを維持したまま、人間向けの補助フィールドを追加する設計であり矛盾しない。

## 作業ステップ

### Step 1: ADR-029 を作成

`docs/70_ADR/029_人間向け表示用IDの導入.md`

内容:
- コンテキスト: UUID の運用課題、ADR-001 との関係整理
- 検討した選択肢:
  1. カウンターテーブル + SELECT FOR UPDATE（テナント単位）✅
  2. テナント別 PostgreSQL SEQUENCE
  3. UUID v7 タイムスタンプ短縮表示
  4. カウンターテーブル + Advisory Lock
- スコープ設計: グローバル vs テナント単位
- プレフィックス設計: 固定 vs カスタマイズ vs なし
- 決定と理由
- 帰結（肯定的・否定的影響）

### Step 2: 詳細設計書を作成

`docs/40_詳細設計書/12_表示用ID設計.md`

内容:
- 表示用 ID の仕様（フォーマット、スコープ、欠番ポリシー）
- 対象エンティティと段階的導入計画
- DB スキーマ設計:
  - `display_id_counters` テーブル（複合 PK: tenant_id + entity_type）
  - `workflow_instances.display_number` カラム追加
  - ユニーク制約・インデックス
- ドメインモデル設計（DisplayNumber 値オブジェクト）
- 採番フロー（シーケンス図）
- API 仕様変更（display_id フィールド追加）
- フロントエンド対応
- 既存データのマイグレーション方針

### Step 3: ID 設計規約を更新

`docs/40_詳細設計書/04_ID設計規約.md` に表示用 ID セクションを追記。

### Step 4: Issue #198 を更新

Issue 本文に実装計画を追記:
- Phase A: WorkflowInstance（3 Sub-issue に分割）
  - A-1: DB スキーマ変更
  - A-2: バックエンド実装
  - A-3: API + フロントエンド
- Phase B: WorkflowStep（Phase A 完了後）

### Step 5: 設計成果物のコミット + Draft PR

設計ドキュメントをコミットし、Draft PR を作成。

## 検証方法

- ADR が既存の ADR（特に ADR-001）と矛盾しないことを確認
- 詳細設計書がデータベース設計書（02）、API 設計書（03）、ID 設計規約（04）と整合することを確認
- `just check-all` でドキュメント以外に影響がないことを確認
