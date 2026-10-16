#!/usr/bin/env python3
"""Convert README.md Go examples to Rust examples."""

import re
import sys

def convert_readme(content: str) -> str:
    text = content

    # 1. Replace top badges and install lines
    text = text.replace(
        '[![Go Version](https://img.shields.io/badge/Go%20Version-1.24.0-blue?logo=go&logoColor=white&labelColor=gray)]()',
        '[![Rust Version](https://img.shields.io/badge/Rust-1.24%2B-blue?logo=rust&logoColor=white&labelColor=gray)]()'
    )
    text = text.replace(
        '[![go documentation](https://img.shields.io/badge/go-documentation-blue)](https://pkg.go.dev/github.com/wangluozhe/requests)',
        '[![Rust documentation](https://img.shields.io/badge/rust-documentation-blue)](https://docs.rs/requests-core/latest/requests_core/)'
    )
    text = text.replace(
        '```bash\ncargo add requests-core\n```',
        '```bash\ncargo add requests-core\n```'
    )

    # 2. Remove standalone go get / go install lines
    text = re.sub(r'```bash\ngo get github.com/wangluozhe/requests\n```\n', '', text)
    text = re.sub(r'```bash\ngo get github.com/wangluozhe/requests@v1\.5\.2\n```\n', '', text)

    # 3. Replace code-block language tags
    text = re.sub(r'```golang', '```rust', text)
    text = re.sub(r'```go(\s*\n)', r'```rust\1', text)

    # 4. Replace import blocks
    text = re.sub(
        r'import \(\n\s*"github\.com/wangluozhe/requests"\n\s*"github\.com/wangluozhe/requests/url"\n\)',
        'use requests_core::{Request, RequestParams, Session, Response};',
        text,
    )
    text = re.sub(
        r'import \(\n\s*"github\.com/wangluozhe/requests"\n\s*"github\.com/wangluozhe/requests/transport"\n\s*"github\.com/wangluozhe/requests/url"\n\)',
        'use requests_core::{Request, RequestParams, Session, TLSExtensions, H2Settings, transport};',
        text,
    )
    text = re.sub(
        r'import \(\n\s*"fmt"\n\s*"github\.com/wangluozhe/requests"\n\s*"github\.com/wangluozhe/requests/url"\n\)',
        'use requests_core::{Request, RequestParams, Session, Response};',
        text,
    )
    text = re.sub(
        r'import \(\n\s*"fmt"\n\s*http "github\.com/wangluozhe/chttp"\n\s*"github\.com/wangluozhe/requests"\n\s*"github\.com/wangluozhe/requests/transport"\n\s*"github\.com/wangluozhe/requests/url"\n\)',
        'use requests_core::{Request, RequestParams, Session, Response, TLSExtensions, H2Settings};',
        text,
    )

    # 5. Replace top-level package main / func main()
    text = re.sub(r'package main\n\n', '', text)
    text = re.sub(r'package main\n', '', text)
    text = re.sub(r'func main\(\) \{\n', 'fn main() -> anyhow::Result<()> {\n', text)
    text = text.replace('panic(err)', 'panic!("{err}")')

    # 6. Replace common API calls
    text = re.sub(r'requests\.SetDebug\(true\)', '// debug enabled via Session Debug fmt', text)
    text = re.sub(r'requests\.SetDebug\(false\)', '// debug disabled', text)
    text = text.replace('requests.Get(', 'Session::new().get(')
    text = text.replace('requests.Post(', 'Session::new().post(')
    text = text.replace('requests.Put(', 'Session::new().put(')
    text = text.replace('requests.Delete(', 'Session::new().delete(')
    text = text.replace('requests.Head(', 'Session::new().head(')
    text = text.replace('requests.Options(', 'Session::new().options(')

    # 7. Replace url helpers
    text = text.replace('url.NewRequest()', 'Request::default()')
    text = text.replace('url.NewParams()', 'RequestParams::new()')
    text = text.replace('url.NewHeaders()', 'reqwest::header::HeaderMap::new()')
    text = text.replace('url.NewData()', 'RequestParams::new()')
    text = text.replace('url.NewFiles()', 'vec![]')
    text = text.replace('url.NewCookies()', '// cookies are shared via Session cookie jar')

    # 8. Replace setters
    text = re.sub(r'\.Set\("([^"]+)",\s*"([^"]+)"\)', r'.add("\1", "\2")', text)
    text = re.sub(r'\.Add\("([^"]+)",\s*"([^"]+)"\)', r'.add("\1", "\2")', text)
    text = text.replace('.Params = url.ParseParams(', '.params = Some(RequestParams::from(')
    text = text.replace('.Data = url.ParseData(', '.data = Some(RequestParams::from(')
    text = text.replace('.Headers = url.ParseHeaders(', '.headers = {')

    # 9. Replace struct field assignments
    text = text.replace('req.Proxies = "', 'session = session.with_proxy("')
    text = text.replace('req.Timeout = 1 * time.Millisecond', 'session = session.with_timeout(std::time::Duration::from_millis(1))')
    text = text.replace('req.Auth = []string{"user","password"}', 'req.auth = Some(("user".into(), "password".into()))')
    text = text.replace('req.Cert = []string{"cert","key"}', 'req.cert = vec!["cert".into(),"key".into()]')
    text = text.replace('req.Body = "testdata"', 'req.body = Some(b"testdata".to_vec())')
    text = text.replace('req.Body = "raw body content"', 'req.body = Some(b"raw body content".to_vec())')
    text = text.replace('req.Stream = true', 'req.stream = true')
    text = text.replace('req.Verify = url.Bool(false)', 'session = session.with_verify(false); req.verify = false')
    text = text.replace('req.AllowRedirects = url.Bool(false)', 'req.allow_redirects = false')
    text = text.replace('req.ForceHTTP1 = url.Bool(true)', 'session = session.with_force_http1(true)')
    text = text.replace('req.Ja3 = "', 'session = session.with_ja3("')
    text = text.replace('req.RandomJA3 = true', 'session = session.with_random_ja3(true)')
    text = text.replace('req.HTTP2Settings = h2ss', 'req.http2_settings = Some(h2ss)')
    text = text.replace('req.TLSExtensions = tes', 'req.tls_extensions = Some(tes)')

    # 10. Replace response fields
    text = text.replace('r.Text()', 'resp.text()')
    text = text.replace('r.Content', 'resp.content')
    text = text.replace('r.StatusCode', 'resp.status_code.as_u16()')
    text = text.replace('r.Headers', 'resp.headers')
    text = text.replace('r.Cookies', 'resp.cookies')
    text = text.replace('r.Url', 'resp.url')
    text = text.replace('r.History', 'resp.history')
    text = text.replace('r.Body', 'resp.content')
    text = text.replace('r.Json(&', 'resp.json(&')
    text = text.replace('r.JsonMap()', 'serde_json::from_slice(&resp.content)?')
    text = text.replace('r.SimpleJson()', 'serde_json::from_slice(&resp.content)?')

    # 11. Replace transport types
    text = text.replace('&transport.Extensions{', '&TLSExtensions {')
    text = text.replace('&transport.H2Settings{', '&H2Settings {')
    text = text.replace('transport.ToTLSExtensions(es)', 'es.clone()')
    text = text.replace('transport.ToHTTP2Settings(h2s)', 'h2s.clone()')

    # 12. Replace url package prefixes in examples
    text = text.replace('url.NewParams()', 'RequestParams::new()')
    text = text.replace('url.NewData()', 'RequestParams::new()')
    text = text.replace('url.NewRequest()', 'Request::default()')
    text = text.replace('url.NewFiles()', 'vec![]')
    text = text.replace('url.NewHeaders()', 'reqwest::header::HeaderMap::new()')
    text = text.replace('url.ParseParams(', 'RequestParams::from(')
    text = text.replace('url.ParseData(', 'RequestParams::from(')
    text = text.replace('url.ParseHeaders(', '// parse headers into HeaderMap: ')
    text = text.replace('url.ParseCookies(', '// parse cookies into Cookie header: ')
    text = text.replace('url.Bool(false)', 'false')
    text = text.replace('url.Bool(true)', 'true')

    # 13. Replace chttp header ordering mentions
    text = text.replace('http.PHeaderOrderKey', '// pseudo-header ordering')
    text = text.replace('http.HeaderOrderKey', '// header ordering')
    text = text.replace('http.UnChangedHeaderKey', '// unchanged header ordering')

    # 14. Replace Go-ish requests API references
    text = text.replace('github.com/wangluozhe/requests/url', 'requests_core')
    text = text.replace('github.com/wangluozhe/requests', 'requests_core')
    text = text.replace('github.com/wangluozhe/chttp', 'reqwest::header')

    # 15. Replace utils package references
    text = text.replace('github.com/wangluozhe/requests/utils', 'requests_core::utils')
    text = text.replace('utils.HexEncode(', 'utils::hex_encode(')
    text = text.replace('utils.HexDecode(', 'utils::hex_decode(')
    text = text.replace('utils.EncodeURIComponent(', 'utils::encode_uri_component(')
    text = text.replace('utils.EncodeURI(', 'utils::encode_uri(')
    text = text.replace('utils.DecodeURIComponent(', 'utils::decode_uri_component(')
    text = text.replace('utils.DecodeURI(', 'utils::decode_uri(')
    text = text.replace('utils.Escape(', 'utils::escape(')
    text = text.replace('utils.UnEscape(', 'utils::unescape(')
    text = text.replace('utils.Base32Encode(', 'utils::base32_encode(')
    text = text.replace('utils.Base32Decode(', 'utils::base32_decode(')
    text = text.replace('utils.Base64Encode(', 'utils::base64_encode(')
    text = text.replace('utils.Base64Decode(', 'utils::base64_decode(')
    text = text.replace('utils.Btoa(', 'utils::btoa(')
    text = text.replace('utils.Atob(', 'utils::atob(')
    text = text.replace('utils.MD4(', 'utils::md4(')
    text = text.replace('utils.RIPEMD160(', 'utils::ripemd160(')
    text = text.replace('utils.MD5(', 'utils::md5(')
    text = text.replace('utils.SHA1(', 'utils::sha1(')
    text = text.replace('utils.SHA224(', 'utils::sha224(')
    text = text.replace('utils.SHA256(', 'utils::sha256(')
    text = text.replace('utils.SHA384(', 'utils::sha384(')
    text = text.replace('utils.SHA512(', 'utils::sha512(')
    text = text.replace('utils.HmacMD4(', 'utils::hmac_md4(')
    text = text.replace('utils.HmacRIPEMD160(', 'utils::hmac_ripemd160(')
    text = text.replace('utils.HmacMD5(', 'utils::hmac_md5(')
    text = text.replace('utils.HmacSHA1(', 'utils::hmac_sha1(')
    text = text.replace('utils.HmacSHA224(', 'utils::hmac_sha224(')
    text = text.replace('utils.HmacSHA256(', 'utils::hmac_sha256(')
    text = text.replace('utils.HmacSHA384(', 'utils::hmac_sha384(')
    text = text.replace('utils.HmacSHA512(', 'utils::hmac_sha512(')

    # 16. Replace fmt.Println / fmt.Printf patterns
    text = text.replace('fmt.Println(err)', 'eprintln!("{err}")')
    text = text.replace('fmt.Println(', 'println!(')
    text = text.replace('fmt.Printf(', 'println!(')

    # 17. Replace os / io patterns
    text = text.replace('io.Copy(out, r.Body)', 'std::io::copy(out, &resp.content)?')
    text = text.replace('os.Create("large_file.dat")', 'std::fs::File::create("large_file.dat")?')

    # 18. Replace time import/usage
    text = text.replace('"time"', '')
    text = text.replace('time.Now()', 'std::time::Instant::now()')
    text = text.replace('time.Since(', 'std::time::Instant::elapsed(&')

    # 19. Replace cookiejar usage
    text = text.replace('cookiejar.New(nil)', '// use Session cookie jar')
    text = text.replace('cookies.SetCookies(urls,[]*http.Cookie{', '// cookies.SetCookies(')
    text = text.replace('&http.Cookie{', '// Cookie{')

    # 20. Replace struct definitions used in examples
    text = text.replace('type GithubEvent struct {', '#[derive(serde::Deserialize)]\nstruct GithubEvent {')
    text = text.replace('`json:"type"`', '#[serde(rename = "type")]')
    text = text.replace('`json:"repo"`', '#[serde(rename = "repo")]')

    # 21. Replace middleware example
    text = text.replace('func LoggingMiddleware(next requests.Handler) requests.Handler {', 'fn logging_middleware(next: requests_core::MiddlewareFn) -> requests_core::MiddlewareFn {')
    text = text.replace('return func(preq *models.PrepareRequest, req *url.Request) (*models.Response, error) {', 'return Arc::new(move |ctx| {')
    text = text.replace('// 执行下一个 Handler\n\t\t\tresp, err := next(preq, req)', 'let resp = next(ctx).await')
    text = text.replace('// 执行下一个 Handler\n\t\t\t\tresp, err := next(preq, req)', 'let resp = next(ctx).await')
    text = text.replace('return resp, err', 'resp')
    text = text.replace('func AuthMiddleware(token string) requests.Middleware {', 'fn auth_middleware(token: impl Into<String>) -> impl Fn(requests_core::MiddlewareFn) -> requests_core::MiddlewareFn {')
    text = text.replace('return func(next requests.Handler) requests.Handler {', 'return move |next| {')
    text = text.replace('return func(preq *models.PrepareRequest, req *url.Request) (*models.Response, error) {', 'return Arc::new(move |ctx| {')

    # 22. Replace session middleware usage
    text = text.replace('s.Use(LoggingMiddleware)', 'session.use_middleware(logging_middleware)')
    text = text.replace('s.Use(AuthMiddleware("my-secret-token"))', 'session.use_middleware(auth_middleware("my-secret-token"))')
    text = text.replace('s := requests.NewSession()', 'let mut session = Session::new()')

    # 23. Replace models.Response / models.Request in examples
    text = text.replace('*models.PrepareRequest', 'MiddlewareContext')
    text = text.replace('*url.Request', 'Request')
    text = text.replace('*models.Response', 'Response')

    # 24. Replace Request.Request.Headers assignment patterns
    text = text.replace('req.Headers = headers', 'req.headers = headers')

    # 25. Replace header order examples with comments
    text = text.replace(
        '(*headers)["Header-Order:"] = []string{\n\t\t\t"user-agent",\n\t\t\t"path",\n\t\t\t"accept-language",\n\t\t\t"scheme",\n\t\t\t"connection",\n\t\t\t"accept-encoding",\n\t\t\t"content-length",\n\t\t\t"host",\n\t\t\t"accept",\n\t\t}',
        '// header ordering note: reqwest preserves insertion order'
    )
    text = text.replace(
        ':authority: spidertools.cn\n\t\t:method: GET\n\t\t:path: /\n\t\t:scheme: https // 不会被加入到头部中\n\t\t// accept: text/html // 不会被加入到头部中，因为被注释了\n\t\taccept: text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.9\n\t\taccept-encoding: gzip, deflate, br\n\t\taccept-language: zh-CN,zh;q=0.9\n\t\tcache-control: no-cache\n\t\tcookie: _ga=GA1.1.630251354.1645893020; Hm_lvt_def79de877408c7bd826e49b694147bc=1647245863,1647936048,1648296630; Hm_lpvt_def79de877408c7bd826e49b694147bc=1648296630\n\t\tpragma: no-cache\n\t\tsec-ch-ua: " Not A;Brand";v="99", "Chromium";v="98", "Google Chrome";v="98"\n\t\tsec-ch-ua-mobile: "?0"\n\t\tsec-ch-ua-platform: "Windows"\n\t\tsec-fetch-dest: document\n\t\tsec-fetch-mode: navigate\n\t\tsec-fetch-site: same-origin\n\t\tsec-fetch-user: "?1"\n\t\tupgrade-insecure-requests: 1\n\t\tuser-agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/98.0.4758.80 Safari/537.36\n\t\t',
        '// use reqwest::header::HeaderMap to build headers in desired order\n'
    )

    # 26. Replace tls fingerprint examples
    text = text.replace('es := &transport.Extensions{', 'let es = TLSExtensions {')
    text = text.replace('h2s := &transport.H2Settings{', 'let h2s = H2Settings {')
    text = text.replace('h2ss := transport.ToHTTP2Settings(h2s)', 'let h2ss = h2s;')
    text = text.replace('tes := transport.ToTLSExtensions(es)', 'let tes = es;')
    text = text.replace('req.TLSExtensions = tes', 'req.tls_extensions = Some(tes)')
    text = text.replace('req.HTTP2Settings = h2ss', 'req.http2_settings = Some(h2ss)')

    # 27. Replace `req.` assignment patterns
    text = re.sub(r'req\.(Ja3|RandomJA3|ForceHTTP1) = ', 'session = session.', text)

    # 28. Replace data.Set -> data.add
    text = text.replace('data.Set("key1","value1")', 'data.add("key1", "value1")')
    text = text.replace('data.Add("key1","value3")', 'data.add("key1", "value3")')

    # 29. Replace files.SetFile / files.SetField comments
    text = text.replace('files.SetFile("api","api","D:\\\\Go\\\\github.com\\\\wangluozhe\\\\requests\\\\api.go","")', '// files.SetFile example omitted; see FileUpload')
    text = text.replace('files.SetField("name","value")', '// files.SetField example omitted; see FileUpload')

    # 30. Replace HTTP method strings where obvious
    text = re.sub(r'requests\.(Get|Post|Put|Delete|Head|Options)\(', lambda m: {
        'Get': 'Session::new().get(',
        'Post': 'Session::new().post(',
        'Put': 'Session::new().put(',
        'Delete': 'Session::new().delete(',
        'Head': 'Session::new().head(',
        'Options': 'Session::new().options(',
    }[m.group(1)], text)

    return text


def main():
    readme_path = '/home/cc11001100/github/crawler-requests/requests/README.md'
    with open(readme_path, 'r', encoding='utf-8') as f:
        content = f.read()

    converted = convert_readme(content)

    with open(readme_path, 'w', encoding='utf-8') as f:
        f.write(converted)

    print('README.md converted to Rust examples.')


if __name__ == '__main__':
    main()
