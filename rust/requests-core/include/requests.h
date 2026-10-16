#ifndef REQUESTS_FFI_H
#define REQUESTS_FFI_H

#include <stddef.h>
#include <stdint.h>

#ifdef _WIN32
    #define REQUESTS_API __declspec(dllexport)
#else
    #define REQUESTS_API __attribute__((visibility("default")))
#endif

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================================
// FFI 响应结构体
// ============================================================================

typedef struct {
    char* json;     // 成功时返回 JSON 序列化的响应
    char* error;    // 失败时返回错误信息
} FFIResponse;

/// 字节数组结构体（用于 RC4 等返回二进制数据的函数）
typedef struct {
    uint8_t* data;  // 指向字节数组的指针
    size_t len;     // 数组长度
} FfiBytes;

// ============================================================================
// 版本与内存管理
// ============================================================================

/// 获取版本字符串
REQUESTS_API const char* requests_version(void);

/// 释放 FFIResponse 结构体及其内部指针
REQUESTS_API void requests_free_response(FFIResponse* resp);

/// 释放字符串（用于工具函数返回的 char*）
REQUESTS_API void requests_free_string(char* s);

/// 释放字节数组（用于工具函数返回的 uint8_t*，需要传入长度）
REQUESTS_API void requests_free_bytes(uint8_t* p, size_t len);

// ============================================================================
// 核心请求接口
// ============================================================================

/// 核心 JSON 契约接口（推荐用于复杂请求）
///
/// config_json 是一个 JSON 对象。浏览器指纹走 libcurl-impersonate：
///   impersonate: "chrome136" | "firefox135" | "safari18_0" | ...
///   akamai:      "1:65536;2:0;4:6291456;6:262144|15663105|0|m,a,s,p"
///   ja3:         可选，覆盖 impersonate 目标的密码套件列表
///   browser_profile: 旧接口，仅在没有 libcurl-impersonate 时回退到外部进程
///
/// 库搜索顺序：REQUESTS_LIBCURL、CURL_IMPERSONATE_LIB、libcurl-impersonate.so。
/// 返回的 FFIResponse 中：
///   - 成功时：json 字段包含 JSON 序列化的 FfiResponseOutput
///   - 失败时：error 字段包含错误信息字符串
REQUESTS_API FFIResponse* requests_request(const char* config_json);

/// 简化 GET 请求接口
REQUESTS_API FFIResponse* requests_send_get(const char* url);

/// 简化 POST 请求接口
/// body 可以为 NULL（表示无请求体）
REQUESTS_API FFIResponse* requests_send_post(const char* url, const char* body);

// ============================================================================
// 工具函数接口
// ============================================================================

/// URI 组件编码
/// 返回的字符串需要调用 requests_free_string 释放
REQUESTS_API char* requests_encode_uri_component(const char* input);

/// 计算 MD5 哈希，返回十六进制字符串
/// 返回的字符串需要调用 requests_free_string 释放
REQUESTS_API char* requests_md5(const uint8_t* data, size_t len);

/// 计算 SHA-256 哈希，返回十六进制字符串
/// 返回的字符串需要调用 requests_free_string 释放
REQUESTS_API char* requests_sha256(const uint8_t* data, size_t len);

/// 计算 HMAC-SHA256，返回十六进制字符串
/// 返回的字符串需要调用 requests_free_string 释放
REQUESTS_API char* requests_hmac_sha256(const uint8_t* key, size_t key_len,
                                        const uint8_t* msg, size_t msg_len);

/// RC4 加密/解密
/// 返回 FfiBytes 结构体，包含 data 指针和 len 长度
/// 释放时调用 requests_free_bytes(data, len)
REQUESTS_API FfiBytes requests_rc4(const uint8_t* key, size_t key_len,
                                    const uint8_t* input, size_t input_len);

#ifdef __cplusplus
}
#endif

#endif /* REQUESTS_FFI_H */
