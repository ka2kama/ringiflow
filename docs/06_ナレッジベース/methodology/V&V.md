# V&V（Verification & Validation）

## 概要

V&V はソフトウェア品質保証の基本概念。「正しいものを作っているか」と「正しく作っているか」という2つの異なる品質活動を区別する。

Barry Boehm が 1984 年に広めた定義:

| 活動 | 問い | 英語の定義 |
|------|------|-----------|
| Validation（妥当性確認） | 正しいものを作っているか？ | Are we building the right product? |
| Verification（検証） | 正しく作っているか？ | Are we building the product right? |

## 標準規格

### IEEE 1012

IEEE 1012（Standard for System, Software, and Hardware Verification and Validation）は V&V プロセスを体系的に定義する標準規格。ソフトウェアライフサイクル全体にわたる V&V 活動を規定する。

### ISO/IEC 12207

ISO/IEC 12207（ソフトウェアライフサイクルプロセス）にも V&V プロセスが含まれている。

### ISO/IEC 25010 との関係

[ISO/IEC 25010](ISO25010.md) はプロダクト品質の「何を測るか」（品質特性）を定義し、V&V は「いつ・どう確認するか」（品質活動）を定義する。両者は補完関係にある。

| フレームワーク | 問い | 役割 |
|--------------|------|------|
| ISO/IEC 25010 | 品質の何を測るか？ | 品質特性の定義（保守性、機能適合性、等） |
| V&V | いつ・どう確認するか？ | 品質活動のタイミングと方法 |

## Validation と Verification の違い

| 観点 | Validation | Verification |
|------|-----------|--------------|
| タイミング | 上流（要件、設計の妥当性） | 下流（実装の正しさ） |
| 対象 | 問題定義、要件、前提 | 設計、コード、テスト |
| 代表的活動 | 要件レビュー、ユーザー受入テスト、プロトタイピング | コードレビュー、単体テスト、静的解析 |
| 失敗の影響 | 正しく作っても、使われない・役に立たない | 正しい問題を解いても、実装が壊れている |

Validation の失敗は Verification では検出できない。逆もまた然り。両方が必要。

## プロジェクトでの適用

[CLAUDE.md](../../../CLAUDE.md) の品質戦略セクションで V&V の2層構造を採用している。

| 層 | 仕組み |
|----|--------|
| Validation | [問題解決フレームワーク](../../../.claude/rules/problem-solving.md)、[Issue 精査](../../04_手順書/04_開発フロー/01_Issue駆動開発.md#既存-issue-の精査) |
| Verification | 守り（欠陥除去）と攻め（設計改善） |

Validation 層は Issue 精査で「正しい問題を解いているか」を検証し、Verification 層は TDD・設計レビュー・品質チェックリストで「正しく作っているか」を検証する。

## 関連リソース

- Boehm, B. (1984) "Verifying and Validating Software Requirements and Design Specifications", IEEE Software
- IEEE 1012-2016: Standard for System, Software, and Hardware Verification and Validation
- ISO/IEC 12207:2017: Systems and software engineering — Software life cycle processes
