-- 開発用ワークフローステップのシード
-- 各ワークフローインスタンスに対応するステップデータを作成し、
-- 承認/却下ボタンの動作確認に使用する
--
-- 依存: 20260128000001_seed_workflow_instances.sql
-- 本番運用時はマイグレーションから分離する予定（Issue #136）
--
-- ステップ構成（ワークフロー定義「汎用申請」の1段階承認フロー）:
--   start → approval → end_approved / end_rejected
--
-- ユーザー:
--   admin (00...01): tenant_admin
--   user  (00...02): 一般ユーザー
--
-- 対応関係:
--   draft (111...)       → ステップなし（未申請のため）
--   pending (222...)     → pending ステップ（承認フロー未開始）
--   in_progress (333...) → active ステップ（承認待ち）★ ボタン表示対象
--   approved (444...)    → completed ステップ（承認済み）
--   rejected (555...)    → completed ステップ（却下済み）

INSERT INTO workflow_steps (id, instance_id, step_id, step_name, step_type, status, version, assigned_to, decision, comment, started_at, completed_at, created_at, updated_at) VALUES
    -- pending インスタンス: 承認フロー未開始
    ('aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa',
     '22222222-2222-2222-2222-222222222222',
     'approval', '承認', 'approval',
     'pending', 1,
     '00000000-0000-0000-0000-000000000001',  -- admin が承認者
     NULL, NULL,
     NULL, NULL,
     '2026-01-21 14:30:00+09', '2026-01-21 14:30:00+09'),

    -- in_progress インスタンス: 承認待ち（★ admin でログインするとボタンが表示される）
    ('bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb',
     '33333333-3333-3333-3333-333333333333',
     'approval', '承認', 'approval',
     'active', 1,
     '00000000-0000-0000-0000-000000000001',  -- admin が承認者
     NULL, NULL,
     '2026-01-22 09:00:00+09', NULL,
     '2026-01-22 09:00:00+09', '2026-01-22 09:00:00+09'),

    -- approved インスタンス: 承認済み
    ('cccccccc-cccc-cccc-cccc-cccccccccccc',
     '44444444-4444-4444-4444-444444444444',
     'approval', '承認', 'approval',
     'completed', 2,
     '00000000-0000-0000-0000-000000000002',  -- user が承認者（申請者は admin）
     'approved', '内容を確認しました。承認します。',
     '2026-01-10 10:00:00+09', '2026-01-12 15:00:00+09',
     '2026-01-10 10:00:00+09', '2026-01-12 15:00:00+09'),

    -- rejected インスタンス: 却下済み
    ('dddddddd-dddd-dddd-dddd-dddddddddddd',
     '55555555-5555-5555-5555-555555555555',
     'approval', '承認', 'approval',
     'completed', 2,
     '00000000-0000-0000-0000-000000000001',  -- admin が承認者
     'rejected', '予算超過のため却下します。再申請をお願いします。',
     '2026-01-09 11:00:00+09', '2026-01-11 16:00:00+09',
     '2026-01-09 11:00:00+09', '2026-01-11 16:00:00+09');
