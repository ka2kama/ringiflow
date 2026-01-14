# Event Sourcing とデータ削除

Event Sourcing における「イベントの不変性」と「個人情報削除要件（GDPR 等）」の衝突は、よく知られた課題である。本ノートでは一般的な対処パターンを解説する。

## 前提: Event Sourcing の原則

Event Sourcing では、状態の変更を「イベント」として記録し、イベントの積み重ねから現在の状態を再構築する。

```
イベントは不変（Immutable）
  - 追記のみ（Append-only）
  - 過去のイベントは書き換えない
  - これが監査証跡・任意時点への再構築の根拠
```

## 問題: 削除要件との衝突

GDPR の「Right to Erasure（忘れられる権利）」等により、個人情報の削除が求められる場合がある。

```
Event #1: ユーザー登録 (email: user@example.com)
Event #2: プロフィール更新 (name: "山田太郎")
Event #3: 注文作成 (address: "東京都...")

→ ユーザーが削除を要求したら、これらのイベントをどうするか？
```

## 対処パターン

### パターン1: Crypto Shredding（暗号シュレッディング）

個人情報を暗号化して保存し、削除時は暗号鍵のみを削除する。

```
保存時:
  Event: {
    email: "encrypted:abc123...",
    name: "encrypted:def456...",
    _key_id: "user-key-001"
  }

削除時:
  鍵 "user-key-001" を削除
  → イベントは残るが復号不可能
```

**利点:**
- イベントの連続性・構造を維持
- 監査時に「イベントが存在した」ことは確認可能

**欠点:**
- 「データが物理的に残っている」ため、厳密には「削除」と言えない
- 将来的に暗号が破られるリスク（理論上）
- 顧客が「物理的に消してくれ」と言った場合に説明が難しい

### パターン2: イベントの匿名化（上書き）

削除対象のフィールドをマスク/ハッシュ化して上書きする。

```
削除前:
  Event #2: { name: "山田太郎", email: "user@example.com" }

削除後:
  Event #2: { name: "[REDACTED]", email: "hash:a1b2c3...", _anonymized_at: "..." }
```

**利点:**
- シンプルな実装
- イベントの因果関係は追える

**欠点:**
- Event Sourcing の「不変性」原則を破る
- 「データは残っている」ことに変わりない

### パターン3: Tombstone イベント

「削除された」というイベントを追加し、読み取り時に過去のイベントを論理的に無効化する。

```
Event #1: ユーザー登録 (email: user@example.com)
Event #2: プロフィール更新 (name: "山田太郎")
Event #3: DataErasureRequested {
            target_events: [1, 2],
            erased_fields: ["email", "name"]
          }

読み取り時:
  Projector が Event #3 を見て、#1, #2 の該当フィールドを無視
```

**利点:**
- 元イベントは不変のまま
- 「消した」という事実も記録される

**欠点:**
- Projector の実装が複雑
- 過去の Snapshot には元データが含まれたまま
- 物理的にはデータが残っている

### パターン4: 物理削除

必要なときはイベントを物理的に削除する。

```
DELETE FROM events WHERE user_id = ?
```

**利点:**
- 「完全に削除した」と言える
- シンプル

**欠点:**
- Event Sourcing の不変性原則を破る
- Snapshot との整合性が壊れる可能性
- 削除後はその集約を再構築できない

## パターン選択の指針

| 状況 | 推奨パターン |
|------|-------------|
| 個人情報が少数フィールドのみ | Crypto Shredding |
| 監査証跡を完全に残したい | Tombstone |
| 顧客が「物理削除」を明確に要求 | 物理削除 |
| テナント単位での削除 | 物理削除（テナント全体なら整合性問題なし） |

## テナント単位削除の特殊性

マルチテナント SaaS において、テナント退会時の削除は特殊なケースである。

```
通常の削除: 一部のイベントを消す → 整合性問題が発生
テナント削除: そのテナントの全イベントを消す → 整合性問題なし
```

テナント退会後にそのテナントの集約を再構築する必要はないため、物理削除しても実害がない。残るテナントの Event Sourcing は影響を受けない。

## 参考資料

- [GDPR and Event Sourcing](https://www.michielrook.nl/2017/11/event-sourcing-gdpr-follow-up/) - Michiel Rook
- [Handling GDPR with Event Sourcing](https://www.eventstore.com/blog/protecting-sensitive-data-in-event-sourced-systems-with-crypto-shredding) - Event Store Blog
- [The Dark Side of Event Sourcing: Managing Data Conversion](https://www.infoq.com/news/2016/04/event-sourcing-data-conversion/) - InfoQ
