---
status: accepted
---

# ルーティンの cron 解釈は `croner` クレートに委譲する

ルーティンの定期実行について、cron 式の解釈（タイムゾーン、月末、曜日と日付の併記）は自前で実装せず、クレート `croner`（Rust 版、Hexagon/croner-rust）に委譲する。`Cargo.toml` は `croner = "4.0"` と宣言し、解決版は `Cargo.lock` で 4.0.3 に固定する。ライセンスは MIT（`deny.toml` の allow リスト内）。この決定は #213 の部品選定であり、ルーティンの安全制限（#194）と実行基盤（キューとの接続）は対象外。

## 採用した記法の範囲

`pelway_schedule::Schedule::parse` は `CronParser::builder().seconds(Seconds::Optional)` で解釈する。受け付けるのは:

- POSIX/Vixie 互換の 5 フィールド（分 時 日 月 曜日）に、先頭の秒フィールドを任意で足した 6 フィールド記法。
- 範囲・リスト・ステップ（`1-5`、`MON,WED,FRI`、`*/15`）、月名・曜日名（JAN-DEC・SUN-SAT、大文字小文字を区別しない）、`?`（`*` と同じ）、ニックネーム（`@hourly` 〜 `@yearly`）。
- 拡張指定子: `L`（月末日・`5#L` など月末の特定曜日）、`#`（第 n 曜日）、`W`（指定日に最も近い平日）、`+`（曜日フィールドの前置きで日付と曜日を AND にする）。

日付（day-of-month）と曜日（day-of-week）を両方限定して書いたときの既定は POSIX の OR 意味（どちらかに合えば発火）とする。AND 意味が要る場合は `+` 前置きか `dom_and_dow(true)` が croner 側に用意されているが、Pelway が受け付ける式の既定は OR に固定した（`crates/pelway-schedule` の結合試験で固定）。

## 比較の結果

- **`croner` 4.0.3（MIT）**: 上の記法をすべて持ち、`chrono::DateTime<Tz>` で評価タイムゾーンを選べる（Asia/Tokyo を渡せる）。参照実装である Rakazo の routines が使う JS 版 croner と同じ式意味を持ち、挙動の読み合わせができる。GitHub で公開されている版は 4.0.3（2026-11-04 リリース）。
- **`cron` 0.15.0（Apache-2.0）**: Rust では普及しているが、秒フィールドが必須で POSIX の 5 フィールド記法をそのまま受け付けず、`L`・`W`・`#`・DOM-AND-DOW の指定子を持たない。月末指定を式で表せないため、#213 の「月末等」の要件を自前で補うことになる。
- **`saffron`（BSD-3-Clause、Cloudflare）**: `L`・`W`・`#` は持つが、評価タイムゾーンを指定する仕組みを持たず、Asia/Tokyo での解釈に向かない。GitHub にリリース版が無く、版の固定の確かめ方も弱い。

`L`（月末指定）とタイムゾーン評価の両方を素直に満たすのは croner だけだった。

## 検証観点（#183 の分類に対応するテスト観点）

Asia/Tokyo（DST なし、通年 UTC+9）を前提とした境界ケースは、`crates/pelway-schedule/tests/schedule.rs` の結合試験で固定した:

| 観点 | 試験 | 期待する振る舞い |
| --- | --- | --- |
| Asia/Tokyo での解釈 | `fires_at_local_time_in_asia_tokyo` | `0 9 * * *` は JST の 09:00 に発火し、`from` と同時刻は次回に回す（二重発火しない） |
| 月末 | `last_day_of_month_via_l` | `L` は 1 月は 31 日、平年 2 月は 28 日、うるう年 2 月は 29 日に発火する |
| うるう年 | `feb_29_skips_non_leap_years` | `29 2` 指定は平年を飛ばして次のうるう年の 2/29 に発火する |
| 曜日と日付の併記 | `dom_and_dow_default_is_posix_or` | `0 9 13 * 5` は「13 日または金曜」の POSIX OR 意味で発火する（AND ではない） |
| 秒フィールド | `six_field_pattern_with_seconds_is_accepted` | 6 フィールド記法を受け付け、秒単位で発火時刻を返す |
| 解釈不能な式 | `invalid_pattern_is_rejected` | panic せず `Err` で返す |

## Consequences

- cron 式の解析・次回発火時刻の計算は `crates/pelway-schedule` の `Schedule` を通して croner に委譲する。cron 解釈の自前実装は置かない。
- 評価タイムゾーンは呼び出し側が `DateTime<Tz>` で渡す。製品のルーティン実行では Asia/Tokyo を渡す。
- 実行基盤（いつ・どこで `next_after` を呼び、発火をキューに登録するか）は別 Issue の仕事であり、このクレートは時刻解釈だけを担う。
- croner の major 更新は API 変更を伴いうるため、更新は Dependabot（#169）の個別 PR で行い、`Schedule` の結合試験で挙動を確認する。
