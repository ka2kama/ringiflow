-- シードデータの definition に position フィールドを追加する
--
-- デザイナーキャンバスで position が必須になったため、
-- 既存のシード定義にも position を追加して新スキーマに適合させる。
-- jsonb_set では配列内の全要素への一括追加が困難なため、JSON 全体置換方式を採用。

-- 汎用申請（1段階承認）
UPDATE workflow_definitions
SET definition = '{
  "form": {
    "fields": [
      {"id": "title", "type": "text", "label": "件名", "required": true, "maxLength": 100},
      {"id": "description", "type": "textarea", "label": "内容", "required": true, "maxLength": 2000}
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
}'
WHERE id = '00000000-0000-0000-0000-000000000001';

-- 2段階承認申請（上長承認 → 経理承認）
UPDATE workflow_definitions
SET definition = '{
  "form": {
    "fields": [
      {"id": "title", "type": "text", "label": "件名", "required": true, "maxLength": 100},
      {"id": "description", "type": "textarea", "label": "内容", "required": true, "maxLength": 2000},
      {"id": "amount", "type": "number", "label": "金額", "required": true}
    ]
  },
  "steps": [
    {"id": "start", "type": "start", "name": "開始", "position": {"x": 400, "y": 50}},
    {"id": "manager_approval", "type": "approval", "name": "上長承認", "assignee": {"type": "user"}, "position": {"x": 400, "y": 200}},
    {"id": "finance_approval", "type": "approval", "name": "経理承認", "assignee": {"type": "user"}, "position": {"x": 400, "y": 350}},
    {"id": "end_approved", "type": "end", "name": "承認完了", "status": "approved", "position": {"x": 250, "y": 500}},
    {"id": "end_rejected", "type": "end", "name": "却下", "status": "rejected", "position": {"x": 550, "y": 500}}
  ],
  "transitions": [
    {"from": "start", "to": "manager_approval"},
    {"from": "manager_approval", "to": "finance_approval", "trigger": "approve"},
    {"from": "manager_approval", "to": "end_rejected", "trigger": "reject"},
    {"from": "finance_approval", "to": "end_approved", "trigger": "approve"},
    {"from": "finance_approval", "to": "end_rejected", "trigger": "reject"}
  ]
}'
WHERE id = '00000000-0000-0000-0000-000000000002';
