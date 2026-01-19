---
paths:
  - "frontend/**"
  - "**/elm.json"
  - "**/package.json"
---

# フロントエンド実装ルール

このルールはフロントエンド（`frontend/`）のファイルを編集する際に適用される。

## 依存関係

依存関係を追加する際は、最新の stable バージョンを使用する。

### npm パッケージ

```bash
# pnpm で追加（自動的に最新バージョン）
pnpm add <package>
pnpm add -D <package>  # devDependencies

# または npm view で確認して手動追加
npm view <package> version
```

### Elm パッケージ

```bash
# elm install で追加
elm install <author/package>
```

## AI エージェントへの指示

1. 依存関係を追加する際は最新の stable バージョンを使用する
2. `pnpm add` または `elm install` で追加
3. 更新後は `pnpm install` でロックファイルを同期

## 参照

- 最新プラクティス方針: [latest-practices.md](latest-practices.md)
