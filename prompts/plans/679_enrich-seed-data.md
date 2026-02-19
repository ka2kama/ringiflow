# シードデータ充実化

## Context

現在のシードデータはユーザー2人、インスタンス5件と最小限で、一覧画面のページネーション・フィルタリングのテスト、デモでの説得力に欠ける。ユーザー10人・インスタンス30件・コメント12件に拡充し、開発・デモ環境を充実させる。

関連 Issue: #136（本番運用時にシードデータ分離。今回は現行方式のマイグレーションで追加）

## スコープ

- **対象**: 新規マイグレーション `backend/migrations/20260219000001_seed_additional_data.sql` を1ファイル作成
- **対象外**: テナント追加、ワークフロー定義追加、Rust/Elm コード変更

## 定数

| 名前 | 値 |
|------|-----|
| テナント ID | `00000000-0000-0000-0000-000000000001` |
| Argon2id ハッシュ (password123) | `$argon2id$v=19$m=65536,t=1,p=1$olntqw+EoVpwH4B1vUAI0A$5yCA1izLODgz8nQOInDGwbuQB/AS0sIQDwpmIilve5M` |
| tenant_admin ロール ID | `00000000-0000-0000-0000-000000000002` |
| user ロール ID | `00000000-0000-0000-0000-000000000003` |
| 汎用申請 定義 ID | `00000000-0000-0000-0000-000000000001` |
| 2段階承認 定義 ID | `00000000-0000-0000-0000-000000000002` |

## UUID 命名規則

| エンティティ | パターン | 範囲 |
|------------|---------|------|
| ユーザー | `00000000-0000-0000-0000-00000000000X` | 03〜0a |
| user_roles | `d0000000-0000-0000-0000-00000000000X` | 01〜08 |
| auth.credentials | `e0000000-0000-0000-0000-00000000000X` | 01〜08 |
| インスタンス | `a0000000-0000-0000-0000-0000000000XX` | 01〜19 |
| ステップ | `b0000000-0000-0000-0000-0000000000XX` | 01〜18 |
| コメント | `c0000000-0000-0000-0000-0000000000XX` | 01〜0c |

## Phase 1: ユーザー追加（8人、display_number 3-10）

### 確認事項
- [x] users テーブルカラム: id, tenant_id, email, name, status, display_number（password_hash は削除済み） → 既存マイグレーション確認済み、6カラム構成
- [x] auth.credentials カラム: id, user_id, tenant_id, credential_type, credential_data, is_active → 6カラム、credential_type='password'
- [x] user_roles カラム: id, user_id, role_id, tenant_id → 4カラム構成

### データ

| display_number | UUID末尾 | name | email | role |
|---|---|---|---|---|
| 3 | 03 | 田中 太郎 | tanaka@example.com | user |
| 4 | 04 | 佐藤 花子 | sato@example.com | user |
| 5 | 05 | 鈴木 一郎 | suzuki@example.com | user |
| 6 | 06 | 高橋 美咲 | takahashi@example.com | tenant_admin |
| 7 | 07 | 伊藤 健太 | ito@example.com | user |
| 8 | 08 | 渡辺 さくら | watanabe@example.com | user |
| 9 | 09 | 山本 大輔 | yamamoto@example.com | user |
| 10 | 0a | 中村 あおい | nakamura@example.com | user |

INSERT 順序: users → user_roles → auth.credentials

### テストリスト

ユニットテスト: 該当なし（SQL マイグレーション）

ハンドラテスト: 該当なし

API テスト: 該当なし

E2E テスト: 該当なし

## Phase 2: ワークフローインスタンス追加（25件、display_number 6-30）

### 確認事項
- [x] form_data 形式: 汎用は `{"title":..., "description":...}`、2段階は `{"title":..., "description":..., "amount":...}` → 既存シードデータ確認済み
- [x] ステータス CHECK 制約値: draft, pending, in_progress, approved, rejected, cancelled, changes_requested → マイグレーション 20260130000001 で定義
- [x] current_step_id: draft は NULL、pending/in_progress は step_id 文字列、approved/rejected は最終 step_id → 既存データと整合確認済み

### データ（25件）

**draft (3件)**: submitted_at=NULL, current_step_id=NULL

| display# | UUID末尾 | title | initiated_by | definition | created_at |
|---|---|---|---|---|---|
| 6 | 01 | リモートワーク申請 | 05 (鈴木) | 汎用 | 2025-12-02 |
| 7 | 02 | 備品購入申請（ノートPC） | 08 (渡辺) | 汎用 | 2025-12-10 |
| 8 | 03 | 予算申請（チームビルディング） | 07 (伊藤) | 2段階 | 2025-12-15 |

**pending (4件)**: submitted_at あり, current_step_id='approval'

| display# | UUID末尾 | title | initiated_by | definition | submitted_at |
|---|---|---|---|---|---|
| 9 | 04 | 経費精算申請（タクシー代） | 03 (田中) | 汎用 | 2025-12-18 |
| 10 | 05 | 研修参加申請（資格対策） | 09 (山本) | 汎用 | 2025-12-22 |
| 11 | 06 | 備品購入申請（ソフトウェアライセンス） | 05 (鈴木) | 汎用 | 2026-01-06 |
| 12 | 07 | 出張申請（福岡） | 07 (伊藤) | 汎用 | 2026-01-08 |

**in_progress (5件)**: submitted_at あり, current_step_id=現在の step_id

| display# | UUID末尾 | title | initiated_by | definition | submitted_at | current_step_id |
|---|---|---|---|---|---|---|
| 13 | 08 | 休暇申請（特別休暇） | 04 (佐藤) | 汎用 | 2026-01-10 | approval |
| 14 | 09 | 経費精算申請（書籍購入） | 0a (中村) | 汎用 | 2026-01-14 | approval |
| 15 | 0a | 備品購入申請（ディスプレイ） | 08 (渡辺) | 汎用 | 2026-01-15 | approval |
| 16 | 0b | 出張申請（海外カンファレンス） | 05 (鈴木) | 2段階 | 2026-01-20 | finance_approval |
| 17 | 0c | 経費精算申請（セミナー参加費） | 03 (田中) | 汎用 | 2026-01-22 | approval |

**approved (9件)**: submitted_at, completed_at あり

| display# | UUID末尾 | title | initiated_by | definition | submitted_at | completed_at |
|---|---|---|---|---|---|---|
| 18 | 0d | 経費精算申請（出張費・大阪） | 03 (田中) | 汎用 | 2026-01-23 | 2026-01-24 |
| 19 | 0e | 備品購入申請（キーボード） | 08 (渡辺) | 汎用 | 2026-01-24 | 2026-01-25 |
| 20 | 0f | 休暇申請（リフレッシュ休暇） | 09 (山本) | 汎用 | 2026-01-27 | 2026-01-28 |
| 21 | 10 | 研修参加申請（技術研修） | 05 (鈴木) | 汎用 | 2026-01-28 | 2026-01-29 |
| 22 | 11 | 出張申請（名古屋） | 07 (伊藤) | 汎用 | 2026-01-29 | 2026-01-30 |
| 23 | 12 | 経費精算申請（交通費・定期外） | 04 (佐藤) | 汎用 | 2026-02-03 | 2026-02-04 |
| 24 | 13 | 備品購入申請（デスク） | 0a (中村) | 汎用 | 2026-02-05 | 2026-02-06 |
| 25 | 14 | 予算申請（部門合宿） | 06 (高橋) | 2段階 | 2026-02-07 | 2026-02-10 |
| 26 | 15 | 休暇申請（有給） | 03 (田中) | 汎用 | 2026-02-10 | 2026-02-11 |

**rejected (4件)**: submitted_at, completed_at あり

| display# | UUID末尾 | title | initiated_by | definition | submitted_at | completed_at |
|---|---|---|---|---|---|---|
| 27 | 16 | 備品購入申請（高級チェア） | 0a (中村) | 汎用 | 2026-02-12 | 2026-02-13 |
| 28 | 17 | 出張申請（札幌） | 09 (山本) | 汎用 | 2026-02-13 | 2026-02-14 |
| 29 | 18 | 経費精算申請（接待費） | 07 (伊藤) | 汎用 | 2026-02-14 | 2026-02-15 |
| 30 | 19 | 出張申請（シンガポール） | 05 (鈴木) | 2段階 | 2026-02-17 | 2026-02-18 |

### テストリスト

ユニットテスト: 該当なし

ハンドラテスト: 該当なし

API テスト: 該当なし

E2E テスト: 該当なし

## Phase 3: ワークフローステップ追加（24件、display_number 5-28）

### 確認事項
- [x] ステップの version: pending/active は 1、completed は 2（既存パターン準拠） → 既存シードデータ4件で確認済み
- [x] display_number: テナント全体で通し番号を採番。ユニーク制約は (instance_id, display_number) → display_id_counters テーブルで管理
- [x] 2段階承認のステップ: manager_approval / finance_approval → 既存ワークフロー定義で確認済み

### 設計判断

**2段階承認インスタンスのステップ構成**:
- instance 0b (in_progress): manager_approval completed(approved) → finance_approval active
- instance 14 (approved): manager_approval completed(approved) → finance_approval completed(approved)
- instance 19 (rejected): manager_approval completed(rejected) のみ（finance_approval なし）

**承認者の割り当て方針**:
- 管理者(admin, 01) と 高橋(tenant_admin, 06) が主な承認者
- 2段階承認: 上長承認 → admin/高橋、経理承認 → 佐藤(04)

### データ（24件）

pending ステップ (4件): status=pending, decision=NULL, version=1

| display# | UUID末尾 | instance末尾 | step_id | assigned_to |
|---|---|---|---|---|
| 5 | 01 | 04 | approval | 01 (admin) |
| 6 | 02 | 05 | approval | 06 (高橋) |
| 7 | 03 | 06 | approval | 01 (admin) |
| 8 | 04 | 07 | approval | 06 (高橋) |

active ステップ (6件): status=active, decision=NULL, version=1

| display# | UUID末尾 | instance末尾 | step_id | assigned_to | started_at |
|---|---|---|---|---|---|
| 9 | 05 | 08 | approval | 01 (admin) | 2026-01-10 |
| 10 | 06 | 09 | approval | 06 (高橋) | 2026-01-14 |
| 11 | 07 | 0a | approval | 01 (admin) | 2026-01-15 |
| 12 | 08 | 0b | manager_approval | 06 (高橋) | ※ completed (below) |
| 13 | 09 | 0b | finance_approval | 04 (佐藤) | 2026-01-21 |
| 14 | 0a | 0c | approval | 06 (高橋) | 2026-01-22 |

※ step 12 (instance 0b の manager_approval) は completed/approved、step 13 が active

completed+approved ステップ (10件): status=completed, decision=approved, version=2

| display# | UUID末尾 | instance末尾 | step_id | assigned_to | comment |
|---|---|---|---|---|---|
| 12 | 08 | 0b | manager_approval | 06 (高橋) | 出張の必要性を確認しました。承認します。 |
| 15 | 0b | 0d | approval | 06 (高橋) | 確認しました。承認します。 |
| 16 | 0c | 0e | approval | 01 (admin) | 承認します。 |
| 17 | 0d | 0f | approval | 06 (高橋) | 良いリフレッシュを。承認します。 |
| 18 | 0e | 10 | approval | 01 (admin) | 技術力向上に有益です。承認します。 |
| 19 | 0f | 11 | approval | 06 (高橋) | 内容を確認しました。承認します。 |
| 20 | 10 | 12 | approval | 01 (admin) | 承認します。 |
| 21 | 11 | 13 | approval | 06 (高橋) | 承認します。 |
| 22 | 12 | 14 | manager_approval | 01 (admin) | 合宿企画、良いと思います。承認します。 |
| 23 | 13 | 14 | finance_approval | 04 (佐藤) | 予算内です。承認します。 |
| 24 | 14 | 15 | approval | 01 (admin) | 承認します。 |

completed+rejected ステップ (4件): status=completed, decision=rejected, version=2

| display# | UUID末尾 | instance末尾 | step_id | assigned_to | comment |
|---|---|---|---|---|---|
| 25 | 15 | 16 | approval | 01 (admin) | 高級チェアは予算外です。標準品での再申請をお願いします。 |
| 26 | 16 | 17 | approval | 06 (高橋) | 出張の緊急性が不明です。再申請してください。 |
| 27 | 17 | 18 | approval | 01 (admin) | 領収書が不足しています。再申請をお願いします。 |
| 28 | 18 | 19 | manager_approval | 06 (高橋) | 海外出張は現時点では承認できません。国内カンファレンスで代替を検討してください。 |

### テストリスト

ユニットテスト: 該当なし

ハンドラテスト: 該当なし

API テスト: 該当なし

E2E テスト: 該当なし

## Phase 4: ワークフローコメント追加（12件）

### 確認事項
- [x] workflow_comments.id は UUID v7（DEFAULT なし、アプリ生成）。シードでは `c0000000-...` 固定パターンを使用 → マイグレーション定義確認済み
- [x] body は 1〜2000文字の CHECK 制約 → 全コメント制約範囲内

### データ（12件）

| UUID末尾 | instance末尾 | posted_by | body | created_at |
|---|---|---|---|---|
| 01 | 08 (特別休暇) | 04 (佐藤) | 慶弔休暇の規定に基づき、3日間の特別休暇を申請します。 | 2026-01-10 08:20 |
| 02 | 08 | 01 (admin) | 承認規定を確認中です。証明書類の提出をお願いします。 | 2026-01-10 14:00 |
| 03 | 0b (海外出張 2段階) | 05 (鈴木) | 参加するセッションの一覧を添付しました。ご確認ください。 | 2026-01-20 10:00 |
| 04 | 0b | 06 (高橋) | セッション内容を確認しました。上長承認完了です。 | 2026-01-21 10:00 |
| 05 | 0d (出張費大阪) | 03 (田中) | 前回の出張と合算での精算になります。領収書は添付済みです。 | 2026-01-23 09:25 |
| 06 | 0d | 06 (高橋) | 確認しました。問題ありません。 | 2026-01-24 14:50 |
| 07 | 14 (部門合宿 2段階) | 06 (高橋) | 合宿の詳細プランを添付しました。 | 2026-02-07 11:00 |
| 08 | 14 | 01 (admin) | 良い企画だと思います。経理確認をお願いします。 | 2026-02-08 14:10 |
| 09 | 14 | 04 (佐藤) | 予算枠内であることを確認しました。 | 2026-02-10 14:50 |
| 0a | 16 (高級チェア) | 0a (中村) | エルゴノミクス対応のチェアを希望します。 | 2026-02-12 11:35 |
| 0b | 16 | 01 (admin) | 標準備品カタログから選定してください。カタログを共有します。 | 2026-02-13 14:10 |
| 0c | 19 (シンガポール 2段階) | 05 (鈴木) | AWS re:Invent Asia への参加を希望します。 | 2026-02-17 10:35 |

### テストリスト

ユニットテスト: 該当なし

ハンドラテスト: 該当なし

API テスト: 該当なし

E2E テスト: 該当なし

## Phase 5: display_id_counters 更新

| entity_type | 旧値 | 新値 |
|---|---|---|
| user | 2 | 10 |
| workflow_instance | 5 | 30 |
| workflow_step | 4 | 28 |

### テストリスト

ユニットテスト: 該当なし

ハンドラテスト: 該当なし

API テスト: 該当なし

E2E テスト: 該当なし

## 検証方法

マイグレーション適用後に以下を実行:

```sql
-- ユーザー数: 10
SELECT COUNT(*) FROM users WHERE tenant_id = '00000000-0000-0000-0000-000000000001';

-- ステータス分布: draft:4, pending:5, in_progress:6, approved:10, rejected:5
SELECT status, COUNT(*) FROM workflow_instances
WHERE tenant_id = '00000000-0000-0000-0000-000000000001'
GROUP BY status ORDER BY status;

-- ステップ数: 28
SELECT COUNT(*) FROM workflow_steps
WHERE tenant_id = '00000000-0000-0000-0000-000000000001';

-- コメント数: 12
SELECT COUNT(*) FROM workflow_comments
WHERE tenant_id = '00000000-0000-0000-0000-000000000001';

-- カウンター確認
SELECT * FROM display_id_counters
WHERE tenant_id = '00000000-0000-0000-0000-000000000001';
```

`just check-all` が通過すること。

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | パスワードが auth.credentials に移行済みだが、当初 users.password_hash に INSERT しようとしていた | 不完全なパス | auth.credentials への INSERT に修正 |
| 2回目 | workflow_steps.version の初期値が曖昧。既存 completed は version=2 | 既存パターン整合 | completed=2, pending/active=1 に統一 |
| 3回目 | 2段階承認の form_data に amount フィールドが必要 | 不完全なパス | 2段階承認の3件すべてに amount を含める |
| 4回目 | current_step_id が NULL のままだった。pending/in_progress では step_id を設定すべき | 未定義 | 各ステータスに応じた current_step_id を明記 |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | users(8), user_roles(8), auth.credentials(8), instances(25), steps(24), comments(12), counters(3 UPDATE) |
| 2 | 曖昧さ排除 | OK | 全 UUID、display_number、タイムスタンプ、form_data を具体的に記載 |
| 3 | 設計判断の完結性 | OK | version値、承認者割り当て、2段階ステップ構成を明記 |
| 4 | スコープ境界 | OK | 対象: マイグレーション1ファイル。対象外: テナント/定義追加、コード変更 |
| 5 | 技術的前提 | OK | RLS はマイグレーション実行ユーザーに影響しない。CHECK制約値を確認済み |
| 6 | 既存ドキュメント整合 | OK | 既存シードのパターン（UUID形式、INSERT順序、コメント形式）に準拠 |
