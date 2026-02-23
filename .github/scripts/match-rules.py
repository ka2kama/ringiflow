#!/usr/bin/env python3
"""PR の変更ファイルにマッチする .claude/rules/ のルールを特定し、内容を出力する。

使い方:
    python3 match-rules.py <changed-files.txt>

入力: 変更ファイル一覧（1 行 1 パス）
出力: マッチしたルールの名前リスト + 各ルールの本文（フロントマター除去済み）
"""

from __future__ import annotations

import re
import sys
from pathlib import Path


def glob_to_regex(pattern: str) -> str:
    """glob パターン（** 対応）を正規表現に変換する。

    変換ルール:
    - **/ → (?:.+/)? （0 個以上のディレクトリ）
    - 末尾 ** → .* （任意のパス）
    - * → [^/]* （単一セグメント内の任意文字列）
    - ? → [^/] （単一セグメント内の任意 1 文字）
    - その他の正規表現メタ文字はエスケープ
    """
    i = 0
    regex: list[str] = []
    n = len(pattern)

    while i < n:
        c = pattern[i]
        if c == "*":
            if i + 1 < n and pattern[i + 1] == "*":
                # ** パターン
                if i + 2 < n and pattern[i + 2] == "/":
                    # **/ → 0 個以上のディレクトリ
                    regex.append("(?:.+/)?")
                    i += 3
                else:
                    # 末尾 ** → 任意のパス
                    regex.append(".*")
                    i += 2
            else:
                # * → 単一セグメント内
                regex.append("[^/]*")
                i += 1
        elif c == "?":
            regex.append("[^/]")
            i += 1
        elif c in r".+^${}()|\\[]":
            regex.append("\\" + c)
            i += 1
        else:
            regex.append(c)
            i += 1

    return "^" + "".join(regex) + "$"


def parse_frontmatter_paths(content: str) -> list[str]:
    """YAML フロントマターから paths パターンを抽出する。

    フォーマット:
    ---
    paths:
      - "pattern1"
      - "pattern2"
    ---
    """
    lines = content.split("\n")
    if not lines or lines[0].strip() != "---":
        return []

    paths: list[str] = []
    in_paths = False
    for line in lines[1:]:
        stripped = line.strip()
        if stripped == "---":
            break
        if stripped == "paths:":
            in_paths = True
            continue
        if in_paths and stripped.startswith("- "):
            pattern = stripped[2:].strip().strip('"').strip("'")
            paths.append(pattern)
        elif in_paths and stripped and not stripped.startswith("- "):
            break

    return paths


def strip_frontmatter(content: str) -> str:
    """YAML フロントマターを除去してルール本文を返す。"""
    lines = content.split("\n")
    if not lines or lines[0].strip() != "---":
        return content

    for i, line in enumerate(lines[1:], 1):
        if line.strip() == "---":
            return "\n".join(lines[i + 1 :]).lstrip("\n")

    return content


def match_rules(
    changed_files: list[str], rules_dir: Path
) -> list[tuple[str, str]]:
    """変更ファイルにマッチするルールを返す。

    Returns: [(ルールファイルパス, フロントマター除去済み本文), ...]
    """
    matched: list[tuple[str, str]] = []

    for rule_file in sorted(rules_dir.glob("*.md")):
        content = rule_file.read_text()
        paths = parse_frontmatter_paths(content)

        if not paths:
            continue

        # いずれかのパターンがいずれかの変更ファイルにマッチすればOK
        rule_matched = False
        for pattern in paths:
            compiled = re.compile(glob_to_regex(pattern))
            for f in changed_files:
                if compiled.match(f):
                    rule_matched = True
                    break
            if rule_matched:
                break

        if rule_matched:
            body = strip_frontmatter(content)
            matched.append((str(rule_file), body))

    return matched


def main() -> None:
    if len(sys.argv) < 2:
        print("Usage: match-rules.py <changed-files.txt>", file=sys.stderr)
        sys.exit(1)

    changed_files_path = sys.argv[1]
    rules_dir = Path(".claude/rules")

    with open(changed_files_path) as f:
        changed_files = [line.strip() for line in f if line.strip()]

    if not changed_files:
        print("<!-- no-matching-rules -->")
        return

    matched = match_rules(changed_files, rules_dir)

    if not matched:
        print("<!-- no-matching-rules -->")
        return

    # マッチしたルールのサマリー
    print(f"マッチしたルール: {len(matched)} 件\n")
    for path, _ in matched:
        print(f"- `{path}`")
    print()

    # 各ルールの本文
    for path, body in matched:
        print(f"### {path}\n")
        print(body)
        print()


if __name__ == "__main__":
    main()
