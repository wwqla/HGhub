<div align="center">

# HGhub

**黄果短剧第三方客户端 · Rust + Tauri 2**

Windows / macOS / iOS / Android · Fluent 设计 · 沉浸式竖屏播放
![alt text](docs/image.png)

</div>

> ⚠️ 本项目仅供 **学习与技术交流**，不提供、不存储、不分发任何音视频内容。
> 所有内容均来自第三方站点，请自行判断合规性，并在 24 小时内删除相关资源。


## macOS 版

> 本仓库在 [Minireel/HGhub](https://github.com/Minireel/HGhub) 基础上补了 macOS 与 iOS 适配，原项目的 MIT 版权与免责声明保持不变。

### 下载

- **最新 Release**：打开仓库右侧 Releases，下载 `HGhub_x.y.z_macOS_universal.dmg`（Apple Silicon 与 Intel 通用，需 macOS 11+）。
- **最新构建**：Actions → **macOS** → 点最近一次成功的运行 → 页面底部 Artifacts 下载 `HGhub-macOS`（解压后得到 dmg）。

### 安装与首次打开（未签名应用）

1. 打开 dmg，把 **HGhub** 拖进「应用程序」。
2. 首次双击会提示「无法验证开发者」。macOS 15 及以后**右键 → 打开已不再生效**，请这样放行：
   打开 **系统设置 → 隐私与安全性**，滚到底部「安全性」，找到 HGhub 那一行点 **仍要打开**，输入密码确认即可。之后正常双击打开。
3. 或者在「终端」执行一次（效果相同）：

   ```bash
   xattr -cr /Applications/HGhub.app
   ```

### 与 Windows 版的差异

- 使用系统原生红绿灯窗口按钮和窗口缩放，其余界面与 Windows 版一致，没有新增设置项。
- 封面解密与视频代理在 macOS 上走本机回环服务（`http://127.0.0.1:<随机端口>/<随机令牌>/*`，仅本机可访问），因为 macOS 26.6 起 WKWebView 会拦截跨自定义协议的加载（见 tauri-apps/wry#1778）；视频由 hls.js 经 MSE 播放。

## iOS 版

### 下载

- **最新 Release**：下载 `HGhub_x.y.z_iOS_unsigned.ipa`（iPhone / iPad，需 iOS 15+）。
- **最新构建**：Actions → **iOS** → 点最近一次成功的运行 → 页面底部 Artifacts 下载 `HGhub-iOS`（解压后得到 ipa）。

### 安装（未签名 IPA，免费）

iOS 不能直接装未签名应用，需要用自己的 Apple ID 签一次名。下面两个工具都免费：

- **Sideloadly**（Windows / macOS）：iPhone 用数据线连电脑 → 打开 Sideloadly，把 ipa 拖进去 → 填你的 Apple ID → Start。
- **AltStore**（Windows / macOS）：电脑装 AltServer，手机装 AltStore，再在 AltStore 里选这个 ipa 安装。

装好后：

1. 打开 **设置 → 通用 → VPN 与设备管理**，信任你的 Apple ID。
2. iOS 16 及以后还要打开 **设置 → 隐私与安全性 → 开发者模式**，按提示重启。

免费 Apple ID 签的应用 **7 天后过期**，到期用同一工具重签即可（数据不丢）；同时最多装 3 个这样的应用。有付费开发者账号的话签名后可长期使用。

### 与 Android 版的差异

- 界面与 Android 版一致（移动端布局、刘海 / 底部横条安全区），没有新增设置项；iOS 没有返回键，用页面上的返回按钮。
- 封面解密与视频代理同 macOS，走本机回环服务；视频由 hls.js 播放（iOS 17.1+ 走 ManagedMediaSource），更老的系统自动改用系统原生 HLS。

## 技术栈

| 层 | 选型 |
| --- | --- |
| 壳 | Tauri 2（Rust 1.9x） |
| 前端 | React 19 + TypeScript + Vite 7 + Tailwind CSS 4 |
| 播放 | hls.js（HLS / AES-128） |
| 网络 | reqwest + rustls |
| 图标 | 内联 SVG |

## 目录

```
.
├─ src/                      前端
│  ├─ api.ts                 接口封装 + 本地媒体协议 URL
│  ├─ store.tsx              全局状态（导航栈 / 用户 / 本地收藏 / 主题）
│  ├─ html.ts                吃瓜正文清洗与图片代理
│  ├─ components/            Feed（播放页）/ VideoSurface / PlayerControls / Panels / UI
│  └─ pages/                 Home / Search / Category / Profile / Posts
├─ src-tauri/                Rust 后端
│  └─ src/
│     ├─ net.rs              请求层（边缘信封 + 重试）
│     ├─ api.rs              业务命令
│     ├─ media.rs            封面解密 + HLS 代理 + 缓存
│     └─ session.rs          token 持久化
└─ docs/API.md               接口文档
```

## 开发 / 构建

```bash
pnpm install          # 首次安装
pnpm app:dev          # 桌面端热重载
pnpm app:build        # 打包 Windows 安装程序（NSIS）；在 Mac 上运行则产出 .app + .dmg
```

macOS 通用包（需 `rustup target add aarch64-apple-darwin x86_64-apple-darwin`）：

```bash
pnpm tauri build --target universal-apple-darwin --bundles app,dmg
```

CI：`.github/workflows/macos.yml` 在推送 main / 手动运行时产出 dmg（Artifacts），推 `v*` tag 时同时发布到 Release。

Android（需 Android SDK + NDK，并配置 `ANDROID_HOME` / `NDK_HOME`）：

```bash
pnpm android:init     # 首次生成 android 工程
pnpm android:dev      # 真机 / 模拟器调试
pnpm android:build    # 产出 APK
```

### 环境注意

- 工程若放在 **exFAT** 盘（不支持符号链接），需保留 `.npmrc` 中的 `node-linker=hoisted`。
- `esbuild` 需在 `package.json` 的 `pnpm.onlyBuiltDependencies` 中放行，否则 `vite build` 缺少二进制。

## 实现要点

1. **统一边缘信封**：站点把 `/api/**` 收敛为 `POST /api/edge/request`（`method/path/body(base64)/content_type` + `Authorization: Bearer`），本项目只对接这一个入口，无需处理 Cookie。
2. **封面解密**：封面为 AES-128-CBC 密文，Rust 侧解密后经自定义协议输出，前端 `<img>` 直接使用。
3. **HLS 代理**：m3u8 与分片经自定义协议转发并改写 `#EXT-X-KEY` / 分片地址，绕开 WebView 的 CORS 与防盗链。
4. **自定义协议**：Windows / Android 为 `http://hg.localhost/*`，macOS / iOS 为本机回环 `http://127.0.0.1:<端口>/<令牌>/*`（`src-tauri/src/loopback.rs`，启动时注入 `window.__HG_LOCAL__`；老系统回退 `hg://localhost/*`）（Rust 侧 `LOCAL_ORIGIN` 与前端 `LOCAL` 按平台生成，必须一致，避免 WebView 无法解析）。直链媒体支持 HTTP Range（206），满足 macOS AVFoundation 的分段读取。
5. **不做签名逆向**：只复用浏览器同源行为，接口字段均有文档与实测样本。

接口细节见 [docs/API.md](docs/API.md)。

## 免责声明

- 本项目与内容提供方无任何关联，作者不参与内容的制作、上传与分发。
- 使用者应自行遵守所在地法律法规；因使用本项目产生的任何后果由使用者自行承担。
- 若相关权利方认为本项目存在侵权，请通过 Issue 联系，我们会及时处理。

## License

[MIT](LICENSE)
## 常见问题

**1. `pnpm install` 报 `ERR_PNPM_EISDIR ... symlink`**

工程放在 exFAT / FAT32 盘时无法创建符号链接，保留 `.npmrc` 里的 `node-linker=hoisted` 即可。

**2. `tauri android build` 报 `Failed to create a symbolic link ... jniLibs`**

同样是 exFAT 不支持符号链接。可手动把 Rust 产物拷进 jniLibs，再直接跑 Gradle：

```bash
cp src-tauri/target/aarch64-linux-android/debug/libhuangguo_client_lib.so \
   src-tauri/gen/android/app/src/main/jniLibs/arm64-v8a/

cd src-tauri/gen/android
./gradlew assembleDebug        # Windows: gradlew.bat assembleDebug
```

产物在 `src-tauri/gen/android/app/build/outputs/apk/`。

**3. Android 构建提示找不到 SDK / NDK**

需要设置 `ANDROID_HOME`（Android SDK 根目录）、`NDK_HOME`（`$ANDROID_HOME/ndk/<版本>`）与 `JAVA_HOME`（JDK 17）。

## LinuxDO
https://linux.do/