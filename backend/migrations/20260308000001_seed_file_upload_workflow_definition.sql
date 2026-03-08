-- ファイル添付付き申請ワークフロー定義のシードデータ
--
-- E2E テスト（E2E-007）で使用する。
-- 汎用申請と同じ1段階承認だが、ファイルフィールドを含む。

INSERT INTO workflow_definitions (id, tenant_id, name, description, version, definition, status, created_by) VALUES
    ('00000000-0000-0000-0000-000000000003',
     '00000000-0000-0000-0000-000000000001',
     'ファイル添付申請',
     'ファイル添付が可能な申請テンプレート',
     1,
     '{
       "form": {
         "fields": [
           {"id": "title", "type": "text", "label": "件名", "required": true, "maxLength": 100},
           {"id": "description", "type": "textarea", "label": "内容", "required": false, "maxLength": 2000},
           {"id": "attachments", "type": "file", "label": "添付ファイル", "required": false}
         ]
       },
       "steps": [
         {"id": "start", "type": "start", "name": "開始", "position": {"x": 400, "y": 50}},
         {"id": "approval", "type": "approval", "name": "承認", "assignee": {"type": "user"}, "position": {"x": 400, "y": 200}},
         {"id": "end_approved", "type": "end", "name": "承認完了", "status": "approved", "position": {"x": 250, "y": 350}},
         {"id": "end_rejected", "type": "end", "name": "却下", "status": "rejected", "position": {"x": 550, "y": 350}}
       ],
       "transitions": [
         {"from": "start", "to": "approval"},
         {"from": "approval", "to": "end_approved", "trigger": "approve"},
         {"from": "approval", "to": "end_rejected", "trigger": "reject"}
       ]
     }',
     'published',
     '00000000-0000-0000-0000-000000000001');
