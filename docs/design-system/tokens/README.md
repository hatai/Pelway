# design-system tokens

Claude Design の書き出しと、その照合に使った検査ツール・結果の置き場。トークン値の正本は `pelway.tokens.json`。

## ファイル

| ファイル | 内容 |
|---|---|
| `pelway.tokens.json` | トークン正本（DTCG 形式の書き出し）。Claude Design round3（2026-11-09、`claude-design-export-round3-2026-11-09`）時点のもの。`crates/pelway-client-ui/tokens/pelway.tokens.json` はこのファイルのバイト単位の写しで、sha256 の一致を `crates/pelway-client-ui/tests/theme_tokens.rs` が検査する |
| `contrast-pairs.json` | コントラスト検査の対象ペア（fg / bg / theme / kind）。kind は decorative・known-exception・retarget-text |
| `expected-values.json` | 検査ツールが期待する値（`meta.round` は 2） |
| `check-result.json` | round2 書き出し（`claude-design-export-round2-2026-11-09`）の検査結果。`ok: true` |
| `check-v4-result.json` | round3 書き出し（`claude-design-export-round3-2026-11-09`）の検査結果。`ok: true`。標準ダークの known-exception の照合結果を含む |
| `tools/check_export_v3.py` | 書き出し照合ツール v3（Round2 拡張: contrast-pairs の kind、個人名禁止、Round2 追加トークン） |

## トークンの出どころ

- 出どころは Claude Design の round2・round3（2026-11-09）。`pelway.tokens.json` と `contrast-pairs.json` は round3 時点の内容で、`pelway.tokens.json` の sha256（`4f523ce1…`）は `docs/design/brief-2026-11-10/PATHMAP.md` が記録する round3 書き出しのハッシュと一致する。
- テーマは 4 つ（light / dark / light-high-contrast / dark-high-contrast）。色トークンは各テーマ 62 キー、`$value` は 514 件（check-result.json の記録どおり）。
- ブランドのアクセントは `#2B36FF`。`#2B37FF` は誤りで、検査ツールはトークン内への混入を失敗として扱う。

## check_export_v3.py の実行方法

```sh
python3 docs/design-system/tokens/tools/check_export_v3.py /path/to/pelway.tokens.json
```

- `contrast-pairs.json` は tokens ファイルと同じディレクトリから自動探索する。
- 期待値は `tools/` の親ディレクトリの `expected-values.json` を読む（この配置では `docs/design-system/tokens/expected-values.json`）。
- 第 2 引数にパスを渡すと個人名検査の対象に追加できる。`tokens/` 直下に `design-system.md` があればそれも検査する。
- 終了コードは検査に通れば 0、失敗なら 1。結果は JSON で標準出力に出る。

### このリポジトリではそのまま実行できない

ツールは元の作業環境（`/workspace/pelway-design-system`）にあった補助ファイルに依存しており、それらはこのリポジトリに取り込んでいない。

- `check_export_v2.py`: `docs/design-system/claude-design-fix-instructions-2026-11-09/tools/` を相対パスで参照する
- `wcag.py`: `/workspace/pelway-design-system/ui-pro-review/tools` を絶対パスで参照する

このため現状は `python3` で実行すると `ModuleNotFoundError: No module named 'wcag'` で止まる。`check-result.json`・`check-v4-result.json` は当時の実行結果の記録。再検査が必要になったら、補助ファイルをこのリポジトリに取り込むか、ツールをこの配置で動くように直す。
