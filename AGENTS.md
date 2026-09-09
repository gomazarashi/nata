# AIエージェント向け作業指針

ブランチ運用・開発手順・バージョニングは `CONTRIBUTING.md` に従うこと。

## 検証

コード変更後は以下を必ず通す:

```bash
cargo fmt --check
cargo clippy --all-targets
cargo test
```

`cargo fmt --check` が失敗したら `cargo fmt` を適用する。

## 作業範囲

- 対象は現在のGit worktreeのみとし、関係のない変更はしない
- commit・push・PR作成/マージは、明示的な依頼がある場合のみ行う
- ドキュメント変更時は `README.md`・`docs/command-spec.md`・`docs/plan/todo.md`・`CHANGELOG.md` の不整合に注意する

## 言葉

- コミットメッセージと応答は日本語・簡潔に
