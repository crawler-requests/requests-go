# Native 二进制交付模式规划（2026-10-17）

## 背景与现状

- 100 个 `requests-<lang>` wrapper 全部以 **subprocess 调用 `requests-cli-linux-amd64`** 工作（唯一例外 requests-python 用 ctypes FFI 直连 `requests-linux-amd64.so`）。
- Rust 引擎通过 `libloading` **动态加载** libcurl-impersonate.so 实现浏览器指纹；CLI 二进制是 wrapper 的唯一依赖物。
- 构建矩阵（`libs_dist/build-all.sh`）：linux amd64/arm64、windows amd64、darwin amd64/arm64 共 **5 平台**。
- libcurl-impersonate 官方 v1.5.6 提供 5 平台发布包（tar.gz）。
- **现状：0 个 GitHub release**；二进制不进 git，靠本地 `cp` 分发 + wrapper 的 `_find_requests_cli` 沿祖先/兄弟仓库回溯定位共享引擎 `libs_dist/`。
- 大多数 wrapper 无包管理文件（仅 python 有 setup.py、go 有 go.mod、java 有 pom.xml），其余是裸脚本 + 一个 `lib/requests.sh` 之类的薄壳。

## 决策：单一来源 = GitHub Release，下载器进语言生态

**选型：主交付走「GitHub Release 托管预编译二进制 + 各语言 wrapper 内置下载器/安装钩子」；自编译仅在无预编译平台或需要定制时兜底。**

### 理由

1. **100 个 wrapper 的形态差异巨大**。只有 python/go/java 有包管理器能挂 install hook；其余 97 个是脚本壳。让"每种语言自己编译依赖"对裸脚本语言（sh、awk、powershell…）是灾难——它们连 Rust 工具链都没有。
2. **单一 Release 源保证一致性**。所有 wrapper 拉同一份二进制，避免"每个仓库各自编译 → 各编出各的"导致的版本漂移（这正是 9-11 旧二进制 + 10-17 新二进制断链的历史教训）。
3. **CLI 单一交付物**（约 8.4MB，含 rustls + 动态加载的 libcurl 路径）已经把复杂度封在引擎内，wrapper 不需要懂 TLS/指纹实现。
4. libcurl-impersonate 是**动态加载**的依赖：要么随 Release 捆绑（linux 上传 .so + 符号链接），要么要求系统/用户自装。**捆绑到 Release 是开箱即用的关键**，否则每平台都要额外找 libcurl，与"wrapper 零配置"矛盾。

## 具体模式

### Release 资产命名（统一约定）

引擎仓库 `crawler-requests/requests-go` 每次发版产出（命名 `{os}-{arch}`）：

| 资产 | 说明 |
|---|---|
| `requests-cli-linux-amd64` / `-linux-arm64` | CLI 可执行（subprocess 模式用） |
| `requests-linux-amd64.so` / `-linux-arm64.so` | FFI 共享库（python 等 FFI 模式用） |
| `requests-windows-amd64.dll` | FFI 共享库 |
| `requests-darwin-amd64.dylib` / `-darwin-arm64.dylib` | FFI 共享库 |
| `libcurl-impersonate.so` + `.so.4` + `.so.4.8.0` | linux libcurl（bundle，三份符号链接） |
| `libcurl-impersonate.dylib` / `.dll` | macos / windows 对应物 |
| `SHA256SUMS` | 校验和 |

> windows/macos 的 libcurl 目前本机无构建，**第 1 版先只发布 linux amd64/arm64 + 对应的 libcurl linux 版**；其余平台标记为"计划中"，wrapper 下载器遇 404 回退到自编译。

### 各语言 wrapper 的三层获取策略

每语言 wrapper 内置统一逻辑（模板由 `scripts/` 生成）：

1. **REQUESTS_CLI（或对应环境的变量）优先**：用户显式指定路径 → 直接用。
2. **本地缓存**：`~/.cache/requests/{version}/{os}-{arch}/requests-cli` —— 已下载则复用，避免重复下载。
3. **Release 下载**：`https://github.com/crawler-requests/requests-go/releases/download/v{ver}/requests-cli-{os}-{arch}`，下载后校验 SHA256，`chmod +x`，写入缓存。
4. **自编译兜底**：仅当下载失败（离线/平台无预编译/需定制）才调用 `build-cli.sh`/`build-all.sh`（要求本机有 Rust）。

### 语言生态的具体接入

| 语言 | 机制 |
|---|---|
| python | `setup.py`/`pyproject.toml` 加 `setup_requires` 钩子，或 `requests_core/__init__.py` 首次 import 时懒下载到 `~/.cache`（**推荐懒下载**，跨平台纯 python 逻辑） |
| node | `index.js` 首次 require 时懒下载（用内置 `https`，零依赖） |
| go | `go generate` 或首调时下载到缓存目录 |
| java | Maven `profile` 或 `requests` 类静态块懒下载 |
| 其余 97 脚本语言 | 统一由 `lib/requests.sh`（或等价）里的 `_ensure_cli` 实现：先变量/缓存/Release/自编译 |

**懒下载优于构建期钩子**：不破坏各语言现有安装流程，失败可降级，日志可控。构建期下载（python setup.py）在隔离/离线 CI 会硬失败。

### 版本对齐

- wrapper 记录**目标引擎版本**（如 `REQUESTS_ENGINE_VERSION="0.1.0"`），固定拉对应 Release tag `v0.1.0`——不追 latest，保证可复现。
- 引擎发版流程：`build-all.sh` 全平台产物 → 打包进 GitHub Release（tag `v{semver}`）+ 生成 SHA256SUMS。
- wrapper 升级 = 改一行版本常量 + 更新 README。

## 实施顺序（建议）

1. **引擎侧**：CI/发布脚本 `release.sh`——调 build-all.sh，汇总 5 平台产物 + libcurl bundle + SHA256SUMS，自动 `gh release create`。
2. **下载器模板**：在 `scripts/` 写通用下载逻辑（bash 版为权威实现），并用 `create_all_wrappers.py` 注入到 100 个 wrapper 的 `lib/requests.sh` / 等价文件。
3. **先落地 3 个重点语言**（python/node/go），验证懒下载 + 缓存 + 版本固定 + 失败降级。
4. **全量 100 仓库**推广模板，更新 README 的安装段。
5. **自编译路径**保留在 `build-*.sh` 作为兜底，并文档化"为何默认 Release"。

## 待验证风险

- windows/macos 的 libcurl 预编译资产缺失 → 首版只保证 linux；wrapper 在非 linux 平台收到 404 时应明确报错而非静默（除非走自编译）。
- GitHub API 下载限流（未认证 60 req/h）——懒下载是低频操作，缓存后不重复请求，风险低；但批量测试 100 仓库同时首启会撞限流，测试脚本需串行/复用缓存。
- 二进制进 git 的现有 libs_dist 是否还保留：**建议保留**（零配置开发体验，本地已就位），但文档标注"开发用，生产用 Release"。

## 决策摘要

- **默认交付 = GitHub Release 下载预编译二进制（每语言懒下载 + 缓存 + 版本固定）**
- **自编译 = 兜底路径**（离线 / 无预编译平台 / 需要定制 libcurl）
- 单一引擎版本常量控制所有 wrapper，杜绝版本漂移
- libcurl 随 Release 捆绑，保证 wrapper 开箱即用
