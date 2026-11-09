//! 環境確認用のスモークテスト。製品機能の試験ではない。

use pelway_dev_smoke::dev_environment_ok;

#[test]
fn dev_environment_smoke() {
    assert!(dev_environment_ok());
}
