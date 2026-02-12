-- workflow_comments テーブルの作成
-- 構文リファレンス: README.md
--
-- ワークフローに対するコメントスレッドを管理する。
-- 承認プロセス中に申請者と承認者がコメントでやり取りするために使用する。
--
-- 既存の workflow_steps.comment（ステップの判定コメント）とは別概念。
-- workflow_steps.comment は承認/却下時の判定コメント、
-- workflow_comments はワークフロー全体のコメントスレッド。
--
-- 注: id は UUID v7（時系列ソート可能）を使用。
-- アプリケーション側で生成するため DEFAULT 句なし。
-- 参照: docs/05_ADR/001_ID形式の選定.md

CREATE TABLE workflow_comments (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    instance_id UUID NOT NULL REFERENCES workflow_instances(id) ON DELETE CASCADE,
    posted_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    body TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT workflow_comments_body_length CHECK (
        char_length(body) >= 1 AND char_length(body) <= 2000
    )
);

-- インデックス
CREATE INDEX workflow_comments_instance_idx ON workflow_comments(instance_id);
CREATE INDEX workflow_comments_tenant_idx ON workflow_comments(tenant_id);

-- updated_at 自動更新トリガー
CREATE TRIGGER workflow_comments_updated_at
    BEFORE UPDATE ON workflow_comments
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- RLS 有効化 + ポリシー作成
ALTER TABLE workflow_comments ENABLE ROW LEVEL SECURITY;

CREATE POLICY tenant_isolation ON workflow_comments
    FOR ALL
    TO ringiflow_app
    USING (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::UUID)
    WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::UUID);

-- コメント
COMMENT ON TABLE workflow_comments IS 'ワークフローコメント（コメントスレッド）';
COMMENT ON COLUMN workflow_comments.id IS '主キー';
COMMENT ON COLUMN workflow_comments.tenant_id IS 'テナントID（FK, RLS用）';
COMMENT ON COLUMN workflow_comments.instance_id IS 'ワークフローインスタンスID（FK）';
COMMENT ON COLUMN workflow_comments.posted_by IS '投稿者ユーザーID（FK）';
COMMENT ON COLUMN workflow_comments.body IS 'コメント本文（1〜2000文字）';
COMMENT ON COLUMN workflow_comments.created_at IS '作成日時';
COMMENT ON COLUMN workflow_comments.updated_at IS '更新日時';
