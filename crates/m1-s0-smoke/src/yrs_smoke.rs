//! yrs 0.18 (CRDT) 兼容性冒烟
//!
//! 验证目标: yrs 0.18 在 Rust 1.98.0 下编译 + Doc + 简单 put/get 通过

use yrs::{Doc, GetString, Text, Transact};

/// Smoke 入口: 创建一个 Doc, put 一段 text, read 回来
pub fn basic_text_roundtrip() -> String {
    let doc = Doc::new();
    let text = doc.get_or_insert_text("hello");
    {
        let mut txn = doc.transact_mut();
        text.insert(&mut txn, 0, "hello ");
        text.insert(&mut txn, 6, "yrs");
    }
    let txn = doc.transact();
    text.get_string(&txn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yrs_text_insert_and_get() {
        let result = basic_text_roundtrip();
        assert_eq!(result, "hello yrs");
    }
}
