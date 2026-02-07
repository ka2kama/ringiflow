# prompts/plans/

Claude Code の plan mode で作成された計画ファイルを保存する。

## 概要

計画ファイルには、設計段階での思考過程が記録されている:

- 設計判断とその理由
- ブラッシュアップループの記録（設計の反復改善過程）
- 収束確認チェックリスト

セッションログ（`runs/`）が「何をしたか」を記録するのに対し、計画ファイルは「どう考えが変遷したか」を記録する。

## ファイル命名

Claude Code は plan mode でランダムな名前（例: `clever-napping-panda.md`）のファイルを自動生成する。plan mode 終了後（実装完了後）に以下の規則でリネームする。

### 命名規則

| パターン | 用途 | 例 |
|---------|------|-----|
| `{Issue番号}_{トピック}.md` | Issue に紐付く計画 | `288_dev-auth-feature-flag.md` |
| `YYYYMMDD_{トピック}.md` | Issue なしの計画 | `20260207_plan-files-git-management.md` |

- トピックは kebab-case の英語
- plan mode に入るようなトピックは原則 Issue 化する。日付フォールバックは例外的な場合のみ
- 同一 Issue に複数の計画がある場合はトピックで区別する（例: `222_remove-nondeterministic-domain.md`, `222_parameter-struct.md`）

### リネームのタイミング

plan mode 中は Claude Code がファイル名で追跡しているため、**plan mode を抜けた後**にリネームする。`/wrap-up` の一環として実施するのが自然。
