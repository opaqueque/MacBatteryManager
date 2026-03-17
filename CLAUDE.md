# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

这是一个针对 Apple Silicon (M1/M2/M3) MacBook 的电池管理开源 GUI 软件。采用 Tauri 2.0 框架构建，实现跨平台原生应用，底层依赖于 `bclm` 命令行工具修改 SMC 寄存器（限制 80% 或充满 100%）。

**技术栈**：
- **后端**: Tauri 2.0 + Rust
- **前端**: Vite + Vue 3 (TypeScript) + Tailwind CSS
- **构建工具**: npm + Cargo

## 项目结构

```
MacBatteryManager/                    # 工作空间根目录
├── battery-manager/                  # 主项目目录
│   ├── src/                         # 前端源代码 (Vue 3 + TypeScript)
│   │   ├── App.vue                  # 主应用组件
│   │   ├── main.ts                  # 应用入口
│   │   ├── assets/                  # 静态资源
│   │   ├── components/              # Vue 组件目录
│   │   └── types/                   # TypeScript 类型定义
│   ├── src-tauri/                   # Tauri 后端 (Rust)
│   │   ├── Cargo.toml              # Rust 项目配置
│   │   ├── tauri.conf.json         # Tauri 应用配置
│   │   └── src/
│   │       ├── lib.rs              # Rust 库入口（主要命令实现）
│   │       └── main.rs             # 应用入口（调用 lib.rs）
│   ├── package.json                # 前端依赖和脚本
│   ├── vite.config.ts              # Vite 配置
│   └── index.html                  # HTML 入口
└── CLAUDE.md                       # 本项目说明文件
```

## 常用命令

### 开发环境
```bash
# 进入项目目录
cd battery-manager

# 安装前端依赖
npm install

# 开发模式运行（同时启动 Vite 开发服务器和 Tauri 应用）
npm run tauri dev

# 仅启动前端开发服务器（不启动 Tauri）
npm run dev
```

### 构建与发布
```bash
# 构建前端（TypeScript 检查 + Vite 构建）
npm run build

# 构建 Tauri 应用（调试版）
npm run tauri build

# 构建 Tauri 应用（发布版）
npm run tauri build -- --release

# 构建并生成安装包
npm run tauri build -- --bundles
```

### Rust 后端相关
```bash
# 进入 Tauri 后端目录
cd src-tauri

# 添加 Rust 依赖
cargo add <package-name>

# 运行 Rust 测试
cargo test

# 检查编译（不生成二进制）
cargo check

# 格式化 Rust 代码
cargo fmt
```

### 前端工具
```bash
# TypeScript 类型检查
npx vue-tsc --noEmit

# 安装额外依赖
npm install <package-name>

# 安装开发依赖
npm install -D <package-name>
```

## 核心架构

### 权限与安全模型
为避免 GUI 持续以 root 权限运行，采用"前后端分离"策略：
1. **GUI 层**: 普通用户权限，负责用户交互和配置文件管理
2. **命令执行**: 通过 `osascript` 唤起 macOS 原生密码/指纹验证，单次提权执行 `bclm` 命令
3. **配置文件**: GUI 生成/修改 `~/.config/battery-manager/schedule.json`

### Rust 后端命令
后端提供以下 Tauri 命令（定义在 `src-tauri/src/lib.rs`）：
- `set_battery_limit(limit: u8)` - 通过 osascript 提权执行 bclm 命令
- `save_schedule(config: String)` - 保存课表配置到 JSON 文件
- `load_schedule()` - 从 JSON 文件加载配置

### 前端组件结构
- **App.vue**: 主应用布局，包含手动控制按钮和课表设置容器
- **ScheduleForm.vue**: 课表设置表单组件，处理用户输入并调用 Rust 命令
- **类型定义**: `src/types/schedule.ts` 定义配置数据结构

### 配置文件格式
配置文件位于 `~/.config/battery-manager/schedule.json`：
```json
{
  "class_days": [2, 3, 4],      // 0=周一, 1=周二, ..., 6=周日
  "class_time": "10:10",        // 上课时间 (HH:MM)
  "charge_start_offset": -2     // 提前多少小时开始充电（负数）
}
```

### 定时任务系统
独立于 GUI 的 Shell 脚本 (`~/.config/battery-manager/battery-scheduler.sh`)：
1. 读取 JSON 配置文件
2. 根据当前时间和课表计算是否需要充电
3. 通过 `launchd` 或 `crontab` 定时执行
4. 执行时通过 `osascript` 提权运行 `bclm`

## 开发注意事项

### 依赖安装
**必需的系统工具**：
```bash
# 安装 bclm（电池限制管理工具）
brew install bclm

# 或从源码编译安装
git clone https://github.com/zackelia/bclm.git
cd bclm
make
sudo make install
```

**前端样式**：
项目计划使用 Tailwind CSS，但当前配置尚未完成。如需添加：
```bash
# 安装 Tailwind CSS
npm install -D tailwindcss postcss autoprefixer

# 初始化配置
npx tailwindcss init -p
```

### Rust 依赖管理
- 所有 Rust 依赖在 `src-tauri/Cargo.toml` 中定义
- 主要依赖：`tauri`, `serde`, `serde_json`, `tokio`
- 添加新依赖时注意版本兼容性

### 跨平台考虑
- **macOS**: 主要目标平台，使用 `osascript` 进行权限提升
- **其他平台**: 当前功能仅针对 macOS，其他平台需要适配

### 调试提示
1. **Tauri 开发工具**: 运行 `npm run tauri dev` 打开开发者工具
2. **Rust 日志**: 在 Rust 代码中使用 `println!` 或 `eprintln!`
3. **前端调试**: 使用 Vue Devtools 和浏览器开发者工具

## 配置文件路径
- **应用配置**: `~/.config/battery-manager/schedule.json`
- **日志文件**: `~/.config/battery-manager/battery-scheduler.log`
- **Shell 脚本**: `~/.config/battery-manager/battery-scheduler.sh`

## 重要文件说明
- `src-tauri/src/lib.rs` - Rust 命令实现的核心文件
- `src/App.vue` - 前端主应用组件
- `src/components/ScheduleForm.vue` - 课表设置组件
- `src-tauri/tauri.conf.json` - Tauri 应用配置（窗口大小、权限等）

## 扩展开发
1. **添加新功能**: 在 `lib.rs` 中添加新的 `#[tauri::command]` 函数
2. **修改界面**: 编辑 Vue 组件和 Tailwind CSS 类
3. **调整定时逻辑**: 修改 Shell 脚本中的时间计算逻辑
4. **添加平台支持**: 为其他操作系统实现相应的权限提升机制