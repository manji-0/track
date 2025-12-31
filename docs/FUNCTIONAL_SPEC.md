# WorkTracker CLI 機能仕様書

本ドキュメントは、WorkTracker CLIツールの具体的な機能仕様を定義します。

---

## 1. タスク管理機能

### 1.1. `track new <name>` - 新規タスク作成

**概要**: 新しい作業コンテキスト（タスク）を作成し、アクティブなタスクとして設定する。

**入力**:
| 引数/フラグ | 型 | 必須 | 説明 |
|-------------|------|------|------|
| `name` | 文字列 | ✓ | タスク名（任意の長さ、空文字不可） |
| `--ticket` / `-t` | 文字列 | | チケットID（下記形式参照） |
| `--ticket-url` | URL | | チケットURL |

**チケットID形式**:
| プラットフォーム | 形式 | 例 |
|------------------|------|-----|
| Jira | `<PROJECT>-<NUMBER>` | `PROJ-123` |
| GitHub Issue | `<owner>/<repo>/<number>` | `myorg/api/456` |
| GitLab Issue | `<group>/<project>/<number>` | `mygroup/app/789` |

**処理フロー**:
1. `name` が空文字でないことを検証
2. `--ticket` が指定された場合:
   - チケットIDの形式を検証（上記いずれかに一致）
   - `--ticket-url` が未指定の場合、空欄で登録
3. `tasks` テーブルに新規レコードを INSERT
   - `status`: `'active'`
   - `ticket_id`: チケットID（指定時）
   - `ticket_url`: チケットURL（指定時）
   - `created_at`: 現在時刻（UTC）
4. `app_state` テーブルの `current_task_id` を新規タスクのIDに更新
5. 成功メッセージを出力

**出力**:
```
Created task #<id>: <name>
Ticket: <ticket_id> (<ticket_url>)
Switched to task #<id>
```

**エラーケース**:
| 条件 | エラーメッセージ |
|------|------------------|
| 名前が空 | `Error: Task name cannot be empty` |
| チケットID重複 | `Error: Ticket '<ticket_id>' is already linked to task #<existing_id>` |
| DB書き込み失敗 | `Error: Failed to create task: <detail>` |

---

### 1.2. チケットID によるタスク参照

タスクIDが必要なすべてのコマンドで、チケットIDによる参照が可能。

**記法**:
- 数値: タスクID（例: `1`, `42`）
- `t:<ticket_id>`: チケットによる参照（例: `t:PROJ-123`, `t:myorg/api/456`）

**使用例**:
```bash
# タスクIDで切り替え
track switch 1

# チケットIDで切り替え
track switch t:PROJ-123

# GitHub Issue形式のチケットで切り替え
track switch t:myorg/api/456

# チケットIDでエクスポート
track export t:PROJ-123

# チケットIDでアーカイブ
track archive t:PROJ-123
```

**解決フロー**:
1. 引数が数値の場合: そのままタスクIDとして使用
2. `t:` で始まる場合: `tasks.ticket_id` から対応するタスクを検索
3. 該当タスクが見つからない場合: エラー

---

### 1.3. `track ticket <ticket_id> <url>` - 既存タスクにチケット登録

**概要**: 現在のタスク（または指定タスク）にチケット情報を追加・更新する。

**入力**:
| 引数/フラグ | 型 | 必須 | 説明 |
|-------------|------|------|------|
| `ticket_id` | 文字列 | ✓ | チケットID |
| `url` | URL | ✓ | チケットURL |
| `--task` | 整数 | | 対象タスクID（省略時: 現在のタスク） |

**出力**:
```
Linked ticket <ticket_id> to task #<task_id>
URL: <url>
```

---

### 1.4. ブランチ命名規則

チケットIDが登録されているタスクでは、worktree作成時のブランチ名に自動的にチケットIDを使用。

**命名パターン**:
| 条件 | ブランチ名 |
|------|-----------|
| チケットあり + ブランチ名省略 | `task/<ticket_id>` (例: `task/PROJ-123`) |
| チケットあり + ブランチ名指定 | `<ticket_id>/<branch>` (例: `PROJ-123/feat-auth`) |
| チケットなし + ブランチ名省略 | `task-<task_id>-<timestamp>` |
| チケットなし + ブランチ名指定 | 指定されたブランチ名をそのまま使用 |

**worktree add での動作**:
```bash
# チケット PROJ-123 が登録されている場合
track worktree add /path/to/repo
# -> ブランチ: task/PROJ-123

track worktree add /path/to/repo feat-auth
# -> ブランチ: PROJ-123/feat-auth
```

---

### 1.5. `track list` - タスク一覧表示

**概要**: 登録されているタスクの一覧を表示する。

**入力**:
| フラグ | 説明 |
|--------|------|
| `--all` / `-a` | アーカイブ済みを含む全タスクを表示 |
| (default) | `status = 'active'` のタスクのみ表示 |

**処理フロー**:
1. `app_state` から `current_task_id` を取得
2. `tasks` テーブルからレコードを取得（フラグに応じてフィルタ）
3. テーブル形式で出力（現在のタスクには `*` を付与）

**出力例**:
```
  ID | Ticket     | Name              | Status   | Created
-----+------------+-------------------+----------+---------------------
*  1 | PROJ-123   | API実装           | active   | 2025-01-01 10:00:00
   2 | -          | バグ修正          | active   | 2025-01-02 14:30:00
```

---

### 1.6. `track switch <task_id>` - タスク切り替え

**概要**: 作業対象のタスクを切り替える。

**入力**:
| 引数 | 型 | 必須 | 説明 |
|------|------|------|------|
| `task_id` | 整数 | ✓ | 切り替え先のタスクID |

**処理フロー**:
1. 指定IDのタスクが存在することを検証
2. タスクの `status` が `'active'` であることを検証
3. `app_state` の `current_task_id` を更新

**出力**:
```
Switched to task #<id>: <name>
```

**エラーケース**:
| 条件 | エラーメッセージ |
|------|------------------|
| タスクが存在しない | `Error: Task #<id> not found` |
| アーカイブ済み | `Error: Task #<id> is archived` |

---

### 1.7. `track info` - 現在タスク詳細表示

**概要**: 現在のタスクに関連する全情報を一覧表示する。

**処理フロー**:
1. `app_state` から現在の `task_id` を取得
2. 以下の関連データを取得・整形して出力:
   - タスク基本情報（名前、チケット、作成日時）
   - TODO一覧（ステータス別にグループ化）
   - リンク一覧
   - Scrap一覧（直近5件）
   - 関連Worktree一覧

**出力例**:
```
=== Task #1: API実装 ===
Ticket: PROJ-123 (https://jira.example.com/browse/PROJ-123)
Created: 2025-01-01 10:00:00

[ TODOs ]
  [ ] エンドポイント設計
  [x] スキーマ定義

[ Links ]
  - Figma設計書: https://figma.com/...

[ Recent Scraps ]
  [10:30] DB設計を完了

[ Worktrees ]
  #1 /home/user/api-worktrees/task/PROJ-123 (task/PROJ-123)
      └─ PR: https://github.com/.../pull/123
```

---

## 2. TODO管理機能

### 2.1. `track todo add <text>` - TODO追加

**概要**: 現在のタスクにTODOを追加する。

**入力**:
| 引数 | 型 | 必須 | 説明 |
|------|------|------|------|
| `text` | 文字列 | ✓ | TODO内容 |

**処理フロー**:
1. 現在のタスクIDを取得（未設定の場合エラー）
2. `todos` テーブルにレコードを INSERT
   - `status`: `'pending'`
   - `created_at`: 現在時刻

**出力**:
```
Added TODO #<id>: <text>
```

**エラーケース**:
| 条件 | エラーメッセージ |
|------|------------------|
| タスク未選択 | `Error: No active task. Run 'track new' or 'track switch' first.` |

---

### 2.2. `track todo list` - TODO一覧表示

**概要**: 現在のタスクのTODO一覧を表示する。

**出力例**:
```
  ID | Status  | Content
-----+---------+---------------------------
   1 | pending | エンドポイント設計
   2 | done    | スキーマ定義
```

---

### 2.3. `track todo update <id> <status>` - TODOステータス更新

**概要**: 指定したTODOのステータスを更新する。

**入力**:
| 引数 | 型 | 必須 | 説明 |
|------|------|------|------|
| `id` | 整数 | ✓ | TODO ID |
| `status` | 文字列 | ✓ | 新しいステータス |

**有効なステータス値**:
- `pending`: 未完了
- `done`: 完了
- `cancelled`: キャンセル

**出力**:
```
Updated TODO #<id> status to '<status>'
```

---

### 2.4. `track todo delete <id>` - TODO削除

**概要**: 指定したTODOを削除する。

**入力**:
| 引数/フラグ | 型 | 必須 | 説明 |
|-------------|------|------|------|
| `id` | 整数 | ✓ | 削除するTODO ID |
| `--force` / `-f` | フラグ | | 確認プロンプトをスキップ |

**処理フロー**:
1. 指定IDのTODOが存在することを検証
2. `--force` が指定されていない場合、確認プロンプトを表示
3. ユーザーが `y` または `yes` を入力した場合のみ削除を実行

**確認プロンプト**:
```
Delete TODO #<id>: "<content>"? [y/N]: 
```

**出力**:
```
Deleted TODO #<id>
```

**キャンセル時**:
```
Cancelled.
```

---

## 3. リンク管理機能

### 3.1. `track link add <url> [title]` - リンク追加

**概要**: 現在のタスクに参考URLを追加する。

**入力**:
| 引数 | 型 | 必須 | 説明 |
|------|------|------|------|
| `url` | 文字列 | ✓ | URL（http/https で始まる） |
| `title` | 文字列 | | リンクのタイトル（省略時はURLをそのまま使用） |

**処理フロー**:
1. URLの形式を検証（http:// または https:// で始まる）
2. `links` テーブルにレコードを INSERT

**出力**:
```
Added link #<id>: <title>
```

---

### 3.2. `track link list` - リンク一覧表示

**出力例**:
```
  ID | Title                | URL
-----+----------------------+--------------------------------
   1 | Figma設計書          | https://figma.com/file/...
   2 | API仕様書            | https://docs.example.com/...
```

---

## 4. Scrap（作業メモ）管理機能

### 4.1. `track scrap add <content>` - Scrap追加

**概要**: 作業メモ（Scrap）を追加する。一時的な思考やメモを時系列で記録する。

**入力**:
| 引数 | 型 | 必須 | 説明 |
|------|------|------|------|
| `content` | 文字列 | ✓ | メモ内容 |

**出力**:
```
Added scrap at <timestamp>
```

---

### 4.2. `track scrap list` - Scrap一覧表示

**概要**: 時系列順にScrapを表示する。

**出力例**:
```
[2025-01-01 10:30:00]
  DB設計を完了。テーブル構成は DESIGN.md を参照。

[2025-01-01 14:15:00]
  API実装開始。まずは認証周りから。
```

---

## 5. Worktree（リポジトリ作業ディレクトリ）連携機能

Git worktree を活用し、タスクごとに独立した作業ディレクトリを管理する。
タスクのライフサイクルに連動して worktree の作成・削除を自動化する。

### 5.1. `track worktree add <repo_path> [branch]` - Worktree作成・登録

**概要**: 指定したリポジトリに新しい worktree を作成し、現在のタスクに関連付ける。

**入力**:
| 引数 | 型 | 必須 | 説明 |
|------|------|------|------|
| `repo_path` | パス | ✓ | 元となるGitリポジトリのパス |
| `branch` | 文字列 | | 作成するブランチ名（省略時: `task-<task_id>-<timestamp>`） |

**処理フロー**:
1. `repo_path` がGitリポジトリであることを検証
2. ブランチ名を決定（指定 or 自動生成）
3. worktree 用のディレクトリパスを決定
   - デフォルト: `<repo_path>/../<repo_name>-worktrees/<branch>`
4. git worktree を作成
   - コマンド: `git -C <repo_path> worktree add -b <branch> <worktree_path>`
5. `git_items` テーブルにレコードを INSERT
   - `path`: worktree のパス
   - `branch`: ブランチ名
   - `base_repo`: 元リポジトリのパス
   - `status`: `'active'`

**出力**:
```
Created worktree: <worktree_path>
Branch: <branch>
Linked to task #<task_id>
```

**エラーケース**:
| 条件 | エラーメッセージ |
|------|------------------|
| リポジトリでない | `Error: <repo_path> is not a Git repository` |
| ブランチが既存 | `Error: Branch '<branch>' already exists` |
| worktree作成失敗 | `Error: Failed to create worktree: <detail>` |

---

### 5.2. `track worktree list` - Worktree一覧表示

**概要**: 現在のタスクに関連付けられた worktree 一覧を表示する。

**出力例**:
```
  ID | Path                              | Branch      | Status | Links
-----+-----------------------------------+-------------+--------+------------------
   1 | /home/user/api-worktrees/feat-x  | feat-x      | active | PR: #123
   2 | /home/user/web-worktrees/task-1  | task-1      | active | Issue: #45
```

---

### 5.3. `track worktree link <worktree_id> <url>` - WorktreeにURL紐付け

**概要**: 登録済み worktree にIssue/PRなどのURLを紐付ける。

**入力**:
| 引数 | 型 | 必須 | 説明 |
|------|------|------|------|
| `worktree_id` | 整数 | ✓ | Worktree ID |
| `url` | 文字列 | ✓ | 紐付けるURL |

**URL種別自動判定**:
| パターン | 判定結果 |
|----------|----------|
| `/pull/` または `/merge_requests/` | `PR` |
| `/issues/` | `Issue` |
| `/discussions/` | `Discussion` |
| その他 | `Link` |

**出力**:
```
Added <kind> link to worktree #<worktree_id>: <url>
```

---

### 5.4. `track worktree remove <worktree_id>` - Worktree削除

**概要**: worktree の登録を解除し、ディスク上の worktree も削除する。

**入力**:
| 引数/フラグ | 型 | 必須 | 説明 |
|-------------|------|------|------|
| `worktree_id` | 整数 | ✓ | 削除する Worktree ID |
| `--force` / `-f` | フラグ | | 確認プロンプトをスキップ |
| `--keep-files` | フラグ | | ディスク上のファイルを保持（登録のみ解除） |

**処理フロー**:
1. 指定IDの worktree が存在することを検証
2. `--force` が指定されていない場合、確認プロンプトを表示
3. ユーザーが承認した場合:
   - `--keep-files` がない場合: `git worktree remove <path>` を実行
   - DBからレコードを削除（関連 `repo_links` も CASCADE 削除）

**確認プロンプト**:
```
Remove worktree #<id>: "<path>" (branch: <branch>)?
This will delete the worktree directory. [y/N]: 
```

**出力**:
```
Removed worktree #<worktree_id>: <path>
```

---

### 5.5. タスクライフサイクル連動

タスクの状態変更に応じて、関連する worktree を自動的に管理する。

#### `track archive <task_id>` - タスクアーカイブ時

**処理フロー**:
1. タスクの `status` を `'archived'` に更新
2. 関連するすべての worktree に対して:
   - 未コミットの変更がないか確認
   - 変更がある場合は警告を表示し、確認を求める
   - 確認後、worktree の `status` を `'archived'` に更新
3. `app_state` の `current_task_id` が該当タスクの場合、クリア

**出力**:
```
Archived task #<task_id>: <name>
  └─ Worktree #1: /path/to/worktree (archived)
  └─ Worktree #2: /path/to/worktree2 (archived)
```

**警告（未コミット変更がある場合）**:
```
WARNING: Worktree #<id> has uncommitted changes:
  M  src/main.rs
  ?? new_file.txt

Archive anyway? [y/N]: 
```

---

#### `track cleanup [--dry-run]` - アーカイブ済みWorktreeの削除

**概要**: `archived` 状態の worktree をディスクから削除する。

**入力**:
| フラグ | 説明 |
|--------|------|
| `--dry-run` | 削除対象を表示するのみ（実際には削除しない） |
| `--force` / `-f` | 確認プロンプトをスキップ |

**処理フロー**:
1. `status = 'archived'` の worktree を全タスクから収集
2. 各 worktree に対して:
   - `git worktree remove <path>` を実行
   - DBから該当レコードを削除

**出力（dry-run）**:
```
Would remove:
  Task #1 (API実装):
    └─ /home/user/api-worktrees/feat-auth
  Task #3 (バグ修正):
    └─ /home/user/web-worktrees/fix-123

Total: 2 worktrees
```

**出力（実行時）**:
```
Removed 2 archived worktrees.
```

---

## 6. 共通仕様

### 6.1. データベースパス

```
$HOME/.local/share/track/track.db
```

XDG Base Directory 仕様に準拠。`directories` クレートを使用。

### 6.2. タイムスタンプ形式

- 保存形式: ISO 8601 (UTC)
- 表示形式: ローカル時刻 `YYYY-MM-DD HH:MM:SS`

### 6.3. 終了コード

| コード | 意味 |
|--------|------|
| `0` | 成功 |
| `1` | 一般的なエラー |
| `2` | 引数エラー |

### 6.4. 共通エラーハンドリング

```rust
// anyhow::Result を使用し、コンテキストを付与
db.execute(...)
    .context("Failed to insert task")?;
```

---

## 7. エクスポート機能（LLM連携）

タスクの全情報を構造化された形式でエクスポートする。
LLMエージェントがタスクレポート生成や作業引き継ぎに活用することを想定。

### 7.1. `track export [task_id]` - タスク情報エクスポート

**概要**: 指定タスク（省略時: 現在のタスク）の全情報を構造化形式で出力する。

**入力**:
| 引数/フラグ | 型 | 必須 | 説明 |
|-------------|------|------|------|
| `task_id` | 整数 | | エクスポート対象のタスクID（省略時: 現在のタスク） |
| `--format` / `-f` | 文字列 | | 出力形式: `markdown`（デフォルト）, `json`, `yaml` |
| `--output` / `-o` | パス | | ファイルに出力（省略時: stdout） |
| `--template` / `-t` | パス | | カスタムテンプレートファイル |

---

### 7.2. 出力形式

#### Markdown形式（デフォルト）

LLMが直接処理しやすい構造化されたMarkdown。

```markdown
# Task Report: API実装

## Metadata
- **Task ID**: 1
- **Status**: active
- **Created**: 2025-01-01 10:00:00
- **Last Activity**: 2025-01-05 15:30:00

## Summary
<!-- LLMによる要約生成用のプレースホルダー -->

## TODOs

### Pending
- [ ] エンドポイント設計
- [ ] 認証処理実装

### Completed
- [x] スキーマ定義
- [x] DB接続設定

## Scraps (Work Log)

### 2025-01-01 10:30
DB設計を完了。テーブル構成は DESIGN.md を参照。

### 2025-01-02 14:15
API実装開始。まずは認証周りから。

## Links
- [Figma設計書](https://figma.com/file/...)
- [API仕様書](https://docs.example.com/...)

## Worktrees

### #1: /home/user/api-worktrees/feat-auth
- **Branch**: feat-auth
- **Status**: active
- **Related**:
  - PR: https://github.com/org/repo/pull/123
  - Issue: https://github.com/org/repo/issues/45
```

---

#### JSON形式

プログラムやLLM APIでの処理に適した構造化データ。

```json
{
  "task": {
    "id": 1,
    "name": "API実装",
    "status": "active",
    "created_at": "2025-01-01T10:00:00Z",
    "last_activity": "2025-01-05T15:30:00Z"
  },
  "todos": [
    {"id": 1, "content": "エンドポイント設計", "status": "pending"},
    {"id": 2, "content": "スキーマ定義", "status": "done"}
  ],
  "scraps": [
    {"timestamp": "2025-01-01T10:30:00Z", "content": "DB設計を完了。..."}
  ],
  "links": [
    {"id": 1, "title": "Figma設計書", "url": "https://..."}
  ],
  "worktrees": [
    {
      "id": 1,
      "path": "/home/user/api-worktrees/feat-auth",
      "branch": "feat-auth",
      "status": "active",
      "repo_links": [
        {"kind": "PR", "url": "https://github.com/.../pull/123"}
      ]
    }
  ]
}
```

---

### 7.3. LLMプロンプト用テンプレート

`--template` オプションで、LLMへの指示を含むカスタムテンプレートを使用可能。

**テンプレート例** (`report_template.md`):
```markdown
あなたはプロジェクトマネージャーです。以下のタスク情報を元に、
進捗報告書を日本語で作成してください。

---

{{task_export}}

---

## 出力要件
1. 完了した作業の要約（箇条書き）
2. 残作業と見積もり時間
3. ブロッカーや懸念事項
4. 次のアクション項目
```

**処理フロー**:
1. テンプレートファイルを読み込む
2. `{{task_export}}` を実際のタスク情報で置換
3. 結果を出力

**出力**:
```
あなたはプロジェクトマネージャーです。以下のタスク情報を元に、
進捗報告書を日本語で作成してください。

---

# Task Report: API実装
...（実際のタスク情報）...

---

## 出力要件
...
```

---

### 7.4. パイプライン連携例

```bash
# LLMにタスクレポート生成を依頼
track export | llm "このタスクの進捗を要約して"

# JSON形式でスクリプトに渡す
track export --format json | jq '.todos[] | select(.status == "pending")'

# ファイルに保存
track export --output ~/reports/task-1-report.md

# カスタムテンプレートでレポート生成
track export --template ~/.config/track/report_template.md | llm
```

---

## 8. 将来拡張（未実装）

以下は現時点で実装対象外だが、将来的に検討:

- `track search <query>`: 全文検索
- `track import`: 外部データのインポート
- MCP Server連携: LLMエージェントからの直接操作

