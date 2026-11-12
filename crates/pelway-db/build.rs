// sqlx::migrate! はコンパイル時に migrations/ を埋め込むが、プロシージャル
// マクロはディレクトリ内のファイル追加を検知できない。migrations/ の変更で
// 必ず再ビルドされるようにする（sqlx のドキュメントが案内する標準の対策）。
fn main() {
    println!("cargo:rerun-if-changed=migrations");
}
