# nata コマンド仕様書

## 概要

`nata` は、PDF をページ単位で編集する CUI ツールです。

PDF の内容を再レンダリングするのではなく、既存 PDF のページを並べ替えたり抜き出したりして、新しい PDF を出力します。

入力 PDF は直接変更しません。

## 基本構文

```bash
nata <command> [options]
```

`nata` は PDF 処理バックエンドとして `qpdf` を利用します。  
`qpdf` は同梱せず、ユーザー環境にインストール済みのものを使用します。

## 現在使えるコマンド

- `merge`: 複数の PDF を結合する
- `extract`: 指定ページを取り出す
- `split`: PDF を複数ファイルに分割する

## 今後追加予定のコマンド

- `remove`: 指定ページを削除する
- `reorder`: 指定順にページを並べ替える
- `insert`: 別 PDF のページを挿入する
- `replace`: 指定ページを別 PDF に差し替える
- `rotate`: 指定ページを回転する

未実装のコマンドは、実装完了まではCLIに含めません。

## コマンドの共通仕様

- 出力は常に新しい PDF として生成します
- 既存の出力先は、明示的に許可しない限り上書きしません
- 処理失敗時に壊れた出力ファイルを残さないよう、一時ファイル経由で出力します
- ページ番号は 1 始まりです

## 主なオプション

- `-o, --output <file>`: 単一 PDF の出力先
- `-d, --output-dir <dir>`: `split` の出力先ディレクトリ
- `--overwrite`: 既存の出力先を上書きする
- `--quiet`: 通常メッセージを抑制する
- `--verbose`: 詳細ログを出力する
- `--qpdf <path>`: 使用する `qpdf` のパスを明示する
- `--strict`: 維持保証できない文書レベル情報を検出した場合に処理を停止する

## ページ指定

主に以下の形式を想定します。

```text
1
1-5
1,3,5-8
all
last
odd
even
4-last
```

ページ指定の順序は維持され、重複指定も許可する方針です。

## `split`

`split` は次の形式を受け付けます。

```bash
nata split <input> (--ranges <spec>... | --every <n> | --each-page) -d <output-dir>
```

- `--ranges` は指定ごとに 1 出力ファイルを生成します
- `--every <n>` は `n` ページごとに連続分割します
- `--each-page` は 1 ページごとに分割します
- `--ranges` では `odd` / `even` を禁止します
- 出力ファイル名は `<input-stem>-<label>.pdf` を基本とします
- 同名が発生した場合は `-2`, `-3` の suffix を付けます
- 出力ディレクトリが存在しない場合は自動作成します

例:

```bash
nata split input.pdf --each-page -d out
nata split input.pdf --every 2 -d out
nata split input.pdf --ranges 1-2 --ranges 3-last -d out
```

## strict モード

- `--strict` 指定時は、入力 PDF の文書レベル情報を `qpdf --json` で検査します
- 維持保証できない情報を検出した場合は、処理前に停止します
- strict モード違反の終了コードは `5` です

### strict 判定対象

- outlines / bookmarks
- tagged PDF logical structure
- AcroForm / forms
- page labels
- attachments / embedded files
- document-level name trees
- encryption / password protection

### strict モードの扱い

- `merge` と `extract` と `split` で有効です
- `--strict` 未指定時は、これらの情報があっても通常どおり処理を続行します
- 判定は入力 PDF に対してのみ行い、出力後 PDF の差分比較までは行いません

## 対応予定環境

- Ubuntu
- Windows

## 備考

詳細な内部仕様、エラー条件、終了コード、MVP 対象外機能などは実装の進行に合わせて別途整理します。
