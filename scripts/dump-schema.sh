#!/usr/bin/env bash
# DB スキーマスナップショットを正規化して stdout に出力する。
# pg_dump の環境依存出力（バージョン行、restrict、extension ブロック）を除去し、
# どの環境でも同一の出力を得る。
#
# Usage: ./scripts/dump-schema.sh <DATABASE_URL>
set -euo pipefail

pg_dump --schema-only --no-owner --no-privileges --no-tablespaces \
    --exclude-table=_sqlx_migrations \
    --exclude-schema=_sqlx_test \
    "$1" \
| awk '
    # バージョン行を除去
    /^-- Dumped from database version/ { next }
    /^-- Dumped by pg_dump version/ { next }
    # PostgreSQL 17 の restrict/unrestrict 行を除去
    /^\\restrict / { next }
    /^\\unrestrict / { next }
    # Extension ブロックの除去:
    # pg_dump は extension を環境依存で出力する（Docker vs ローカルで差異が出る）。
    # extension 定義はマイグレーションで管理するため、スナップショットからは除外する。
    #
    # -- で始まるコメント区切り行をバッファし、次の行が extension ヘッダなら破棄する。
    /^--$/ {
        if (skip) next
        held = held $0 "\n"
        next
    }
    /^-- Name: .*Type: EXTENSION/ || /^-- Name: EXTENSION .*Type: COMMENT/ {
        held = ""
        skip = 1
        next
    }
    /^CREATE EXTENSION/ { next }
    /^COMMENT ON EXTENSION/ { next }
    /^$/ {
        if (skip) { skip = 0; held = ""; next }
        if (held != "") { printf "%s", held; held = "" }
        print
        next
    }
    skip { next }
    {
        if (held != "") { printf "%s", held; held = "" }
        print
    }
    END { if (held != "") printf "%s", held }
' \
| cat -s
