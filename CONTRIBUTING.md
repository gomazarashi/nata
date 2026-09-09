# 貢献ガイド

## 前提

- Rust (開発時 1.95.0)
- `qpdf` (PDF再構成)、`pdftoppm` (PNG描画)。同梱しないため各自用意する
- 改行は LF (`.gitattributes` 参照)。`/target` と `*.pdf` はコミットしない

## ブランチ運用

- `main`: リリース専用。タグ (`vX.Y.Z`) と対応させる。直作業・直pushしない
- `develop`: 統合用。機能はここへ集める
- `feature/*`: `develop` 起点で作業し、`develop` へPRする。`main` へ直接PRしない

## 開発手順

1. `develop` を最新化して `feature/*` を切る
2. 実装と単体テスト (プローブは `src/test_support.rs` 参照) を追加する
3. 仕様変更があれば `README.md`・`docs/command-spec.md` を更新する
4. `docs/plan/todo.md` の該当項目を更新する
5. `CHANGELOG.md` の `[Unreleased]` に `### Added` で追記する (版上げはリリース時)
6. 下記検証を通してコミットし、`develop` へPRする

コミットメッセージは日本語・簡潔に (例: `splitコマンド追加と関連ドキュメント更新`)。

## 検証

```bash
cargo fmt --check
cargo clippy --all-targets
cargo test
```

`cargo fmt --check` が失敗したら `cargo fmt` を適用する。PRにCIはないため、手元で上記3点を必ず通す。

## リリース手順 (develop -> main)

1. `develop` で `Cargo.toml` の版上げと `CHANGELOG.md` の `[Unreleased]` -> `vX.Y.Z` 昇格を行う
2. `main` へ `Release vX.Y.Z` のPRを出し、マージ後にタグ `vX.Y.Z` を打つ
3. `main` を `develop` へ逆マージする (例: `main反映`)
