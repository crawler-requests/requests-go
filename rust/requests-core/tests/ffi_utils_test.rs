//! FFI 工具函数测试
//!
//! 直接测试 Rust 层的 utils 函数，FFI 包装是薄层，直接映射到这些函数

#[test]
fn test_utils_encode_uri_component() {
    let result = requests_core::encode_uri_component("hello world");
    assert_eq!(result, "hello%20world");
}

#[test]
fn test_utils_md5() {
    let result = requests_core::md5(b"hello");
    assert_eq!(result, "5d41402abc4b2a76b9719d911017c592");
}

#[test]
fn test_utils_sha256() {
    let result = requests_core::sha256(b"hello");
    assert_eq!(
        result,
        "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
    );
}

#[test]
fn test_utils_hmac_sha256() {
    let key = b"key";
    let msg = b"The quick brown fox jumps over the lazy dog";
    let result = requests_core::hmac_sha256(key, msg);
    let hex: String = result.iter().map(|b| format!("{:02x}", b)).collect();
    assert_eq!(
        hex,
        "f7bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8"
    );
}

#[test]
fn test_utils_rc4() {
    let key = b"Key";
    let plaintext = b"Plaintext";
    let encrypted = requests_core::rc4(key, plaintext);
    let decrypted = requests_core::rc4(key, &encrypted);
    assert_eq!(decrypted, plaintext);
}