-- notification_logs テーブル
--
-- メール通知の送信結果を記録するログテーブル。
-- fire-and-forget パターンのため、送信失敗も記録する。

CREATE TABLE notification_logs (
    id                   UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id            UUID         NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    event_type           VARCHAR(50)  NOT NULL,
    workflow_instance_id UUID         NOT NULL REFERENCES workflow_instances(id) ON DELETE CASCADE,
    workflow_title       VARCHAR(255) NOT NULL,
    workflow_display_id  VARCHAR(50)  NOT NULL,
    recipient_user_id    UUID         NOT NULL,
    recipient_email      VARCHAR(255) NOT NULL,
    subject              VARCHAR(500) NOT NULL,
    status               VARCHAR(20)  NOT NULL,
    error_message        TEXT,
    sent_at              TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

-- RLS ポリシー
ALTER TABLE notification_logs ENABLE ROW LEVEL SECURITY;

CREATE POLICY notification_logs_tenant_isolation ON notification_logs
    USING (tenant_id = current_setting('app.current_tenant_id')::uuid);

-- インデックス
CREATE INDEX idx_notification_logs_tenant_id ON notification_logs (tenant_id);
CREATE INDEX idx_notification_logs_workflow_instance_id ON notification_logs (workflow_instance_id);
CREATE INDEX idx_notification_logs_recipient_user_id ON notification_logs (recipient_user_id);
CREATE INDEX idx_notification_logs_sent_at ON notification_logs (sent_at DESC);
