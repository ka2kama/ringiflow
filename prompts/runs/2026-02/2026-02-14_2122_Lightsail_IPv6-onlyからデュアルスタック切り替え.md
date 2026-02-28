# Lightsail IPv6-only からデュアルスタック切り替え

## 概要

Lightsail インスタンスを IPv6-only（$5/月）からデュアルスタック（$7/月）に切り替えるためのドキュメント・スクリプト変更を実施した。

関連 Issue: #276

## 実施内容

1. ADR-047 作成 — IPv6-only → デュアルスタック切り替えの意思決定を記録
2. ADR-030 の変更履歴にデュアルスタック切り替えの旨を追記
3. `infra/lightsail/deploy.sh` — IPv6 ワークアラウンド（`SCP_TARGET` 分岐）を削除し `SSH_TARGET` に統一
4. `scripts/deploy-lightsail.sh` — IPv6 関連コメントを簡略化
5. `infra/lightsail/README.md` — コスト更新（$5→$7）、IPv6 注意事項セクション削除、マイグレーション手順再構成（sqlx-cli を推奨に変更）
6. ナレッジベース（IPv6-only 環境での Docker 運用）にアーカイブ注記追加

## 判断ログ

特筆すべき判断なし。計画通りの実装。

## 成果物

コミット:
- `aedb202` #276 Switch Lightsail from IPv6-only to dual-stack

作成ファイル:
- `docs/70_ADR/047_LightsailのIPv6-onlyからデュアルスタックへの切り替え.md`

更新ファイル:
- `docs/70_ADR/030_Lightsail個人環境の構築.md`
- `docs/80_ナレッジベース/infra/IPv6-only環境でのDocker運用.md`
- `infra/lightsail/README.md`
- `infra/lightsail/deploy.sh`
- `scripts/deploy-lightsail.sh`
