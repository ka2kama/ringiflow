--
-- PostgreSQL database dump
--

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET transaction_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

--
-- Name: auth; Type: SCHEMA; Schema: -; Owner: -
--

CREATE SCHEMA auth;

--
-- Name: SCHEMA auth; Type: COMMENT; Schema: -; Owner: -
--

COMMENT ON SCHEMA auth IS 'Auth Service が所有するスキーマ。認証情報を管理。';

--
-- Name: update_updated_at(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.update_updated_at() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$;

SET default_table_access_method = heap;

--
-- Name: credentials; Type: TABLE; Schema: auth; Owner: -
--

CREATE TABLE auth.credentials (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    user_id uuid NOT NULL,
    tenant_id uuid NOT NULL,
    credential_type character varying(20) NOT NULL,
    credential_data text NOT NULL,
    is_active boolean DEFAULT true NOT NULL,
    last_used_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT chk_credential_type CHECK (((credential_type)::text = ANY ((ARRAY['password'::character varying, 'totp'::character varying, 'oidc'::character varying, 'saml'::character varying])::text[])))
);

--
-- Name: TABLE credentials; Type: COMMENT; Schema: auth; Owner: -
--

COMMENT ON TABLE auth.credentials IS '認証情報。テナント退会時は tenant_id で削除。';

--
-- Name: COLUMN credentials.id; Type: COMMENT; Schema: auth; Owner: -
--

COMMENT ON COLUMN auth.credentials.id IS '主キー';

--
-- Name: COLUMN credentials.user_id; Type: COMMENT; Schema: auth; Owner: -
--

COMMENT ON COLUMN auth.credentials.user_id IS 'ユーザーID（外部キー制約なし、サービス境界の独立性のため）';

--
-- Name: COLUMN credentials.tenant_id; Type: COMMENT; Schema: auth; Owner: -
--

COMMENT ON COLUMN auth.credentials.tenant_id IS 'テナントID（テナント退会時の削除に使用）';

--
-- Name: COLUMN credentials.credential_type; Type: COMMENT; Schema: auth; Owner: -
--

COMMENT ON COLUMN auth.credentials.credential_type IS '認証種別: password, totp, oidc, saml';

--
-- Name: COLUMN credentials.credential_data; Type: COMMENT; Schema: auth; Owner: -
--

COMMENT ON COLUMN auth.credentials.credential_data IS '認証データ（パスワードハッシュ等）';

--
-- Name: COLUMN credentials.is_active; Type: COMMENT; Schema: auth; Owner: -
--

COMMENT ON COLUMN auth.credentials.is_active IS '有効フラグ';

--
-- Name: COLUMN credentials.last_used_at; Type: COMMENT; Schema: auth; Owner: -
--

COMMENT ON COLUMN auth.credentials.last_used_at IS '最終使用日時';

--
-- Name: display_id_counters; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.display_id_counters (
    tenant_id uuid NOT NULL,
    entity_type character varying(50) NOT NULL,
    last_number bigint DEFAULT 0 NOT NULL,
    CONSTRAINT chk_last_number_non_negative CHECK ((last_number >= 0))
);

--
-- Name: TABLE display_id_counters; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON TABLE public.display_id_counters IS '表示用 ID の採番カウンター';

--
-- Name: COLUMN display_id_counters.tenant_id; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.display_id_counters.tenant_id IS 'テナント ID（FK）';

--
-- Name: COLUMN display_id_counters.entity_type; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.display_id_counters.entity_type IS 'エンティティ種別（workflow_instance, workflow_step）';

--
-- Name: COLUMN display_id_counters.last_number; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.display_id_counters.last_number IS '最後に採番した番号（0 は未採番）';

--
-- Name: roles; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.roles (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    tenant_id uuid,
    name character varying(100) NOT NULL,
    description text,
    permissions jsonb DEFAULT '[]'::jsonb NOT NULL,
    is_system boolean DEFAULT false NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);

--
-- Name: TABLE roles; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON TABLE public.roles IS 'システムロール: system_admin, tenant_admin, user が定義されている';

--
-- Name: COLUMN roles.id; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.roles.id IS '主キー';

--
-- Name: COLUMN roles.tenant_id; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.roles.tenant_id IS 'テナントID（NULL = システムロール）';

--
-- Name: COLUMN roles.name; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.roles.name IS 'ロール名';

--
-- Name: COLUMN roles.description; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.roles.description IS '説明';

--
-- Name: COLUMN roles.permissions; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.roles.permissions IS '権限リスト（JSON配列）';

--
-- Name: COLUMN roles.is_system; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.roles.is_system IS 'システム定義ロールか（削除・編集不可）';

--
-- Name: tenants; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.tenants (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    name character varying(255) NOT NULL,
    subdomain character varying(63) NOT NULL,
    plan character varying(50) DEFAULT 'free'::character varying NOT NULL,
    status character varying(20) DEFAULT 'active'::character varying NOT NULL,
    settings jsonb DEFAULT '{}'::jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT tenants_plan_check CHECK (((plan)::text = ANY ((ARRAY['free'::character varying, 'standard'::character varying, 'professional'::character varying, 'enterprise'::character varying])::text[]))),
    CONSTRAINT tenants_status_check CHECK (((status)::text = ANY ((ARRAY['active'::character varying, 'suspended'::character varying, 'deleted'::character varying])::text[])))
);

--
-- Name: TABLE tenants; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON TABLE public.tenants IS '開発用テナント (subdomain=dev) が定義されている';

--
-- Name: COLUMN tenants.id; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.tenants.id IS '主キー';

--
-- Name: COLUMN tenants.name; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.tenants.name IS 'テナント名';

--
-- Name: COLUMN tenants.subdomain; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.tenants.subdomain IS 'サブドメイン（ユニーク）';

--
-- Name: COLUMN tenants.plan; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.tenants.plan IS 'プラン（free/standard/professional/enterprise）';

--
-- Name: COLUMN tenants.status; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.tenants.status IS '状態（active/suspended/deleted）';

--
-- Name: COLUMN tenants.settings; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.tenants.settings IS 'テナント設定（JSON）';

--
-- Name: user_roles; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.user_roles (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    user_id uuid NOT NULL,
    role_id uuid NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    tenant_id uuid NOT NULL
);

--
-- Name: TABLE user_roles; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON TABLE public.user_roles IS 'ユーザーとロールの関連';

--
-- Name: COLUMN user_roles.id; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.user_roles.id IS '主キー';

--
-- Name: COLUMN user_roles.user_id; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.user_roles.user_id IS 'ユーザーID（FK）';

--
-- Name: COLUMN user_roles.role_id; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.user_roles.role_id IS 'ロールID（FK）';

--
-- Name: COLUMN user_roles.tenant_id; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.user_roles.tenant_id IS 'テナントID（FK、RLS 二重防御用）';

--
-- Name: users; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.users (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    tenant_id uuid NOT NULL,
    email character varying(255) NOT NULL,
    name character varying(255) NOT NULL,
    status character varying(20) DEFAULT 'active'::character varying NOT NULL,
    last_login_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    display_number bigint NOT NULL,
    CONSTRAINT users_status_check CHECK (((status)::text = ANY ((ARRAY['active'::character varying, 'inactive'::character varying, 'deleted'::character varying])::text[])))
);

--
-- Name: TABLE users; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON TABLE public.users IS 'ユーザー情報（認証情報は auth.credentials で管理）';

--
-- Name: COLUMN users.id; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.users.id IS '主キー';

--
-- Name: COLUMN users.tenant_id; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.users.tenant_id IS 'テナントID（FK）';

--
-- Name: COLUMN users.email; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.users.email IS 'メールアドレス（テナント内でユニーク）';

--
-- Name: COLUMN users.name; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.users.name IS '表示名';

--
-- Name: COLUMN users.status; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.users.status IS '状態（active/inactive/deleted）';

--
-- Name: COLUMN users.last_login_at; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.users.last_login_at IS '最終ログイン日時';

--
-- Name: COLUMN users.display_number; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.users.display_number IS '表示用連番（テナント内で一意）';

--
-- Name: workflow_comments; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.workflow_comments (
    id uuid NOT NULL,
    tenant_id uuid NOT NULL,
    instance_id uuid NOT NULL,
    posted_by uuid NOT NULL,
    body text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT workflow_comments_body_length CHECK (((char_length(body) >= 1) AND (char_length(body) <= 2000)))
);

--
-- Name: TABLE workflow_comments; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON TABLE public.workflow_comments IS 'ワークフローコメント（コメントスレッド）';

--
-- Name: COLUMN workflow_comments.id; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.workflow_comments.id IS '主キー';

--
-- Name: COLUMN workflow_comments.tenant_id; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.workflow_comments.tenant_id IS 'テナントID（FK, RLS用）';

--
-- Name: COLUMN workflow_comments.instance_id; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.workflow_comments.instance_id IS 'ワークフローインスタンスID（FK）';

--
-- Name: COLUMN workflow_comments.posted_by; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.workflow_comments.posted_by IS '投稿者ユーザーID（FK）';

--
-- Name: COLUMN workflow_comments.body; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.workflow_comments.body IS 'コメント本文（1〜2000文字）';

--
-- Name: COLUMN workflow_comments.created_at; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.workflow_comments.created_at IS '作成日時';

--
-- Name: COLUMN workflow_comments.updated_at; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.workflow_comments.updated_at IS '更新日時';

--
-- Name: workflow_definitions; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.workflow_definitions (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    tenant_id uuid NOT NULL,
    name character varying(255) NOT NULL,
    description text,
    version integer DEFAULT 1 NOT NULL,
    definition jsonb NOT NULL,
    status character varying(20) DEFAULT 'draft'::character varying NOT NULL,
    created_by uuid NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT workflow_definitions_status_check CHECK (((status)::text = ANY ((ARRAY['draft'::character varying, 'published'::character varying, 'archived'::character varying])::text[])))
);

--
-- Name: TABLE workflow_definitions; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON TABLE public.workflow_definitions IS 'MVP用の汎用申請ワークフローが定義されている';

--
-- Name: COLUMN workflow_definitions.id; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.workflow_definitions.id IS '主キー';

--
-- Name: COLUMN workflow_definitions.tenant_id; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.workflow_definitions.tenant_id IS 'テナントID（FK）';

--
-- Name: COLUMN workflow_definitions.name; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.workflow_definitions.name IS '定義名';

--
-- Name: COLUMN workflow_definitions.description; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.workflow_definitions.description IS '説明';

--
-- Name: COLUMN workflow_definitions.version; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.workflow_definitions.version IS 'バージョン';

--
-- Name: COLUMN workflow_definitions.definition; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.workflow_definitions.definition IS '定義本体（JSON）';

--
-- Name: COLUMN workflow_definitions.status; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.workflow_definitions.status IS '状態（draft/published/archived）';

--
-- Name: COLUMN workflow_definitions.created_by; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.workflow_definitions.created_by IS '作成者（FK）';

--
-- Name: workflow_instances; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.workflow_instances (
    id uuid NOT NULL,
    tenant_id uuid NOT NULL,
    definition_id uuid NOT NULL,
    definition_version integer NOT NULL,
    title character varying(500) NOT NULL,
    form_data jsonb DEFAULT '{}'::jsonb NOT NULL,
    status character varying(20) DEFAULT 'draft'::character varying NOT NULL,
    current_step_id character varying(100),
    initiated_by uuid NOT NULL,
    submitted_at timestamp with time zone,
    completed_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    version integer DEFAULT 1 NOT NULL,
    display_number bigint NOT NULL,
    CONSTRAINT workflow_instances_status_check CHECK (((status)::text = ANY ((ARRAY['draft'::character varying, 'pending'::character varying, 'in_progress'::character varying, 'approved'::character varying, 'rejected'::character varying, 'cancelled'::character varying, 'changes_requested'::character varying])::text[])))
);

--
-- Name: TABLE workflow_instances; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON TABLE public.workflow_instances IS '開発用サンプルデータ: 各ステータスのワークフローインスタンスが定義されている';

--
-- Name: COLUMN workflow_instances.id; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.workflow_instances.id IS '主キー';

--
-- Name: COLUMN workflow_instances.tenant_id; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.workflow_instances.tenant_id IS 'テナントID（FK）';

--
-- Name: COLUMN workflow_instances.definition_id; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.workflow_instances.definition_id IS '定義ID（FK）';

--
-- Name: COLUMN workflow_instances.definition_version; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.workflow_instances.definition_version IS '定義バージョン（作成時点）';

--
-- Name: COLUMN workflow_instances.title; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.workflow_instances.title IS 'タイトル';

--
-- Name: COLUMN workflow_instances.form_data; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.workflow_instances.form_data IS 'フォームデータ（JSON）';

--
-- Name: COLUMN workflow_instances.status; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.workflow_instances.status IS '状態（draft/pending/in_progress/approved/rejected/cancelled/changes_requested）';

--
-- Name: COLUMN workflow_instances.current_step_id; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.workflow_instances.current_step_id IS '現在のステップID';

--
-- Name: COLUMN workflow_instances.initiated_by; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.workflow_instances.initiated_by IS '申請者（FK）';

--
-- Name: COLUMN workflow_instances.submitted_at; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.workflow_instances.submitted_at IS '申請日時';

--
-- Name: COLUMN workflow_instances.completed_at; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.workflow_instances.completed_at IS '完了日時';

--
-- Name: COLUMN workflow_instances.version; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.workflow_instances.version IS '楽観的ロック用バージョン番号';

--
-- Name: COLUMN workflow_instances.display_number; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.workflow_instances.display_number IS '表示用連番（テナント内で一意）';

--
-- Name: workflow_steps; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.workflow_steps (
    id uuid NOT NULL,
    instance_id uuid NOT NULL,
    step_id character varying(100) NOT NULL,
    step_name character varying(255) NOT NULL,
    step_type character varying(50) NOT NULL,
    status character varying(20) DEFAULT 'pending'::character varying NOT NULL,
    assigned_to uuid,
    decision character varying(50),
    comment text,
    due_date timestamp with time zone,
    started_at timestamp with time zone,
    completed_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    version integer DEFAULT 1 NOT NULL,
    display_number bigint NOT NULL,
    tenant_id uuid NOT NULL,
    CONSTRAINT workflow_steps_decision_check CHECK (((decision IS NULL) OR ((decision)::text = ANY ((ARRAY['approved'::character varying, 'rejected'::character varying, 'request_changes'::character varying])::text[])))),
    CONSTRAINT workflow_steps_status_check CHECK (((status)::text = ANY ((ARRAY['pending'::character varying, 'active'::character varying, 'completed'::character varying, 'skipped'::character varying])::text[])))
);

--
-- Name: TABLE workflow_steps; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON TABLE public.workflow_steps IS 'ワークフローステップ（承認タスク）';

--
-- Name: COLUMN workflow_steps.id; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.workflow_steps.id IS '主キー';

--
-- Name: COLUMN workflow_steps.instance_id; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.workflow_steps.instance_id IS 'インスタンスID（FK）';

--
-- Name: COLUMN workflow_steps.step_id; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.workflow_steps.step_id IS '定義上のステップID';

--
-- Name: COLUMN workflow_steps.step_name; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.workflow_steps.step_name IS 'ステップ名';

--
-- Name: COLUMN workflow_steps.step_type; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.workflow_steps.step_type IS 'ステップ種別（approval/notification/...）';

--
-- Name: COLUMN workflow_steps.status; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.workflow_steps.status IS '状態（pending/active/completed/skipped）';

--
-- Name: COLUMN workflow_steps.assigned_to; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.workflow_steps.assigned_to IS '担当者（FK）';

--
-- Name: COLUMN workflow_steps.decision; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.workflow_steps.decision IS '判断（approved/rejected/request_changes）';

--
-- Name: COLUMN workflow_steps.comment; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.workflow_steps.comment IS 'コメント';

--
-- Name: COLUMN workflow_steps.due_date; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.workflow_steps.due_date IS '期限';

--
-- Name: COLUMN workflow_steps.started_at; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.workflow_steps.started_at IS '開始日時';

--
-- Name: COLUMN workflow_steps.completed_at; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.workflow_steps.completed_at IS '完了日時';

--
-- Name: COLUMN workflow_steps.version; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.workflow_steps.version IS '楽観的ロック用バージョン番号';

--
-- Name: COLUMN workflow_steps.display_number; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.workflow_steps.display_number IS '表示用連番（インスタンス内で一意）';

--
-- Name: COLUMN workflow_steps.tenant_id; Type: COMMENT; Schema: public; Owner: -
--

COMMENT ON COLUMN public.workflow_steps.tenant_id IS 'テナントID（FK、RLS 二重防御用）';

--
-- Name: credentials credentials_pkey; Type: CONSTRAINT; Schema: auth; Owner: -
--

ALTER TABLE ONLY auth.credentials
    ADD CONSTRAINT credentials_pkey PRIMARY KEY (id);

--
-- Name: credentials uq_credentials_user_type; Type: CONSTRAINT; Schema: auth; Owner: -
--

ALTER TABLE ONLY auth.credentials
    ADD CONSTRAINT uq_credentials_user_type UNIQUE (user_id, credential_type);

--
-- Name: display_id_counters display_id_counters_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.display_id_counters
    ADD CONSTRAINT display_id_counters_pkey PRIMARY KEY (tenant_id, entity_type);

--
-- Name: roles roles_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.roles
    ADD CONSTRAINT roles_pkey PRIMARY KEY (id);

--
-- Name: roles roles_tenant_name_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.roles
    ADD CONSTRAINT roles_tenant_name_key UNIQUE (tenant_id, name);

--
-- Name: tenants tenants_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.tenants
    ADD CONSTRAINT tenants_pkey PRIMARY KEY (id);

--
-- Name: tenants tenants_subdomain_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.tenants
    ADD CONSTRAINT tenants_subdomain_key UNIQUE (subdomain);

--
-- Name: user_roles user_roles_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_roles
    ADD CONSTRAINT user_roles_pkey PRIMARY KEY (id);

--
-- Name: user_roles user_roles_user_role_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_roles
    ADD CONSTRAINT user_roles_user_role_key UNIQUE (user_id, role_id);

--
-- Name: users users_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_pkey PRIMARY KEY (id);

--
-- Name: users users_tenant_email_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_tenant_email_key UNIQUE (tenant_id, email);

--
-- Name: workflow_comments workflow_comments_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.workflow_comments
    ADD CONSTRAINT workflow_comments_pkey PRIMARY KEY (id);

--
-- Name: workflow_definitions workflow_definitions_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.workflow_definitions
    ADD CONSTRAINT workflow_definitions_pkey PRIMARY KEY (id);

--
-- Name: workflow_instances workflow_instances_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.workflow_instances
    ADD CONSTRAINT workflow_instances_pkey PRIMARY KEY (id);

--
-- Name: workflow_steps workflow_steps_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.workflow_steps
    ADD CONSTRAINT workflow_steps_pkey PRIMARY KEY (id);

--
-- Name: idx_credentials_tenant_id; Type: INDEX; Schema: auth; Owner: -
--

CREATE INDEX idx_credentials_tenant_id ON auth.credentials USING btree (tenant_id);

--
-- Name: idx_credentials_user_id; Type: INDEX; Schema: auth; Owner: -
--

CREATE INDEX idx_credentials_user_id ON auth.credentials USING btree (user_id);

--
-- Name: idx_users_display_number; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX idx_users_display_number ON public.users USING btree (tenant_id, display_number) WHERE (display_number IS NOT NULL);

--
-- Name: idx_workflow_instances_display_number; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX idx_workflow_instances_display_number ON public.workflow_instances USING btree (tenant_id, display_number) WHERE (display_number IS NOT NULL);

--
-- Name: idx_workflow_steps_display_number; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX idx_workflow_steps_display_number ON public.workflow_steps USING btree (instance_id, display_number) WHERE (display_number IS NOT NULL);

--
-- Name: user_roles_role_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_roles_role_idx ON public.user_roles USING btree (role_id);

--
-- Name: user_roles_tenant_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_roles_tenant_id_idx ON public.user_roles USING btree (tenant_id);

--
-- Name: user_roles_user_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX user_roles_user_idx ON public.user_roles USING btree (user_id);

--
-- Name: users_tenant_status_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX users_tenant_status_idx ON public.users USING btree (tenant_id, status);

--
-- Name: workflow_comments_instance_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX workflow_comments_instance_idx ON public.workflow_comments USING btree (instance_id);

--
-- Name: workflow_comments_tenant_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX workflow_comments_tenant_idx ON public.workflow_comments USING btree (tenant_id);

--
-- Name: workflow_definitions_tenant_status_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX workflow_definitions_tenant_status_idx ON public.workflow_definitions USING btree (tenant_id, status);

--
-- Name: workflow_instances_created_at_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX workflow_instances_created_at_idx ON public.workflow_instances USING btree (tenant_id, created_at DESC);

--
-- Name: workflow_instances_initiated_by_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX workflow_instances_initiated_by_idx ON public.workflow_instances USING btree (initiated_by);

--
-- Name: workflow_instances_tenant_status_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX workflow_instances_tenant_status_idx ON public.workflow_instances USING btree (tenant_id, status);

--
-- Name: workflow_steps_assigned_to_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX workflow_steps_assigned_to_idx ON public.workflow_steps USING btree (assigned_to) WHERE ((status)::text = 'active'::text);

--
-- Name: workflow_steps_instance_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX workflow_steps_instance_idx ON public.workflow_steps USING btree (instance_id);

--
-- Name: workflow_steps_tenant_id_idx; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX workflow_steps_tenant_id_idx ON public.workflow_steps USING btree (tenant_id);

--
-- Name: credentials credentials_updated_at; Type: TRIGGER; Schema: auth; Owner: -
--

CREATE TRIGGER credentials_updated_at BEFORE UPDATE ON auth.credentials FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();

--
-- Name: roles roles_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER roles_updated_at BEFORE UPDATE ON public.roles FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();

--
-- Name: tenants tenants_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER tenants_updated_at BEFORE UPDATE ON public.tenants FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();

--
-- Name: users users_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER users_updated_at BEFORE UPDATE ON public.users FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();

--
-- Name: workflow_comments workflow_comments_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER workflow_comments_updated_at BEFORE UPDATE ON public.workflow_comments FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();

--
-- Name: workflow_definitions workflow_definitions_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER workflow_definitions_updated_at BEFORE UPDATE ON public.workflow_definitions FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();

--
-- Name: workflow_instances workflow_instances_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER workflow_instances_updated_at BEFORE UPDATE ON public.workflow_instances FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();

--
-- Name: workflow_steps workflow_steps_updated_at; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER workflow_steps_updated_at BEFORE UPDATE ON public.workflow_steps FOR EACH ROW EXECUTE FUNCTION public.update_updated_at();

--
-- Name: display_id_counters display_id_counters_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.display_id_counters
    ADD CONSTRAINT display_id_counters_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;

--
-- Name: roles roles_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.roles
    ADD CONSTRAINT roles_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;

--
-- Name: user_roles user_roles_role_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_roles
    ADD CONSTRAINT user_roles_role_id_fkey FOREIGN KEY (role_id) REFERENCES public.roles(id) ON DELETE CASCADE;

--
-- Name: user_roles user_roles_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_roles
    ADD CONSTRAINT user_roles_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;

--
-- Name: user_roles user_roles_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.user_roles
    ADD CONSTRAINT user_roles_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;

--
-- Name: users users_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;

--
-- Name: workflow_comments workflow_comments_instance_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.workflow_comments
    ADD CONSTRAINT workflow_comments_instance_id_fkey FOREIGN KEY (instance_id) REFERENCES public.workflow_instances(id) ON DELETE CASCADE;

--
-- Name: workflow_comments workflow_comments_posted_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.workflow_comments
    ADD CONSTRAINT workflow_comments_posted_by_fkey FOREIGN KEY (posted_by) REFERENCES public.users(id) ON DELETE RESTRICT;

--
-- Name: workflow_comments workflow_comments_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.workflow_comments
    ADD CONSTRAINT workflow_comments_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;

--
-- Name: workflow_definitions workflow_definitions_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.workflow_definitions
    ADD CONSTRAINT workflow_definitions_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id);

--
-- Name: workflow_definitions workflow_definitions_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.workflow_definitions
    ADD CONSTRAINT workflow_definitions_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;

--
-- Name: workflow_instances workflow_instances_definition_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.workflow_instances
    ADD CONSTRAINT workflow_instances_definition_id_fkey FOREIGN KEY (definition_id) REFERENCES public.workflow_definitions(id);

--
-- Name: workflow_instances workflow_instances_initiated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.workflow_instances
    ADD CONSTRAINT workflow_instances_initiated_by_fkey FOREIGN KEY (initiated_by) REFERENCES public.users(id);

--
-- Name: workflow_instances workflow_instances_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.workflow_instances
    ADD CONSTRAINT workflow_instances_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;

--
-- Name: workflow_steps workflow_steps_assigned_to_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.workflow_steps
    ADD CONSTRAINT workflow_steps_assigned_to_fkey FOREIGN KEY (assigned_to) REFERENCES public.users(id) ON DELETE SET NULL;

--
-- Name: workflow_steps workflow_steps_instance_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.workflow_steps
    ADD CONSTRAINT workflow_steps_instance_id_fkey FOREIGN KEY (instance_id) REFERENCES public.workflow_instances(id) ON DELETE CASCADE;

--
-- Name: workflow_steps workflow_steps_tenant_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.workflow_steps
    ADD CONSTRAINT workflow_steps_tenant_id_fkey FOREIGN KEY (tenant_id) REFERENCES public.tenants(id) ON DELETE CASCADE;

--
-- Name: credentials; Type: ROW SECURITY; Schema: auth; Owner: -
--

ALTER TABLE auth.credentials ENABLE ROW LEVEL SECURITY;

--
-- Name: credentials tenant_isolation; Type: POLICY; Schema: auth; Owner: -
--

CREATE POLICY tenant_isolation ON auth.credentials TO ringiflow_app USING ((tenant_id = (NULLIF(current_setting('app.tenant_id'::text, true), ''::text))::uuid)) WITH CHECK ((tenant_id = (NULLIF(current_setting('app.tenant_id'::text, true), ''::text))::uuid));

--
-- Name: display_id_counters; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.display_id_counters ENABLE ROW LEVEL SECURITY;

--
-- Name: roles; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.roles ENABLE ROW LEVEL SECURITY;

--
-- Name: display_id_counters tenant_isolation; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY tenant_isolation ON public.display_id_counters TO ringiflow_app USING ((tenant_id = (NULLIF(current_setting('app.tenant_id'::text, true), ''::text))::uuid)) WITH CHECK ((tenant_id = (NULLIF(current_setting('app.tenant_id'::text, true), ''::text))::uuid));

--
-- Name: roles tenant_isolation; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY tenant_isolation ON public.roles TO ringiflow_app USING (((tenant_id = (NULLIF(current_setting('app.tenant_id'::text, true), ''::text))::uuid) OR (tenant_id IS NULL))) WITH CHECK (((tenant_id = (NULLIF(current_setting('app.tenant_id'::text, true), ''::text))::uuid) OR (tenant_id IS NULL)));

--
-- Name: tenants tenant_isolation; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY tenant_isolation ON public.tenants TO ringiflow_app USING ((id = (NULLIF(current_setting('app.tenant_id'::text, true), ''::text))::uuid)) WITH CHECK ((id = (NULLIF(current_setting('app.tenant_id'::text, true), ''::text))::uuid));

--
-- Name: user_roles tenant_isolation; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY tenant_isolation ON public.user_roles TO ringiflow_app USING ((tenant_id = (NULLIF(current_setting('app.tenant_id'::text, true), ''::text))::uuid)) WITH CHECK ((tenant_id = (NULLIF(current_setting('app.tenant_id'::text, true), ''::text))::uuid));

--
-- Name: users tenant_isolation; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY tenant_isolation ON public.users TO ringiflow_app USING ((tenant_id = (NULLIF(current_setting('app.tenant_id'::text, true), ''::text))::uuid)) WITH CHECK ((tenant_id = (NULLIF(current_setting('app.tenant_id'::text, true), ''::text))::uuid));

--
-- Name: workflow_comments tenant_isolation; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY tenant_isolation ON public.workflow_comments TO ringiflow_app USING ((tenant_id = (NULLIF(current_setting('app.tenant_id'::text, true), ''::text))::uuid)) WITH CHECK ((tenant_id = (NULLIF(current_setting('app.tenant_id'::text, true), ''::text))::uuid));

--
-- Name: workflow_definitions tenant_isolation; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY tenant_isolation ON public.workflow_definitions TO ringiflow_app USING ((tenant_id = (NULLIF(current_setting('app.tenant_id'::text, true), ''::text))::uuid)) WITH CHECK ((tenant_id = (NULLIF(current_setting('app.tenant_id'::text, true), ''::text))::uuid));

--
-- Name: workflow_instances tenant_isolation; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY tenant_isolation ON public.workflow_instances TO ringiflow_app USING ((tenant_id = (NULLIF(current_setting('app.tenant_id'::text, true), ''::text))::uuid)) WITH CHECK ((tenant_id = (NULLIF(current_setting('app.tenant_id'::text, true), ''::text))::uuid));

--
-- Name: workflow_steps tenant_isolation; Type: POLICY; Schema: public; Owner: -
--

CREATE POLICY tenant_isolation ON public.workflow_steps TO ringiflow_app USING ((tenant_id = (NULLIF(current_setting('app.tenant_id'::text, true), ''::text))::uuid)) WITH CHECK ((tenant_id = (NULLIF(current_setting('app.tenant_id'::text, true), ''::text))::uuid));

--
-- Name: tenants; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.tenants ENABLE ROW LEVEL SECURITY;

--
-- Name: user_roles; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.user_roles ENABLE ROW LEVEL SECURITY;

--
-- Name: users; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.users ENABLE ROW LEVEL SECURITY;

--
-- Name: workflow_comments; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.workflow_comments ENABLE ROW LEVEL SECURITY;

--
-- Name: workflow_definitions; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.workflow_definitions ENABLE ROW LEVEL SECURITY;

--
-- Name: workflow_instances; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.workflow_instances ENABLE ROW LEVEL SECURITY;

--
-- Name: workflow_steps; Type: ROW SECURITY; Schema: public; Owner: -
--

ALTER TABLE public.workflow_steps ENABLE ROW LEVEL SECURITY;

--
-- PostgreSQL database dump complete
--

