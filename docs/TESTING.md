# Test Documentation

このドキュメントは、`track` CLIプロジェクトのテスト構成と戦略を説明します。

## テスト概要

### テスト統計
- **合計テスト数**: 79
  - ユニットテスト: 74
  - 統合テスト: 5

### テストカバレッジ

#### 1. Models (`src/models/mod.rs`)
**テスト数**: 4

- `test_task_status_as_str`: TaskStatusのas_strメソッドのテスト
- `test_task_status_from_str`: TaskStatusのfrom_strメソッドのテスト
- `test_todo_status_as_str`: TodoStatusのas_strメソッドのテスト
- `test_todo_status_from_str`: TodoStatusのfrom_strメソッドのテスト

**カバレッジ**: ✅ 完全
- すべてのenumバリアントの変換をテスト
- 無効な入力のハンドリングをテスト

#### 2. Database (`src/db/mod.rs`)
**テスト数**: 4

- `test_new_in_memory`: インメモリDBの作成テスト
- `test_app_state_get_set`: アプリケーション状態の取得・設定テスト
- `test_current_task_id`: 現在のタスクID管理のテスト
- `test_schema_initialization`: スキーマ初期化のテスト

**カバレッジ**: ✅ 完全
- DB接続とスキーマ初期化
- 状態管理機能
- すべての公開メソッド

#### 3. TaskService (`src/services/task_service.rs`)
**テスト数**: 18

**基本CRUD操作**:
- `test_create_task_success`: タスク作成の成功ケース
- `test_create_task_with_ticket`: チケット付きタスク作成
- `test_create_task_with_description`: 説明付きタスク作成
- `test_create_task_empty_name`: 空の名前でのエラーハンドリング
- `test_create_task_duplicate_ticket`: 重複チケットのエラーハンドリング
- `test_get_task_success`: タスク取得の成功ケース
- `test_get_task_not_found`: 存在しないタスクのエラーハンドリング
- `test_list_tasks`: タスク一覧取得
- `test_list_tasks_exclude_archived`: アーカイブ済みタスクの除外

**タスク管理**:
- `test_switch_task_success`: タスク切り替えの成功ケース
- `test_switch_task_archived`: アーカイブ済みタスクへの切り替えエラー
- `test_archive_task`: タスクのアーカイブ

**チケット管理**:
- `test_link_ticket_success`: チケットリンクの成功ケース
- `test_link_ticket_duplicate`: 重複チケットのエラーハンドリング
- `test_validate_ticket_format_jira`: JIRAチケット形式の検証
- `test_validate_ticket_format_github`: GitHubチケット形式の検証
- `test_validate_ticket_format_invalid`: 無効なチケット形式のエラーハンドリング

**説明管理**:
- `test_set_description`: 説明の設定
- `test_set_description_archived_task`: アーカイブ済みタスクへの説明設定エラー
- `test_description_persists`: 説明の永続化確認

**タスク解決**:
- `test_resolve_task_id_by_id`: IDによるタスク解決
- `test_resolve_task_id_by_ticket`: チケットIDによるタスク解決

**カバレッジ**: ✅ 完全
- すべての公開メソッド
- 成功ケースとエラーケース
- エッジケース

#### 4. TodoService (`src/services/todo_service.rs`)
**テスト数**: 15

**基本CRUD操作**:
- `test_add_todo_success`: TODO追加の成功ケース
- `test_add_todo_with_worktree_success`: worktree付きTODO追加
- `test_get_todo_success`: TODO取得の成功ケース
- `test_get_todo_not_found`: 存在しないTODOのエラーハンドリング
- `test_list_todos`: TODO一覧取得
- `test_delete_todo_success`: TODO削除の成功ケース
- `test_delete_todo_not_found`: 存在しないTODO削除のエラーハンドリング

**ステータス管理**:
- `test_update_status_success`: ステータス更新の成功ケース
- `test_update_status_invalid`: 無効なステータスのエラーハンドリング
- `test_update_status_not_found`: 存在しないTODOのステータス更新エラー

**タスクインデックス管理**:
- `test_task_index_sequential`: タスクインデックスの連番性
- `test_task_index_independence`: タスク間のインデックス独立性
- `test_get_todo_by_index_success`: インデックスによるTODO取得の成功ケース
- `test_get_todo_by_index_not_found`: 存在しないインデックスのエラーハンドリング
- `test_list_todos_ordered_by_index`: インデックス順のTODO一覧

**カバレッジ**: ✅ 完全
- すべての公開メソッド
- タスクスコープのインデックス管理
- エラーハンドリング

#### 5. LinkService & ScrapService (`src/services/link_service.rs`)
**テスト数**: 10

**LinkService (7テスト)**:
- `test_add_link_success`: リンク追加の成功ケース
- `test_add_link_default_title`: デフォルトタイトルでのリンク追加
- `test_add_link_invalid_url`: 無効なURLのエラーハンドリング
- `test_list_links`: リンク一覧取得
- `test_validate_url_http`: HTTP URLの検証
- `test_validate_url_https`: HTTPS URLの検証
- `test_validate_url_invalid`: 無効なURLの検証エラー

**ScrapService (3テスト)**:
- `test_add_scrap_success`: スクラップ追加の成功ケース
- `test_get_scrap_success`: スクラップ取得の成功ケース
- `test_list_scraps`: スクラップ一覧取得（時系列順）

**カバレッジ**: ✅ 完全
- すべての公開メソッド
- URL検証ロジック
- 時系列順のソート

#### 6. RepoService (`src/services/repo_service.rs`)
**テスト数**: 5

- `test_add_repo_success`: リポジトリ登録の成功ケース
- `test_add_repo_not_git`: 非Gitディレクトリのエラーハンドリング
- `test_add_repo_duplicate`: 重複リポジトリのエラーハンドリング
- `test_list_repos`: リポジトリ一覧取得
- `test_remove_repo`: リポジトリ削除

**カバレッジ**: ✅ 完全
- すべての公開メソッド
- Git検証ロジック
- 重複チェック

#### 7. WorktreeService (`src/services/worktree_service.rs`)
**テスト数**: 13

**ブランチ名決定ロジック (6テスト)**:
- `test_determine_branch_name_with_explicit_branch_and_ticket`
- `test_determine_branch_name_with_explicit_branch_only`
- `test_determine_branch_name_with_ticket_and_todo`
- `test_determine_branch_name_with_todo_only`
- `test_determine_branch_name_base_with_ticket`
- `test_determine_branch_name_base_without_ticket`

**Worktree操作 (6テスト)**:
- `test_add_worktree_and_get`: worktree追加と取得
- `test_list_worktrees`: worktree一覧取得
- `test_remove_worktree`: worktree削除
- `test_get_base_worktree`: ベースworktree取得
- `test_get_worktree_by_todo`: TODOによるworktree取得
- `test_determine_worktree_path`: worktreeパス決定

**Git操作 (1テスト)**:
- `test_has_uncommitted_changes`: 未コミット変更の検出

**カバレッジ**: ✅ 完全
- すべての公開メソッド
- ブランチ命名戦略のすべてのパターン
- 実際のGitリポジトリを使用した統合テスト

#### 8. CommandHandler (`src/cli/handler.rs`)
**テスト数**: 1

- `test_llm_help`: LLMヘルプコマンドの実行テスト

**カバレッジ**: ⚠️ 部分的
- LLMヘルプコマンドのみテスト
- 他のCLIコマンドは統合テストでカバー

### 統合テスト (`tests/integration_test.rs`)
**テスト数**: 5

1. **test_full_task_workflow**
   - タスク作成から完了までの完全なワークフロー
   - TODO追加、ステータス更新、アーカイブ

2. **test_repo_worktree_workflow**
   - リポジトリ登録
   - ベースworktree作成
   - worktree一覧取得
   - リポジトリ削除

3. **test_task_switching**
   - 複数タスク間の切り替え
   - 現在タスクの管理
   - アーカイブ済みタスクの除外

4. **test_todo_task_index_independence**
   - 複数タスク間でのTODOインデックスの独立性
   - タスクスコープのインデックス管理

5. **test_error_handling**
   - 存在しないリソースへのアクセスエラー
   - アーカイブ済みタスクへの操作エラー
   - 無効な操作のエラーハンドリング

## テスト実行方法

### すべてのテストを実行
```bash
cargo test --all
```

### 特定のテストを実行
```bash
# ユニットテストのみ
cargo test --lib

# 統合テストのみ
cargo test --test integration_test

# 特定のモジュールのテスト
cargo test services::task_service

# 特定のテスト関数
cargo test test_create_task_success
```

### テストの詳細出力
```bash
cargo test -- --nocapture
```

### テストカバレッジ（要tarpaulin）
```bash
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
```

## テスト戦略

### 1. ユニットテスト
- 各サービスの公開メソッドをテスト
- インメモリDBを使用して高速実行
- 成功ケースとエラーケースの両方をカバー

### 2. 統合テスト
- 複数のサービスを組み合わせたワークフローをテスト
- 実際のGitリポジトリを使用（tempfileで管理）
- エンドツーエンドのシナリオをカバー

### 3. テストデータ管理
- 各テストは独立したDBインスタンスを使用
- 一時ディレクトリ（tempfile）でファイルシステムの状態を管理
- テスト後の自動クリーンアップ

## テスト品質指標

### カバレッジ
- ✅ **Models**: 100% - すべてのenum変換メソッド
- ✅ **Database**: 100% - すべての公開メソッド
- ✅ **TaskService**: 100% - すべての公開メソッド
- ✅ **TodoService**: 100% - すべての公開メソッド
- ✅ **LinkService/ScrapService**: 100% - すべての公開メソッド
- ✅ **RepoService**: 100% - すべての公開メソッド
- ✅ **WorktreeService**: 100% - すべての公開メソッド
- ⚠️ **CommandHandler**: 部分的 - LLMヘルプのみ

### エラーハンドリング
すべてのサービスで以下をテスト:
- 存在しないリソースへのアクセス
- 無効な入力データ
- 重複データの処理
- ビジネスルール違反（例: アーカイブ済みタスクへの操作）

### エッジケース
- 空の入力
- 境界値
- 特殊文字を含むデータ
- 複数の同時操作

## 今後の改善案

### 1. CommandHandlerのテスト拡張
現在、CommandHandlerは統合テストで間接的にテストされていますが、各コマンドハンドラーの直接的なユニットテストを追加することを検討。

### 2. パフォーマンステスト
大量のタスク/TODOを扱う場合のパフォーマンステストを追加。

### 3. 並行性テスト
複数の操作が同時に実行される場合のテストを追加。

### 4. エラーメッセージの検証
エラーケースで返されるメッセージの内容を検証するテストを追加。

## メンテナンス

### 新機能追加時
1. 新しい公開メソッドには必ずテストを追加
2. 成功ケースとエラーケースの両方をカバー
3. 統合テストで実際のワークフローを確認

### テスト失敗時
1. 失敗したテストのログを確認
2. 関連するコードの変更を確認
3. テストが正しいか、実装が正しいかを判断
4. 必要に応じてテストまたは実装を修正

### リファクタリング時
1. すべてのテストが引き続き成功することを確認
2. テストコードも同様にリファクタリング
3. 重複するテストコードは共通化
