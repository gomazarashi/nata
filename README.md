# nata

`nata`は、PDFをページ単位で編集するRust製CLIツールです。

PDFの内容を再レンダリングするのではなく、既存PDFのページを再構成して新しいPDFを出力します。入力PDFは直接変更しません。

## 前提

`nata`の利用には`qpdf`が必要です。`nata`自体は`qpdf`を同梱せず、`--qpdf`、`NATA_QPDF`、`PATH`の順で使用する実行ファイルを検出します。

## 現在使えるコマンド

- `merge`: 複数のPDFを結合する
- `extract`: 指定ページを取り出す

## 予定しているコマンド

- `remove`: 指定ページを削除する
- `reorder`: 指定順にページを並べ替える
- `rotate`: 指定ページを回転する
- `insert`: 別PDFのページを挿入する
- `replace`: 指定ページを別PDFに差し替える
- `split`: PDFを複数ファイルに分割する

## 使い方

```bash
nata <command> [options]
```

### merge

```bash
nata merge a.pdf b.pdf -o merged.pdf
```

### extract

```bash
nata extract input.pdf --pages 1-3 -o out.pdf
nata extract input.pdf --pages 1,3,last -o out.pdf
nata extract input.pdf --pages odd -o out.pdf
```

ページ指定では`1`, `1-5`, `1,3,5-8`, `all`, `last`, `odd`, `even`, `4-last`を使用できます。順序は維持され、重複指定も保持されます。

## 共通仕様

- 出力は常に新しいPDFとして生成します。
- 既存の出力先は、`--overwrite`を指定しない限り上書きしません。
- 処理失敗時に壊れた出力ファイルを残さないよう、一時ファイル経由で出力します。
- ページ番号は1始まりです。

## License

MIT License
