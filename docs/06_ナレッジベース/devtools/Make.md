# Make

## 概要

Make は UNIX 系 OS に標準搭載されるビルド自動化ツール。1976 年に開発され、C/C++ プロジェクトのビルドを中心に広く使われてきた。

Makefile にターゲット（タスク）と依存関係を記述し、`make <ターゲット>` で実行する。

## 基本構文

```makefile
# ターゲット: 依存関係
#     コマンド（タブでインデント必須）

build: src/main.c
	gcc -o main src/main.c

clean:
	rm -f main

.PHONY: clean
```

## Make の問題点

Make は歴史あるツールだが、現代的な開発ワークフローでは以下の問題がある。

### 1. タブ vs スペース問題

コマンド行は**タブ文字でインデントしなければならない**。スペースだとエラーになる。

```makefile
# NG: スペースでインデント
build:
    echo "これはエラー"

# OK: タブでインデント
build:
	echo "これは動く"
```

エディタの設定によってはタブが自動でスペースに変換され、見た目では区別がつかないため初学者が混乱しやすい。

### 2. 変数展開の複雑さ

Make には複数の変数展開構文があり、混乱を招く:

```makefile
# Make 変数
VAR = value
$(VAR)       # Make 変数を展開

# シェル変数
$$VAR        # シェル変数を展開（$ を 2 つ）
$${VAR}      # 同上

# 環境変数の参照
${VAR}       # シェルでは動くが Make では $(VAR) と同じ
```

特に Make 変数とシェル変数の違い（`$(VAR)` vs `$$VAR`）は頻繁なミスの原因となる。

### 3. シェルの扱い

各行が独立したシェルで実行されるため、複数行にまたがる処理は工夫が必要:

```makefile
# NG: cd の効果が次の行に引き継がれない
build:
	cd src
	gcc -o main main.c

# OK: 1 行にまとめる（セミコロンと \ で継続）
build:
	cd src && \
	gcc -o main main.c

# または .ONESHELL を使う（GNU Make 3.82+）
.ONESHELL:
build:
	cd src
	gcc -o main main.c
```

### 4. クロスプラットフォームの問題

- Windows には標準で Make がない（MinGW, Cygwin, WSL などが必要）
- BSD Make と GNU Make で機能差がある
- シェルコマンドの互換性問題（`rm` vs `del` など）

### 5. タスク一覧の表示

Make にはタスク一覧を表示する標準的な方法がない:

```makefile
# 自前で help ターゲットを作る必要がある
help:
	@echo "Available targets:"
	@echo "  build  - Build the project"
	@echo "  clean  - Clean build artifacts"
```

コメントからタスク説明を自動抽出する仕組みもあるが、追加の工夫が必要。

### 6. エラーメッセージの分かりにくさ

```
Makefile:3: *** missing separator.  Stop.
```

「タブがない」という意味だが、メッセージからは分かりにくい。

## 代替ツール

| ツール | 特徴 |
|--------|------|
| just | Make 風構文だが問題点を解消。タブ不要、`--list` で一覧表示 |
| Task | YAML で定義。Go 製 |
| cargo-make | TOML で定義。Rust 向け |
| npm scripts | Node.js プロジェクト向け |

本プロジェクトでは just を採用。詳細は [ADR-006](../../05_ADR/006_コマンドランナーの選定.md) を参照。

## Make が適しているケース

問題点はあるが、以下のケースでは Make が適切な選択となりうる:

- C/C++ プロジェクト: ファイルの依存関係と再ビルド判定が必要な場合
- UNIX 環境限定: 追加インストールなしで使える利点がある
- 既存プロジェクト: すでに Makefile が整備されている場合

## 関連リソース

- [GNU Make マニュアル](https://www.gnu.org/software/make/manual/)
- [just - A Handy Way to Save and Run Project-Specific Commands](https://github.com/casey/just)
