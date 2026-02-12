-- 2段階承認ワークフロー定義のシードデータ
--
-- 上長承認 → 経理承認の2段階順次承認ワークフロー。
-- Phase 2-3 の多段階承認機能の検証に使用する。

INSERT INTO workflow_definitions (id, tenant_id, name, description, version, definition, status, created_by) VALUES
    ('00000000-0000-0000-0000-000000000002',
     '00000000-0000-0000-0000-000000000001',
     '2段階承認申請',
     '上長承認・経理承認の2段階承認ワークフロー',
     1,
     '{
       "form": {
         "fields": [
           {"id": "title", "type": "text", "label": "件名", "required": true, "maxLength": 100},
           {"id": "description", "type": "textarea", "label": "内容", "required": true, "maxLength": 2000},
           {"id": "amount", "type": "number", "label": "金額", "required": true}
         ]
       },
       "steps": [
         {"id": "start", "type": "start", "name": "開始"},
         {"id": "manager_approval", "type": "approval", "name": "上長承認", "assignee": {"type": "user"}},
         {"id": "finance_approval", "type": "approval", "name": "経理承認", "assignee": {"type": "user"}},
         {"id": "end_approved", "type": "end", "name": "承認完了", "status": "approved"},
         {"id": "end_rejected", "type": "end", "name": "却下", "status": "rejected"}
       ],
       "transitions": [
         {"from": "start", "to": "manager_approval"},
         {"from": "manager_approval", "to": "finance_approval", "trigger": "approve"},
         {"from": "manager_approval", "to": "end_rejected", "trigger": "reject"},
         {"from": "finance_approval", "to": "end_approved", "trigger": "approve"},
         {"from": "finance_approval", "to": "end_rejected", "trigger": "reject"}
       ]
     }',
     'published',
     '00000000-0000-0000-0000-000000000001');
