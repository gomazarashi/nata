# nata

`nata`は、PDFをページ単位で編集するRust製CLIツールです。

PDFの内容を再レンダリングするのではなく、既存PDFのページを再構成して新しいPDFを出力します。入力PDFは直接変更しません。

## 前提

`nata`の利用には`qpdf`が必要です。`nata`自体は`qpdf`を同梱せず、`--qpdf`、`NATA_QPDF`、`PATH`の順で使用する実行ファイルを検出します。

## インストール

現時点では、リポジトリを取得した上で次のようにインストールします。

```bash
cargo install --path .
```

インストール後は`nata`コマンドとして実行できます。インストールせずに試す場合は、このリポジトリ直下で`cargo run -- <command> [options]`を実行してください。

## 現在使えるコマンド

- `merge`: 複数のPDFを結合する
- `extract`: 指定ページを取り出す
- `split`: PDFを複数ファイルに分割する

## 今後追加予定のコマンド

- `remove`: 指定ページを削除する
- `reorder`: 指定順にページを並べ替える
- `rotate`: 指定ページを回転する
- `insert`: 別PDFのページを挿入する
- `replace`: 指定ページを別PDFに差し替える

## 使い方

```bash
nata <command> [options]
```

```bash
cargo run -- <command> [options]
```

### 共通オプション

- `--qpdf <path>`: 使用する `qpdf` 実行ファイルのパスを明示する
- `--strict`: 維持保証できない文書レベル情報を検出した場合に処理を停止する
- `--quiet`: 通常メッセージを抑制する
- `--verbose`: 詳細ログを出力する

### merge

```bash
nata merge a.pdf b.pdf -o merged.pdf
nata --strict merge a.pdf b.pdf -o merged.pdf
```

### extract

```bash
nata extract input.pdf --pages 1-3 -o out.pdf
nata extract input.pdf --pages 1,3,last -o out.pdf
nata extract input.pdf --pages odd -o out.pdf
nata --strict extract input.pdf --pages 1-3 -o out.pdf
```

### split

```bash
nata split input.pdf --each-page -d out
nata split input.pdf --every 2 -d out
nata split input.pdf --ranges 1-2 --ranges 3-last -d out
nata --strict split input.pdf --each-page -d out
```

`split` は `<output-dir>/<入力stem>-<label>.pdf` の形式で複数PDFを出力します。`--ranges` では `odd` と `even` は使えません。出力ディレクトリが存在しない場合は自動作成されます。同名出力が発生する場合は `-2`, `-3` のsuffixを付けて共存させます。

ページ指定では`1`, `1-5`, `1,3,5-8`, `all`, `last`, `odd`, `even`, `4-last`を使用できます。順序は維持され、重複指定も保持されます。

## 共通仕様

- 出力は常に新しいPDFとして生成します。
- 既存の出力先は、`--overwrite`を指定しない限り上書きしません。
- 処理失敗時に壊れた出力ファイルを残さないよう、一時ファイル経由で出力します。
- ページ番号は1始まりです。
- `--strict`指定時は、ページ操作で維持保証できない文書レベル情報を検出すると終了コード`5`で停止します。

## strictモード

`--strict`は、入力PDFに次のような文書レベル情報が含まれる場合に処理を止めます。

- outlines / bookmarks
- tagged PDF logical structure
- AcroForm / forms
- page labels
- attachments / embedded files
- document-level name trees
- encryption / password protection

`qpdf --json`の結果をもとに判定するため、実際に停止する項目は入力PDFの構造に依存します。

## License

MIT License
