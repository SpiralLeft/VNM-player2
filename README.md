# VNM Player

VNM Player 是一个基于 **Tauri 2、React 和 Rust** 的本地桌面音频播放器。除 MP3、WAV、OGG 和 FLAC 外，项目还实现了独立的 **NWA 解码器**，用于播放视觉小说游戏中使用的 NWA 音频文件。

当前项目版本为 `0.1.0`。本文以仓库现有实现为准，不代表所有格式、设备或操作系统均已完成兼容性验证。

## 功能

- **本地音频播放**：打开文件，播放、暂停、停止，以及通过进度条跳转。
- **播放列表**：添加单个文件或扫描文件夹，切换上一首 / 下一首，移除条目或清空列表；相同路径的条目不会重复添加。
- **循环模式**：不循环、单曲循环、列表循环。
- **曲目信息**：显示文件名、时长和播放进度；尝试读取音频文件中的内嵌封面，未找到时显示默认图标。
- **音量控制**：通过滑块调整 `0%` 至 `100%` 的输出音量。
- **原生音频输出**：由 Rust 后端解码并通过系统默认音频设备输出，采样率不一致时使用线性插值重采样。

### 支持的文件格式

| 扩展名 | 解码实现 | 说明 |
| --- | --- | --- |
| `.mp3` | Symphonia | MP3 音频 |
| `.wav` | Symphonia | WAV / PCM 音频 |
| `.ogg` | Symphonia | Ogg / Vorbis 音频；不表示支持 Ogg 中的所有编码 |
| `.flac` | Symphonia | FLAC 音频 |
| `.nwa` | 项目内置 `NwaDecoder` | 支持单声道 / 双声道、8 / 16 位，以及未压缩模式（`-1`）和压缩级别 `0` 至 `5` |

实际能否播放还取决于文件的编码参数、完整性和输出设备配置。

## 技术栈

| 层次 | 技术 |
| --- | --- |
| 桌面应用与前后端通信 | Tauri 2、Tauri Dialog 插件 |
| 用户界面 | React 19、TypeScript 5.7 |
| 状态管理 | Zustand 5 |
| 样式与前端构建 | Tailwind CSS 4、Vite 6 |
| 后端 | Rust，Edition 2021 |
| 音频解码 | Symphonia 0.6、自定义 NWA 解码器 |
| 音频输出与缓冲 | CPAL 0.15、ringbuf 0.4 |

依赖声明见 `package.json` 和 `src-tauri/Cargo.toml`；具体解析版本以 `package-lock.json` 和 `src-tauri/Cargo.lock` 为准。

## 开发环境

需要准备：

- **Node.js 与 npm**：Node.js 版本需满足依赖要求。当前 Vite 6 的版本范围为 `^18.0.0 || ^20.0.0 || >=22.0.0`。
- **Rust 工具链**：包含 `rustc` 和 `cargo`。
- **Tauri 2 所需的系统构建依赖**：Windows 环境需准备 Microsoft C++ Build Tools（C++ 桌面开发工具及 Windows SDK）和 WebView2 Runtime，并使用匹配的 Rust MSVC 工具链。其他系统需准备对应的原生构建依赖。
- **可用的默认音频输出设备**：播放器通过系统默认设备输出声音。

以下命令均在项目根目录执行。

### 安装前端依赖

```sh
npm ci
```

### 启动桌面开发模式

```sh
npm run tauri dev
```

Tauri 会先执行 `npm run dev` 启动 Vite，再构建 Rust 后端并打开桌面窗口。开发服务器固定使用 `1420` 端口，端口被占用时不会自动切换。

> `npm run dev` 只启动前端开发服务器。文件对话框、播放控制和音频输出依赖 Tauri 原生后端，不能将浏览器中的页面当作完整播放器使用。

### 构建桌面应用

```sh
npm run tauri build
```

该命令先执行前端构建，再构建 Rust 后端并生成当前平台的桌面产物：

- 前端静态产物位于 `dist/`。
- Rust 发布构建产物位于 `src-tauri/target/release/`。
- 安装包通常位于 `src-tauri/target/release/bundle/` 下，具体格式取决于平台。

应用名称、窗口配置、版本、图标和打包配置位于 `src-tauri/tauri.conf.json`。

### 常用命令

| 命令 | 用途 |
| --- | --- |
| `npm run dev` | 仅启动 Vite 前端开发服务器 |
| `npm run build` | 执行 TypeScript 构建检查并生成前端静态产物 |
| `npm run preview` | 预览已构建的前端页面，不提供原生播放能力 |
| `npm run tauri dev` | 启动完整桌面开发环境 |
| `npm run tauri build` | 构建并打包桌面应用 |
| `cargo check --manifest-path src-tauri/Cargo.toml` | 检查 Rust 后端编译 |

## 使用方式

1. 点击播放列表顶部的 **Open**，选择一个受支持的音频文件。文件会加入列表并加载，随后点击底部 **Play** 开始播放。
2. 点击 **+Folder**，将所选文件夹中的音频文件加入播放列表。扫描仅处理当前文件夹，不递归进入子文件夹，结果按路径排序。
3. 点击列表条目切换曲目。正在播放时切换会继续播放；暂停时切换只加载曲目，需要手动点击播放。
4. 使用底部按钮停止、切换上一首 / 下一首或播放 / 暂停。停止会将进度归零，但保留已加载曲目。
5. 拖动进度条跳转播放位置，拖动音量滑块调节音量。
6. 点击列表条目右侧的 **×** 移除条目，或点击 **Clear** 清空列表。

播放列表顶部的循环按钮依次切换以下模式：

| 图标 | 模式 | 行为 |
| --- | --- | --- |
| `→` | 不循环 | 当前曲目结束后停止自动续播 |
| `↻1` | 单曲循环 | 由 Rust 解码线程将当前曲目跳回开头继续解码 |
| `↻` | 列表循环 | 曲目结束后自动切换下一首，到列表末尾后回到第一首 |

## 项目结构

```text
.
├── src/
│   ├── App.tsx                  # 主界面布局与播放事件监听初始化
│   ├── components/             # 曲目信息、播放列表、播放控制、进度与音量组件
│   ├── stores/playerStore.ts   # Zustand 状态、Tauri 命令调用和事件处理
│   ├── types/index.ts          # 播放状态、文件信息与事件类型
│   └── main.tsx                # React 入口
├── src-tauri/
│   ├── src/
│   │   ├── lib.rs              # Tauri 初始化、共享状态与命令注册
│   │   ├── commands.rs         # 播放与列表命令、内嵌封面提取和缓存
│   │   ├── player.rs           # 播放状态、解码线程与进度事件
│   │   ├── playlist.rs         # 列表管理、循环模式和文件夹扫描
│   │   ├── decoder.rs          # 统一解码接口与错误类型
│   │   ├── symphonia_decoder.rs # 常见音频格式解码
│   │   ├── nwa_decoder.rs      # NWA 文件解析与解码
│   │   ├── resampler.rs        # 线性插值重采样
│   │   └── audio_engine.rs     # CPAL 输出、环形缓冲消费和音量处理
│   ├── capabilities/           # Tauri 权限配置
│   ├── icons/                  # 应用图标
│   ├── Cargo.toml              # Rust 依赖
│   └── tauri.conf.json         # 桌面窗口与构建配置
├── package.json               # 前端依赖与 npm 脚本
└── vite.config.ts             # Vite、React 与 Tailwind 配置
```

### 播放链路

```text
React 界面 → Zustand store → Tauri invoke → Rust Player
                                              │
                          NWA / Symphonia 解码器
                                              ↓
                                f32 PCM → 线性重采样
                                              ↓
                                  ringbuf → CPAL → 默认音频设备
```

Rust 后端通过 `playback-progress`、`playback-state-changed` 和 `playback-ended` 事件同步前端状态。单曲循环在后端解码线程处理，列表循环的自动切歌由前端事件处理逻辑触发。

## 当前限制与排查

- **仅面向本地文件**：尚未实现网络流媒体、播放列表导入 / 导出、随机播放或拖拽导入。
- **状态不持久化**：播放列表、循环模式和封面缓存保存在内存中，重启后不会恢复；音量也未保存到磁盘。
- **封面不是外部图片搜索**：仅尝试提取文件中的内嵌图片，不读取同目录封面文件，也不联网下载；界面以文件名作为曲目名称。
- **音频设备适配有限**：当前使用系统默认设备，没有设备选择界面；重采样不包含声道数转换，也未提供无缝播放保证。
- **列表时长为 `0:00`**：扫描时未能读取时长会记为 `0`，加载曲目时会再次读取时长。
- **开发启动失败**：检查 `1420` 端口、Rust / C++ 构建工具和 WebView 运行时。
- **没有声音或打开失败**：检查默认输出设备、系统音量、应用音量及文件编码；后端错误和解码日志可在开发模式终端查看。

需要更多 Rust 日志时，可以在 PowerShell 中运行：

```powershell
$env:RUST_LOG = "info"
npm run tauri dev
```

当前仓库未配置独立的自动化测试套件或 lint 脚本。前端构建和 `cargo check` 只能检查构建问题；格式兼容性、进度跳转、循环和实际音频输出仍需在桌面应用中使用音频样本验证。
