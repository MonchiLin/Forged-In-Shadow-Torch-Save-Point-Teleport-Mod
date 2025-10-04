# 仓库协作说明

> 本文件仅使用中文描述。

## 项目概述
本项目是一个基于 Tauri + Vue 的桌面应用，用于《Forged In Shadow Torch》游戏的保存点传送功能。前端提供地图选择和标记点交互界面，通过文件通信与游戏内的 UE4SS Lua 模组进行数据交换。

## 目录与模块规划
- **前端界面**：`src/` 目录包含 Vue 组件，`App.vue` 为主界面，`style.css` 管理样式（极简透明设计）
- **地图配置**：
  - `src/config/in_game_points.json` - 游戏内提取的保存点坐标数据（包含 i、name、x、y、z 字段）
  - 不再使用 `maps.json`，地图数据在 `App.vue` 中硬编码定义
  - markers 的 `id` 和 `label` 字段均使用 `in_game_points.json` 中的 `name` 字段（如 `BP_SavePoint10_2`）
- **Tauri 后端**：`src-tauri/` 包含 Rust 代码和配置文件
  - `src/main.rs` - 主程序，处理 IPC 和文件通信
  - `src/window_control.rs` - 窗口管理（1280×900 固定、透明度、DPI 适配）
  - `src/gamepad.rs` - Xbox 手柄输入（仅 Windows）
- **Lua 模组**：`scripts/main.lua` - UE4SS 脚本，处理保存点扫描和传送
- **静态资源**：`assets/` - 地图图片（仅包含 `WorldMinimap.png`，尺寸 1600×800）

## 构建与调试
```bash
npm run dev          # 浏览器调试
npm run tauri:dev    # 桌面应用调试
npm run build        # 前端生产构建（输出到 dist/）
npm run tauri:build  # 完整应用构建（输出到 src-tauri/target/release/）
npm run mod          # 运行 Lua 模组（用于开发调试）
```

## 自动发布
项目配置了 GitHub Actions 自动发布流程：

### 工作流程
1. 推送代码到 `master` 分支自动触发发布
2. 从 `package.json` 读取版本号（如 `0.1.0`）
3. 编译 Windows 安装包（.msi 和 .exe）
4. 创建 Git tag（如 `v0.1.0`）
5. 创建 GitHub Release 并上传安装包

### 版本更新
修改 `package.json` 中的 `version` 字段，然后提交推送：
```json
{
  "version": "0.2.0"
}
```

### 配置文件
- `.github/workflows/release.yml` - GitHub Actions 工作流配置
- `tauriScript: npm run tauri:build` - 使用项目的构建脚本
- 权限：`contents: write` 允许创建 release 和 tag

## 文件通信机制
**目的**：Tauri 与 UE4SS Lua 模组通过文件进行进程间通信。

**通信目录**：
- `%TEMP%\Forged-In-Shadow-Torch-Save-Point-Teleport-Mod\`
- 文件：`cmd.txt`（命令）、`resp.txt`（响应）

**通信协议**：
1. **Rust 发送**：清理旧文件 → 写入命令（附带时间戳）→ 轮询等待响应
2. **Lua 接收**：轮询检测 `cmd.txt` → 读取并删除 → 解析命令和时间戳
3. **Lua 响应**：执行命令 → 写入 `resp.txt`（响应内容 + 时间戳后缀）
4. **Rust 接收**：读取 `resp.txt` → 验证时间戳匹配 → 删除文件并返回结果

**时间戳机制**：
- 命令格式：`SCAN 1728000000123`（Unix 毫秒时间戳）
- 响应格式：`{...} TIMESTAMP:1728000000123`
- 作用：防止读取过时响应，确保请求-响应一一对应

**支持命令**：
- `PING [timestamp]` → `PONG TIMESTAMP:...`
- `SCAN [timestamp]` → `{"save_points":[{"i":1,"name":"BP_SavePoint10_2","x":0,"y":-177863.53,"z":5284.82},...]} TIMESTAMP:...`
- `TP <index> [timestamp]` → `OK/ERR... TIMESTAMP:...`
- `TPNAME <name> [timestamp]` → `OK teleported TIMESTAMP:...` / `ERR savepoint not found: <name> TIMESTAMP:...`
  - 优先精确匹配保存点名称，失败后使用子串匹配
- `MOVE <x> <y> <z> [timestamp]` → `OK/ERR... TIMESTAMP:...`
- `POS [timestamp]` → `{JSON...} TIMESTAMP:...`

**路径独立性**：Rust 使用 `env::temp_dir()`，Lua 使用 `os.getenv('TEMP')`，双方各自创建子目录，无需相互依赖安装路径。

**错误处理**：
- 超时（5秒）自动清理文件并返回错误
- 时间戳不匹配时忽略响应并继续等待
- 所有操作完成后清理临时文件

## 编码规范
### 前端
- CSS：极简透明设计，所有背景使用 `transparent`
- Vue：`<script setup>` 语法，`camelCase` 变量，`SCREAMING_SNAKE_CASE` 常量
- 地图配置：从 `in_game_points.json` 导入，坐标映射使用模式 2（X=游戏Y坐标，Y=100-游戏Z坐标）
- 坐标转换公式：
  ```javascript
  x = ((point.y - yMin) / (yMax - yMin) * 100);
  y = 100 - ((point.z - zMin) / (zMax - zMin) * 100);
  ```
  其中 yMin=-177863, yMax=117350, zMin=-30545, zMax=82281

### Rust
- 执行 `cargo fmt` 格式化
- IPC 命令：`push_coord`、`scan_save_points`、`teleport_to_savepoint`、`set_window_opacity`
- 窗口尺寸：使用 `LogicalSize` 而非 `PhysicalSize`
- 文件通信：使用 `send_lua_command(cmd)` 统一处理命令发送和响应接收

### Lua
- 单文件结构（`scripts/main.lua`）
- **禁止使用 `string.format` 和 `\n`**：UE4SS print 函数兼容性问题，使用字符串拼接
- 示例：`print("[Mod] Value: " .. tostring(value))` ✅
- 错误：`print(string.format("[Mod] Value: %d", value))` ❌
- 兼容 Windows 路径分隔符
- **K2_SetActorLocation 调用规范**：第三个参数必须传入 Lua 表（如 `out_hit = {}`）接收碰撞信息
  - 正确：`pawn:K2_SetActorLocation(location, false, {}, true)` ✅
  - 错误：`pawn:K2_SetActorLocation(location, false, false, true)` ❌

## 调试功能
### 调试模式
- 点击窗口顶部的 "Debug" 按钮开启/关闭调试面板
- 调试模式下可拖拽地图标记点，调整其位置
- 拖拽结束后自动复制 JSON 坐标到剪贴板，格式如下：
  ```json
  {
    "i": 9,
    "name": "BP_SavePoint19",
    "x": 0,
    "y": 116446,
    "z": 11772
  }
  ```
  其中 `y` 和 `z` 字段为调整后的地图百分比坐标（地图 x% → 游戏 y，地图 y% → 游戏 z），需手动替换到 `in_game_points.json` 中

### 地图交互
- **缩放**：鼠标滚轮朝向鼠标位置缩放（0.6x-3.0x）
- **平移**：鼠标右键或中键拖拽（禁用左键拖拽，避免与标记点拖拽冲突）
- **重置**：双击地图重置缩放和平移到初始状态
- **标记点拖拽**：
  - 调试模式下左键拖拽标记点，实时更新位置
  - 拖拽时禁用地图缩放和平移，避免冲突
  - 使用响应式 `maps` ref 确保 Vue 实时更新标记位置
  - 拖拽过程中根据地图当前 transform（缩放/平移）实时计算正确坐标

## 测试要求
### 前端测试
- 缩放：滚轮朝向鼠标位置缩放，范围 0.6x-3.0x
- 平移：右键或中键拖拽地图
- 重置：双击地图恢复初始缩放和位置
- 标记点点击：传送到对应保存点
- `Ctrl+Shift+Space` 窗口显示/隐藏
- 手柄导航（Xbox 手柄）
- 不透明度滑块（40%-100%，默认 100%）
- 调试模式：
  - 点击 "Debug" 按钮切换调试面板
  - 拖拽标记点，实时看到位置变化
  - 松开鼠标后自动复制 JSON 到剪贴板
  - 拖拽期间地图缩放/平移被禁用

### 文件通信测试
1. 游戏加载 UE4SS 和 `scripts/main.lua` 模组
2. UE4SS 控制台显示 `[SavePointTeleport] Initialization complete!`
3. 点击前端 "Scan" 按钮，前端显示"已扫描到 X 个保存点"
4. 检查 `%TEMP%\Forged-In-Shadow-Torch-Save-Point-Teleport-Mod\` 目录
5. 浏览器控制台输出扫描结果 JSON

### 传送测试
1. 确保 `in_game_points.json` 中保存点名称与游戏内一致
2. 点击地图标记点，首次点击会自动扫描
3. 检查 UE4SS 控制台：`[SavePointTeleport] Teleporting to: BP_SavePoint10_2`
4. 验证玩家位置已传送到目标保存点
5. 如果传送失败，检查 Lua 控制台错误信息和 `out_hit` 参数传递

### 构建测试
- `npm run build` 无错误
- `npm run tauri:build` 生成可执行文件

## 功能需求
### 界面
- 窗口：1280×900 固定，不可调整大小，透明背景
- Header：地图名称、Scan 按钮、不透明度滑块
- 地图选择器：卡片式布局，支持手柄导航（未选中/聚焦/选中三态）
- 地图画板：显示地图图片和标记点，支持缩放拖拽

### 标记点
- 样式：青色空心圆圈（8px）+ 文字标签
- 悬停/选中：圆圈变白色，文字变青色
- 点击：调用 `teleport_to_savepoint` IPC 命令，发送 `marker.id` 到 Lua 执行传送
- **首次传送自动扫描**：如果未执行过 Scan，首次点击标记点会自动触发 `SCAN` 命令

### 手柄控制
- 左摇杆/D-Pad：切换标记点或地图
- A 键：选择标记点或确认地图切换
- B 键：进入地图选择模式
- 双击 B 键：重置地图缩放

### Scan 功能
- 点击 Scan 按钮触发 Lua `SCAN` 命令
- 扫描结果输出到浏览器控制台，并更新 `hasScanned` 状态
- 显示提示："已扫描到 X 个保存点"

### 传送功能
- **工作流程**：
  1. 用户点击地图标记点（`marker.id` 如 `BP_SavePoint10_2`）
  2. 前端检查 `hasScanned` 状态，首次传送时自动执行 `SCAN`
  3. 调用 `teleport_to_savepoint` IPC 命令发送 `TPNAME <id>` 到 Lua
  4. Lua 在 `S.save_points` 列表中查找匹配的保存点（精确/子串匹配）
  5. 找到后调用 `K2_SetActorLocation` 传送玩家
- **错误处理**：
  - 如果保存点未找到，提示"保存点未找到，请点击 Scan 重新扫描"
  - 自动重置 `hasScanned` 状态，下次点击重新扫描

## 提交规范
- Conventional Commits：`feat:`、`fix:`、`chore:` 等
- PR 描述：列出改动模块、验证步骤、截图（如有 UI 变更）

## 配置维护
- 窗口尺寸：`src-tauri/tauri.conf.json` 和 `src-tauri/src/window_control.rs` 同步
- IPC 命令：`src-tauri/src/main.rs` 的 `invoke_handler` 注册
- 依赖更新后重新执行 `cargo fmt` 和构建流程
