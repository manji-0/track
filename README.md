# WorkTracker CLI

開発者の作業ログを「コンテキスト（現在の作業状態）」に基づいて記録・管理するCLIツールです。

## 特徴

- **コンテキストベースの作業管理**: 現在のタスクを設定することで、都度IDを指定せずにTODO、メモ、リポジトリを管理
- **チケット統合**: Jira、GitHub Issue、GitLab Issueとの連携
- **Git Worktree管理**: タスクごとに独立した作業ディレクトリを自動管理
- **シンプルなCLI**: 直感的なコマンド体系
- **高速**: Rustによるシングルバイナリ実装

## インストール

```bash
# ビルド
cargo build --release

# インストール（オプション）
cargo install --path .
```

## クイックスタート

```bash
# 新しいタスクを作成
track new "API実装" --ticket PROJ-123 --ticket-url https://jira.example.com/browse/PROJ-123

# タスク一覧を表示
track list

# TODOを追加
track todo add "エンドポイント設計"
track todo add "認証処理実装"

# リンクを追加
track link add https://figma.com/design/... "Figma設計書"

# 作業メモを追加
track scrap add "DB設計を完了。テーブル構成はDESIGN.mdを参照"

# Worktreeを作成
track worktree add /path/to/repo

# 現在のタスク情報を表示
track info
```

## コマンド一覧

### タスク管理

| コマンド | 説明 |
|---------|------|
| `track new <name>` | 新規タスクを作成し、アクティブに設定 |
| `track list [--all]` | タスク一覧を表示 |
| `track switch <task_id>` | タスクを切り替え |
| `track info` | 現在のタスクの詳細情報を表示 |
| `track ticket <ticket_id> <url>` | タスクにチケットを紐付け |
| `track archive <task_id>` | タスクをアーカイブ |

### TODO管理

| コマンド | 説明 |
|---------|------|
| `track todo add <text>` | TODOを追加 |
| `track todo list` | TODO一覧を表示 |
| `track todo update <id> <status>` | TODOステータスを更新 |
| `track todo delete <id>` | TODOを削除 |

### リンク管理

| コマンド | 説明 |
|---------|------|
| `track link add <url> [title]` | 参考URLを追加 |
| `track link list` | リンク一覧を表示 |

### Scrap（作業メモ）管理

| コマンド | 説明 |
|---------|------|
| `track scrap add <content>` | 作業メモを追加 |
| `track scrap list` | メモ一覧を表示 |

### Worktree管理

| コマンド | 説明 |
|---------|------|
| `track worktree add <repo_path> [branch]` | Worktreeを作成 |
| `track worktree list` | Worktree一覧を表示 |
| `track worktree link <worktree_id> <url>` | WorktreeにURLを紐付け |
| `track worktree remove <worktree_id>` | Worktreeを削除 |

## チケット参照

タスクIDの代わりにチケットIDで参照可能：

```bash
# チケットIDでタスクを切り替え
track switch t:PROJ-123

# チケットIDでアーカイブ
track archive t:PROJ-123
```

## ブランチ命名規則

チケットが登録されているタスクでは、Worktree作成時に自動的にチケットIDを使用：

```bash
# チケット PROJ-123 が登録されている場合
track worktree add /path/to/repo
# → ブランチ: task/PROJ-123

track worktree add /path/to/repo feat-auth
# → ブランチ: PROJ-123/feat-auth
```

## データベース

データは以下のパスに保存されます：

```
$HOME/.local/share/track/track.db
```

XDG Base Directory仕様に準拠しています。

## 技術スタック

- **言語**: Rust (Edition 2021)
- **CLI**: clap v4.4+
- **データベース**: SQLite (rusqlite with bundled feature)
- **エラー処理**: anyhow, thiserror
- **日時**: chrono
- **表示**: prettytable-rs

## プロジェクト構造

詳細は [PROJECT_STRUCTURE.md](PROJECT_STRUCTURE.md) を参照してください。

## ドキュメント

- [DESIGN.md](DESIGN.md) - 設計仕様書
- [docs/FUNCTIONAL_SPEC.md](docs/FUNCTIONAL_SPEC.md) - 機能仕様書
- [PROJECT_STRUCTURE.md](PROJECT_STRUCTURE.md) - プロジェクト構造

## ライセンス

MIT License

## 開発

```bash
# 開発ビルド
cargo build

# テスト実行
cargo test

# リリースビルド
cargo build --release
```
