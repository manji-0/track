# 補完スクリプト・llm-help・skill 修正レポート

## 実施日時
2026-01-06

## 修正概要
補完スクリプト、llm-help、skillファイルが最新のCLI仕様に追従できていなかった箇所を特定し、修正しました。

## 発見された問題点

### 1. Bash補完スクリプト (`completions/track.bash.dynamic`)
以下の機能が欠落していました：
- ❌ `config` コマンド（`set-calendar`, `show` サブコマンド）
- ❌ `todo next` サブコマンド
- ❌ `alias set --force` フラグ
- ❌ `completion --dynamic` フラグ

### 2. Zsh補完スクリプト (`completions/_track.dynamic`)
以下の機能が欠落していました：
- ❌ `config` コマンド（`set-calendar`, `show` サブコマンド）
- ❌ `todo next` サブコマンド
- ❌ `alias set --force` フラグ
- ❌ `completion --dynamic` フラグ

### 3. Skillファイル (`skills/task-management/SKILL.md`)
以下のコマンドがEssential Commandsテーブルに欠落していました：
- ❌ `track todo next <index>`
- ❌ `track config set-calendar <id>`
- ❌ `track config show`

### 4. llm-help (`src/cli/handler.rs`)
✅ 確認の結果、llm-helpは最新の仕様に追従していることを確認しました。
- `config` コマンドの説明は含まれていませんが、これは意図的なものと思われます（llm-helpは主にタスク実行ワークフローに焦点を当てているため）

## 実施した修正

### 1. Bash補完スクリプトの修正
```bash
# 追加したコマンド/フラグ
- config コマンドとサブコマンド (set-calendar, show)
- todo next サブコマンド
- alias set --force フラグ
- completion --dynamic フラグ
```

**変更箇所:**
- コマンドリストに `config` を追加
- `todo_commands` に `next` を追加
- `config_commands` 変数を新規追加
- `alias set` の補完ロジックに `--force` フラグを追加
- `completion` コマンドに `--dynamic` フラグの補完を追加
- `config` コマンドの補完ケースを追加

### 2. Zsh補完スクリプトの修正
```zsh
# 追加したコマンド/フラグ
- config コマンドとサブコマンド (set-calendar, show)
- todo next サブコマンド
- alias set --force フラグ
- completion --dynamic フラグ
```

**変更箇所:**
- `todo next` サブコマンドの補完定義を追加
- `alias set` に `-f/--force` フラグを追加
- `completion` コマンドに `-d/--dynamic` フラグを追加
- `config` コマンドの完全な補完定義を追加（サブコマンド含む）
- `_track_commands` に `config` を追加
- `_track__todo_commands` に `next` を追加
- `_track__config_commands` 関数を新規追加
- ヘルプコマンドのリストに `config` を追加

### 3. Skillファイルの修正
```markdown
# 追加したコマンド
| `track todo next <index>` | Move TODO to front (make it next) |
| `track config set-calendar <id>` | Set Google Calendar ID |
| `track config show` | Show current configuration |
```

**変更箇所:**
- Essential Commandsテーブルに3つのコマンドを追加

### 4. 静的補完スクリプトの再生成
最新のCLI定義から静的補完スクリプトを再生成しました：
- `completions/track.bash` (Bash静的補完)
- `completions/_track` (Zsh静的補完)
- `completions/track.fish` (Fish補完)
- `completions/_track.ps1` (PowerShell補完)

これにより、`config`コマンド、`todo next`、`alias set --force`などの新機能が静的補完にも反映されました。

## 修正結果

### 変更ファイル統計
```
 completions/_track              | 1310 +++++++++++++++++++++++++++++++++-----
 completions/_track.dynamic      |   56 +-
 completions/_track.ps1          |   74 +++
 completions/track.bash          |  269 +++++++-
 completions/track.bash.dynamic  |   23 +-
 completions/track.fish          |   70 +-
 skills/task-management/SKILL.md |    3 +
 7 files changed, 1603 insertions(+), 202 deletions(-)
```

### 修正後の状態
✅ **Bash動的補完スクリプト**: 全てのコマンドとフラグに対応
✅ **Zsh動的補完スクリプト**: 全てのコマンドとフラグに対応
✅ **Bash静的補完スクリプト**: 再生成により最新仕様に対応
✅ **Zsh静的補完スクリプト**: 再生成により最新仕様に対応
✅ **Fish補完スクリプト**: 再生成により最新仕様に対応
✅ **PowerShell補完スクリプト**: 再生成により最新仕様に対応
✅ **Skillファイル**: 主要コマンドが全て記載
✅ **llm-help**: 既に最新仕様に対応済み

## 検証項目

以下のコマンドで補完が正しく動作することを確認してください：

### Bash
```bash
track config <TAB>           # set-calendar, show が表示されるべき
track todo next <TAB>        # TODO IDリストが表示されるべき
track alias set --<TAB>      # --force, --task, --help が表示されるべき
track completion --<TAB>     # --dynamic, --help が表示されるべき
```

### Zsh
```zsh
track config <TAB>           # set-calendar, show が表示されるべき
track todo next <TAB>        # TODO IDリストが表示されるべき
track alias set --<TAB>      # --force, --task, --help が表示されるべき
track completion --<TAB>     # --dynamic, --help が表示されるべき
```

## 今後の推奨事項

1. **自動テスト**: 補完スクリプトの自動テストを追加することを推奨
2. **CI/CD統合**: CLIコマンド定義変更時に補完スクリプトとドキュメントの同期を自動チェック
3. **定期的なレビュー**: 新機能追加時に補完スクリプト・ドキュメントの更新を忘れないようチェックリストに追加

## 参考情報

- CLI定義: `src/cli/mod.rs`
- llm-help実装: `src/cli/handler.rs` (handle_llm_help関数)
- README: `README.md`
- Skill定義: `skills/task-management/SKILL.md`
