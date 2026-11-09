# 10 見開きのページ

作品（`.cmc`）のページ管理で「見開きページに変更」したページの `.clip`。CSP EX 5.1.4 が作った見開きと、同じ作品の単ページを比べ、
同じ形に書いたファイルを CSP EX 5.1.4 で開いて確かめた（確認済み。2 ページがノドで合わさり、作品情報とノンブルが両方のページに出て、開いても未保存の印が付かない）。

見開きは 2 ページ分の幅を持つ 1 つのファイルで、単ページ 2 つを並べたものではない。作品の側（`CanvasNode` の `PageFlag` と `LinkWidth`、ページの番号）は 07 章。

## キャンバス

| 列 | 単ページ | 見開き |
|---|---|---|
| `CanvasWidth` | W | 2W（キャンバスの単位のまま 2 倍。px は単位から丸め直す。297 mm・600 dpi なら 7016 px → 594 mm = 14031 px） |
| `CanvasHeight` | H | H |
| `CanvasDoublePage` | 0 か列なし | 1 |
| `ComicIsLeftPage` | 0 / 1 | 0 |
| `ComicPageIndex` | ページの位置 | 2 ページのうち番号の若いほうの位置（右綴じなら右のページ） |
| `CropFrameOptionFlag` | ビット 1（2）= 左、ビット 2（4）= 右 | 左右のビットがなく、17（ビット 0 と 4）が立つ |
| `ComicStoryRightLayerCreated`、`ComicNombreRightLayerCreated`、`ComicNombreLayerCreated` | 列がない | 1（作品情報のレイヤーがあるとき） |

- 枠の値（`CropFrameWidth` / `Height`、`CropFrameInner*`、裁ち落とし）は **1 ページ分のまま**。2 ページの仕上がり枠はノド（キャンバスの真ん中）で合わさる。
  仕上がりの幅を T px、見開きの幅を W2 px とすると、左のページの仕上がりは `ceil((W2 − 2T) / 2)` から、右のページはその値 + T（ノド）から始まる。

## 作品情報とノンブル

- `ComicStoryInfoType` 1（作品情報）と 2（ノンブル）のテキストレイヤー（03 章、`LayerType` 800）が、ページごとに 1 組、計 2 組ある。
- 右のページの組は `Layer.ComicStoryInfoForRightPage` = 1、左のページの組は NULL（単ページのファイルにはこの列がない）。レイヤー名は 4 つとも単ページと同じ。
- 文字のブロックの外接矩形（06 章のレコード 0x2A。レイヤーの座標で、`LayerOffsetX` を足すとキャンバスの座標）は、単ページでの位置を各ページの仕上がりまでずらしたもの。
  ノンブルはノド側の端が入れ替わる（単ページの左のページの置き方を左に、右のページの置き方を右に使う）。
- CSP が描いた文字の画像（レコード 0x32 が指す `Offscreen`）は、文字や位置を変えたら消す。残すと古い画像が出る。CSP は開いたときに作り直す。

## キャンバスの大きさで決まるもの

幅を変えたら、次も合わせる。合っていないと CSP は開けるが、表示が単ページの幅で切れる。

- レイヤーの描画用の `Offscreen`（04 章）。ルートフォルダーはキャンバスちょうど、ほかのレイヤーは 256 の倍数に切り上げた大きさで、ミップマップの各段は前の段の半分（切り捨て）。
  `Attribute` の Parameter 節の幅・高さ・ブロック数と、BlockSize 節のブロックごとの長さを書き直す。画素を持たない段（外部チャンクがない段）は ID だけのままでよい。
- `LayerThumbnail` の `ThumbnailCanvasWidth` / `Height`（キャンバスの px。サムネイル自体は 512×512 のまま）。
- `CanvasPreview`（キャンバスの 1/8 ほど。見開きなら 1753×1240）と、その `ImageWidth` / `ImageHeight`。

## ライブラリ

- `ClipFile::make_double_page`（`story::make_double_page`）: 上の表のキャンバスの設定と、`LayerThumbnail` のキャンバスの大きさ。
- `ClipFile::set_offscreen`: `Offscreen` の `Attribute` と画素の差し替え。`ClipFile::set_preview`: プレビューとその大きさ。
- 右のページの作品情報のレイヤーは、左のページのものを `ClipFile::copy_layer_from` で写し、`ComicStoryInfoForRightPage` を 1 にして、
  文字のブロックを `story::edit_text_blocks`・`story::map_bbox_x` でずらす（`story::drop_render_cache` で描いた画像を消す）。

画素の広げ方（どのレイヤーをどこに置くか）は使う側が決める。Name Studio の `clip-core` の `write::spread` が例。
