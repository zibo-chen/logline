# Logline - 高性能实时日志查看器

[English](README_EN.md) | 简体中文

![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)
![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-blue.svg)
![License](https://img.shields.io/badge/License-Apache2-green.svg)

Logline 是一个使用 Rust + egui 开发的跨平台日志查看器应用，专注于提供高性能的实时日志监控和分析能力。支持 MCP (Model Context Protocol)，可与 AI 助手集成进行智能日志分析。

## 📸 应用截图

<div align="center">
  <img src="res/application.png" alt="Logline 主界面" width="800"/>
  <p><i>Logline 主界面 - 实时日志监控与高亮显示</i></p>
</div>

<div align="center">
  <img src="res/mcp_support.png" alt="MCP 集成" width="800"/>
  <p><i>MCP 支持 - AI 驱动的日志分析</i></p>
</div>

## ✨ 功能特性

### 核心功能
- 🔄 **实时文件监控** - 自动检测文件变化，支持 tail -f 风格的自动滚动
- 📜 **虚拟滚动** - 高效渲染百万行级别的日志文件，保持流畅的 60 FPS
- 🔍 **强大搜索** - 支持关键词搜索、正则表达式、大小写敏感选项
- 🎨 **语法高亮** - 自动识别日志级别 (ERROR/WARN/INFO/DEBUG/TRACE) 并着色显示
- 🏷️ **日志级别过滤** - 快速筛选特定级别的日志，支持多选
- 🌐 **远程日志支持** - 通过 [logline-agent](https://github.com/zibo-chen/logline-agent) 实时查看远程服务器的日志文件

### 高级功能
- 📑 **书签功能** - 标记重要日志行，支持添加备注，快速跳转定位
- 🤖 **MCP 集成** - 支持 Model Context Protocol，可与 AI 助手 (Claude、ChatGPT 等) 集成
  - 智能日志分析：错误模式识别、时间线分析、统计摘要
  - 自然语言查询：用自然语言描述查询条件
  - 远程日志管理：统一管理本地和远程日志源
- 🔬 **高级过滤** - 多条件组合过滤，支持时间范围筛选
- 🌓 **主题切换** - 支持明亮/暗黑主题，自适应系统主题
- ⌨️ **快捷键支持** - 高效的键盘操作，提升工作效率
- 💾 **配置持久化** - 自动保存窗口大小、最近文件、偏好设置
- 📊 **性能监控** - 实时显示内存使用、日志行数等统计信息

## 🚀 快速开始

### 系统要求
- Rust 1.75 或更高版本
- 支持 OpenGL 3.2 或更高版本的显卡驱动

### 安装方式

#### 方式一：从源码编译

```bash
# 克隆项目
git clone https://github.com/zibo-chen/logline.git
cd logline

# 编译运行 (开发模式)
cargo run

# 编译发布版本 (推荐)
cargo build --release

# 运行发布版本
./target/release/logline
```

#### 方式二：使用预编译二进制

从 [Releases](https://github.com/zibo-chen/logline/releases) 页面下载适合你操作系统的预编译版本。

### 基本使用

#### 打开本地日志文件

1. 启动应用后点击 "📂 Open" 按钮
2. 或使用快捷键 `Cmd/Ctrl + O`
3. 选择要查看的日志文件

#### 查看远程日志

1. 在远程服务器上启动 logline-agent：
```bash
logline-agent --name my-server --file /var/log/app.log --server 127.0.0.1:12500
```

2. 在 Logline 主应用中，远程日志源会自动出现在资源管理器中

### MCP 集成使用

Logline 支持 MCP (Model Context Protocol)，可以与 AI 助手集成进行智能日志分析。

#### 配置 Claude Desktop

在 Claude Desktop 的配置文件中添加：

**macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
**Windows**: `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "logline": {
      "command": "/path/to/logline",
      "args": ["--mcp"],
      "env": {}
    }
  }
}
```

#### MCP 功能

启用 MCP 后，AI 助手可以：

- **分析日志**：识别错误模式、统计分布、时间线分析
- **智能搜索**：使用自然语言描述查询条件
- **错误诊断**：自动聚合相似错误，提供上下文
- **趋势分析**：分析日志频率变化，发现异常峰值
- **书签管理**：智能标记重要日志行

示例提示词：
```
"分析最近一小时的错误日志"
"找出所有包含 'OutOfMemory' 的错误，并显示上下文"
"统计各个日志级别的分布情况"
"分析日志活动的时间趋势"
```

## ⌨️ 快捷键

| 快捷键 | 功能 |
|--------|------|
| `Cmd/Ctrl + O` | 打开文件 |
| `Cmd/Ctrl + F` | 打开搜索 |
| `Cmd/Ctrl + L` | 清空显示 |
| `Cmd/Ctrl + G` | 跳转到行 |
| `Cmd/Ctrl + B` | 切换书签 |
| `Cmd/Ctrl + C` | 复制选中行 |
| `Space` | 暂停/恢复自动滚动 |
| `Home` | 跳转到顶部 |
| `End` | 跳转到底部 |
| `F3` | 查找下一个 |
| `Shift + F3` | 查找上一个 |
| `Esc` | 关闭搜索/对话框 |

## 📁 项目架构

```
logline/
├── Cargo.toml              # 项目配置和依赖
├── src/
│   ├── main.rs             # 应用入口
│   ├── app.rs              # egui 应用主逻辑
│   ├── file_watcher.rs     # 文件监控模块 (基于 notify)
│   ├── log_reader.rs       # 日志读取和解析
│   ├── log_buffer.rs       # 环形缓冲区管理
│   ├── log_entry.rs        # 日志条目数据结构
│   ├── virtual_scroll.rs   # 虚拟滚动实现 (支持百万行)
│   ├── search.rs           # 搜索和过滤引擎
│   ├── highlighter.rs      # 语法高亮渲染
│   ├── config.rs           # 配置管理和持久化
│   ├── protocol.rs         # 远程通信协议
│   ├── remote_server.rs    # 远程日志服务器
│   ├── mcp/                # MCP (Model Context Protocol) 模块
│   │   ├── mod.rs          # 模块导出
│   │   ├── server.rs       # MCP 服务器实现
│   │   ├── tools.rs        # MCP 工具定义 (9+ 工具)
│   │   └── types.rs        # MCP 类型定义
│   └── ui/                 # UI 组件
│       ├── mod.rs
│       ├── main_view.rs    # 主日志显示区
│       ├── search_bar.rs   # 搜索栏
│       ├── filter_panel.rs # 过滤面板
│       ├── explorer_panel.rs # 资源管理器
│       ├── global_search_panel.rs # 全局搜索
│       ├── settings_panel.rs # 设置面板
│       ├── status_bar.rs   # 状态栏
│       ├── toolbar.rs      # 工具栏
│       └── activity_bar.rs # 活动栏
├── logline-agent/          # 远程日志代理 (https://github.com/zibo-chen/logline-agent)
├── res/                    # 应用资源
│   ├── application.png     # 应用截图
│   └── mcp_support.png     # MCP 功能截图
├── assets/                 # 字体等运行时资源
└── README.md
```

## 🔧 技术栈

- **语言**: Rust 1.75+
- **GUI 框架**: egui 0.29 / eframe (即时模式 GUI)
- **文件监控**: notify 6.0 (跨平台文件系统事件)
- **异步运行时**: tokio 1.x (异步 I/O 和网络)
- **日志解析**: regex (正则表达式引擎)
- **序列化**: serde + serde_json (配置和数据序列化)
- **MCP 协议**: rmcp (Model Context Protocol 实现)
- **远程通信**: 自定义二进制协议 (基于 TCP)

## 🎯 MCP 工具列表

Logline 提供以下 MCP 工具供 AI 助手使用：

| 工具名称 | 功能描述 |
|---------|---------|
| `list_log_sources` | 列出所有可用的日志源 (本地文件和远程流) |
| `get_log_entries` | 分页读取日志条目，支持级别过滤 |
| `search_logs` | 搜索日志，支持关键词和正则表达式 |
| `advanced_filter` | 高级多条件组合过滤 (AND 逻辑) |
| `get_log_statistics` | 获取日志统计信息 (总行数、级别分布、错误率) |
| `analyze_errors` | 分析错误模式，聚合相似错误 |
| `analyze_timeline` | 分析日志时间线，统计时间段内的日志频率 |
| `list_bookmarks` | 列出所有书签 |
| `manage_bookmarks` | 管理书签 (添加/删除/切换/清空) |

## 📊 性能指标

- ✅ 支持 **100 万行以上**的日志文件
- ✅ 虚拟滚动保持 **60 FPS** 流畅度
- ✅ 内存占用优化：环形缓冲区，默认最多缓存 **10 万行**
- ✅ 实时文件监控，**<100ms** 更新延迟
- ✅ 搜索性能：百万行日志 **秒级响应**
- ✅ 远程日志传输：高效的二进制协议，**低带宽占用**

## 🧪 测试场景

经过以下场景验证：

- ✅ 5GB+ 日志文件 (>500万行) 的加载和滚动
- ✅ 实时监控高频写入的日志 (每秒1000+行)
- ✅ 复杂正则表达式搜索大文件
- ✅ 多个远程日志源同时连接
- ✅ 长时间运行 (24小时+) 的内存稳定性

## 🛠️ 配置

配置文件位置:
- macOS: `~/Library/Application Support/logline/config.toml`
- Linux: `~/.config/logline/config.toml`
- Windows: `%APPDATA%\logline\config.toml`

配置示例:
```toml
[window]
width = 1200.0
height = 800.0
maximized = false

[display]
font_size = 13.0
line_height = 1.4
show_line_numbers = true
word_wrap = false

[buffer]
max_lines = 100000
auto_trim = true
```

## 🗺️ 开发路线图

### Phase 1 - MVP ✅ (已完成)
- [x] 基础 GUI 框架搭建
- [x] 打开并显示日志文件
- [x] 虚拟滚动实现
- [x] 实时文件监控
- [x] 基础语法高亮

### Phase 2 - 核心功能 ✅ (已完成)
- [x] 关键词搜索功能
- [x] 日志级别过滤
- [x] 暂停/恢复功能
- [x] 快捷键支持
- [x] 跳转到行

### Phase 3 - 增强功能 ✅ (已完成)
- [x] 正则表达式搜索
- [x] 书签功能 (支持备注)
- [x] 配置持久化
- [x] 主题切换
- [x] 远程日志支持
- [x] 资源管理器面板

### Phase 4 - AI 集成 ✅ (已完成)
- [x] MCP 协议实现
- [x] 9+ 日志分析工具
- [x] 错误模式识别
- [x] 时间线分析
- [x] 高级多条件过滤


## 📄 许可证

本项目采用 Apache-2.0 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。

## 🤝 贡献

欢迎贡献！无论是报告 Bug、提出新功能建议，还是提交代码改进，都非常欢迎。

### 开发指南

```bash
# 克隆仓库
git clone https://github.com/zibo-chen/logline.git
cd logline

# 运行测试
cargo test

# 运行检查
cargo clippy

# 格式化代码
cargo fmt

# 构建文档
cargo doc --open
```

## ⭐ Star History

如果这个项目对你有帮助，请给一个 ⭐️ Star！

---
