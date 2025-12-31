# WorkTracker CLI 設計仕様書

## 1. 概要
WorkTracker は、開発者の作業ログを「コンテキスト（現在の作業状態）」に基づいて記録・管理するCLIツールです。
ユーザーは現在取り組んでいるタスク（WorkTracker）をセットすることで、都度IDを指定することなくTODOの消化、メモの記録、関連リポジトリの管理を行えます。

## 2. 技術スタック (Rust)
シングルバイナリで高速に動作し、堅牢なエラーハンドリングを実現するためにRustを採用します。

| カテゴリ | クレート | 用途 |
| :--- | :--- | :--- |
| CLI引数解析 | clap (v4.4+) | サブコマンド、フラグ、ヘルプメッセージの自動生成 |
| DB操作 | rusqlite (bundled) | SQLiteへの接続。システム依存を減らすため bundled featureを使用 |
| パス管理 | directories | XDG Base Directory準拠 (~/.local/share/...) のパス解決 |
| エラー処理 | anyhow, thiserror | エラー伝播の簡略化とコンテキスト付与 |
| 日付時刻 | chrono | 作業ログのタイムスタンプ管理 |
| 表示整形 | prettytable-rs | リスト表示時のテーブル整形 |

## 3. データベース設計 (SQLite)
データは `$HOME/.local/share/track/track.db` に保存されます。

### 3.1. スキーマ定義

#### app_state
アプリケーションの現在の状態を保持します。
- `key`: TEXT PK (例: 'current_task_id')
- `value`: TEXT

#### tasks (作業コンテキスト)
- `id`: INTEGER PK
- `name`: TEXT (作業名)
- `status`: TEXT (例: 'active', 'archived')
- `created_at`: DATETIME

#### todos (タスク内TODO)
- `id`: INTEGER PK
- `task_id`: INTEGER (FK -> tasks.id)
- `content`: TEXT
- `status`: TEXT (例: 'pending', 'done')
- `created_at`: DATETIME

#### links (汎用的な関連URL)
- `id`: INTEGER PK
- `task_id`: INTEGER (FK -> tasks.id)
- `url`: TEXT
- `title`: TEXT
- `created_at`: DATETIME

#### logs (作業記録/Scrap)
- `id`: INTEGER PK
- `task_id`: INTEGER (FK -> tasks.id)
- `content`: TEXT
- `created_at`: DATETIME

#### git_items (関連リポジトリ/Worktree)
- `id`: INTEGER PK
- `task_id`: INTEGER (FK -> tasks.id)
- `path`: TEXT (リポジトリの絶対パス)
- `branch`: TEXT (登録時のブランチ名)
- `description`: TEXT (任意)

#### repo_links (リポジトリに関連するIssue/PR)
- `id`: INTEGER PK
- `git_item_id`: INTEGER (FK -> git_items.id)
- `url`: TEXT
- `kind`: TEXT (自動判定: 'PR', 'Issue', 'Discussion', 'Link')
- `created_at`: DATETIME

## 4. コマンドインターフェース設計
コマンドは `track` をプレフィックスとして実行します。

### 4.1. コンテキスト管理 (Global Operations)

| コマンド | 引数 | 動作 |
| :--- | :--- | :--- |
| `track new` | `<name>` | 新規タスクを作成し、自動的にそのタスクにswitchする。 |
| `track list` | `--all` | 最近のタスク一覧を表示。現在のタスクには `*` を表示。 |
| `track switch` | `<task_id>` | 作業対象のタスクを切り替える。 |
| `track info` | | 現在のタスクの全情報（TODO, Log, Repo, Link）をまとめて表示。 |

### 4.2. タスクアイテム操作
これらは現在switchされているタスクに対して実行されます。

| カテゴリ | コマンド | 引数 | 動作 |
| :--- | :--- | :--- | :--- |
| **TODO** | `track todo add` | `<text>` | TODOを追加。 |
| | `track todo list` | | TODO一覧を表示。 |
| | `track todo update` | `<id> <status>` | ステータス更新（done等）。 |
| | `track todo delete` | `<id>` | TODO削除。 |
| **Link** | `track link add` | `<url> [title]` | 参考URLを追加。 |
| | `track link list` | | リンク一覧を表示. |
| **Log** | `track log add` | `<content>` | 作業ログ（Scrap）を追加。 |
| | `track log list` | | 時系列順にログを表示。 |

### 4.3. Gitリポジトリ連携 (repo)

| コマンド | 引数 | 動作 |
| :--- | :--- | :--- |
| `track repo add` | `[path]` | 指定パス（省略時はカレント）をGit項目として登録。内部で `git rev-parse` を呼び出しブランチ名を自動保存する。 |
| `track repo list` | | 登録済みリポジトリと、それに紐づくIssue/PRを表示。 |
| `track repo link` | `<repo_id> <url>` | 指定したリポジトリ項目にURLを紐付ける。URLパターンから PR, Issue 等を自動判定する。 |
| `track repo delete` | `<repo_id>` | リポジトリ登録を解除する。 |

## 5. ロジック詳細

### 自動コンテキストスイッチ
`track new` 実行時、DBへのINSERT後、即座に `app_state` テーブルの `current_task_id` を更新します。
以降の `add` 系コマンドは、`app_state` からIDを取得して外部キーとして使用します。

### Git情報の取得
`track repo add` 実行時、Rustの `std::process::Command` を使用して git コマンドをサブプロセスとして実行します。
取得コマンド: `git -C <path> rev-parse --abbrev-ref HEAD`
これにより、Gitライブラリ(git2)への依存を排除し、ビルド時間とバイナリサイズを最適化します。

### URL種別推論
`track repo link` でURLが渡された際、以下の文字列マッチングで `kind` を決定します。
- `/pull/` or `/merge_requests/` -> `PR`
- `/issues/` -> `Issue`
- `/discussions/` -> `Discussion`
- その他 -> `Link`