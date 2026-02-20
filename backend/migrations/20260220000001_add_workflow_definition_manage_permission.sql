-- tenant_admin ロールに workflow_definition:manage 権限を追加する
-- ワークフロー定義の CRUD 操作には独立した権限が必要
-- （workflow:* は workflow_definition:manage にマッチしないため）

UPDATE roles
SET permissions = permissions || '["workflow_definition:manage"]'::jsonb
WHERE id = '00000000-0000-0000-0000-000000000002'
  AND name = 'tenant_admin'
  AND NOT permissions ? 'workflow_definition:manage';
