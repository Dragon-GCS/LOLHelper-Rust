# LOL Helper

[![Version](https://img.shields.io/badge/version-0.2.4-blue.svg)](https://github.com/Dragon-GCS/LOLHelper-Rust)
[![Rust](https://img.shields.io/badge/rust-2024-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

> 本项目为 [LoLHelper](https://github.com/Dragon-GCS/LOLHelper) 的 Rust 重构版本，提供更高的性能和更低的资源占用。

## ✨ 功能特性

- 🎯 **自动接受对局** - 支持 0-15 秒延迟配置，避免掉线惩罚
- 🦸 **自动选择英雄** - 预设英雄后自动完成选择和确认
- 📊 **队友数据分析** - 实时分析队友战绩并自动发送至聊天框
- 👤 **召唤师信息查询** - 快速获取当前召唤师详细信息

## 🚀 快速开始

### 环境要求

- Windows 操作系统
- 英雄联盟客户端

### 从源码构建

```bash
# 克隆仓库
git clone https://github.com/Dragon-GCS/LOLHelper-Rust.git
cd LOLHelper-Rust

# 开发模式构建
cargo build

# 发布模式构建
cargo build --release

# 运行
cargo run --release
```

## 📂 项目结构

```shell
lol-helper/
├── lcu-backend/        # LCU API 后端库
│   └── src/
│       ├── api/        # API 端点实现
│       └── events/     # 事件监听器
├── src/                # 主应用程序
├── examples/           # 示例代码
└── windows/            # Windows 资源文件
```

## 🛠️ 技术栈

- **GUI 框架**: [eframe](https://github.com/emilk/egui) - 跨平台即时模式 GUI
- **异步运行时**: [Tokio](https://tokio.rs/) - Rust 异步编程框架
- **日志系统**: [log4rs](https://docs.rs/log4rs/) - 灵活的日志框架
- **序列化**: [serde](https://serde.rs/) - Rust 序列化框架

## ⚠️ 已知问题

### 自动接受对局间歇性失效

- **现象**: 每隔一段时间自动接受功能不可用，需要在客户端弹出对局接受界面时启动助手
- **原因**: 助手基于 WebSocket 事件处理 `/lol-matchmaking/v1/ready-check` 事件，但每天第一场对局该事件可能无法接收
- **临时解决方案**: 在对局接受界面出现后启动助手，后续可正常工作

### 海克斯乱斗英雄选择问题

- **现象**: 海克斯乱斗模式的二/三选一时部分英雄无法自动选择
- **原因**: 海克斯乱斗的英雄选择 API 与普通模式不同，请求格式仍在研究中
- **状态**: 开发中

## 🤝 贡献指南

欢迎提交 Issue 和 Pull Request！

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启 Pull Request

## 📝 许可证

本项目采用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情

## 🔗 相关链接

- [原 Python 版本](https://github.com/Dragon-GCS/LOLHelper)
- [问题反馈](https://github.com/Dragon-GCS/LOLHelper-Rust/issues)

## ⭐ Star History

如果这个项目对您有帮助，请给它一个 Star！

---

**免责声明**: 本项目仅供学习交流使用，使用本工具产生的任何后果由使用者自行承担。
