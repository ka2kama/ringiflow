# prompts/plans/

Claude Code の plan mode で作成された計画ファイルを保存する。

## 概要

計画ファイルには、設計段階での思考過程が記録されている:

- 設計判断とその理由
- ブラッシュアップループの記録（設計の反復改善過程）
- 収束確認チェックリスト

セッションログ（`runs/`）が「何をしたか」を記録するのに対し、計画ファイルは「どう考えが変遷したか」を記録する。

## ファイル命名

Claude Code が自動生成するランダム名（例: `clever-napping-panda.md`）をそのまま使用する。ファイルの内容は1行目のタイトル（例: `# Plan: #288 ...`）で識別する。

## ディレクトリ構造

フラット構造。月別サブディレクトリは使用しない。

```text
prompts/plans/
├── clever-napping-panda.md
├── abstract-orbiting-codd.md
└── README.md
```
