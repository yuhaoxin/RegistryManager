# Registry 管理器

Registry 管理器是一个用于管理本地 Docker Registry v2 实例的 Tauri v2 桌面应用。它专注于安全的本地工作流：手动注册本地 Registry v2 URL、浏览仓库和清单、按摘要删除清单、执行本地存储回收，并保留审计/缓存状态以便离线查看。

## 功能

- 手动管理本地 Registry v2 URL 配置。
- 浏览 Registry v2 端点中的仓库、标签、清单和摘要。
- 实时检查 Registry 健康/状态，支持手动刷新和离线缓存回退。
- 通过确认、影响预览和审计日志安全删除清单。
- 本地存储回收会停止本地 Registry 容器，使用临时容器执行真实的 Docker Registry GC，并在完成后尝试重启原容器。
- 使用 SQLite 离线缓存已浏览过的仓库和清单。
- 凭据存储在系统钥匙串中；不会以明文持久化凭据。

## 目标平台

| 平台 | 支持基线 | 说明 |
| --- | --- | --- |
| macOS | macOS 13+ | 推荐使用 Docker Desktop。 |
| Windows | Windows 10+ | 推荐使用带 WSL2 后端的 Docker Desktop。 |
| Linux | Ubuntu 22.04+ | 需要 Docker Engine 和 Tauri WebKitGTK 依赖。 |

本仓库未配置生产签名、公证和应用商店打包。除非你提供平台证书并单独配置 Tauri 签名，否则发布构建不会签名。

## 前置条件

- Rust stable 工具链和 Cargo。
- 与项目锁文件兼容的 Node.js。
- pnpm。
- Docker Desktop 或 Docker Engine。
- 当前操作系统所需的 Tauri 平台前置依赖。

## 安装

```bash
pnpm install
```

## 开发

以 Tauri 开发模式启动桌面应用：

```bash
pnpm tauri dev
```

如需在浏览器中开发界面，也可以只启动 Vite 前端：

```bash
pnpm dev
```

## 构建

为当前操作系统创建平台包：

```bash
pnpm tauri build
```

Tauri 配置使用产品名 `Registry Manager`、应用标识 `com.yuhaoxin.registry-manager`、1280x800 默认窗口，并启用当前平台可用的安装包目标。推送 `v*` 标签会触发 GitHub Release 工作流，分别构建 macOS `.dmg`、Windows NSIS/MSI、Linux `.deb`/`.AppImage` 产物；这里不声明安装器签名/公证能力。

## 测试

运行 Rust 测试：

```bash
cargo test
```

运行前端单元测试：

```bash
pnpm vitest run
```

运行浏览器 E2E 测试：

```bash
pnpm playwright test
```

运行前端生产构建/类型检查：

```bash
pnpm build
```

## 本地 Docker Registry 设置

在 `localhost:5000` 启动一个一次性的本地 Registry v2 容器：

```bash
docker run -d -p 5000:5000 --name registry registry:2
```

如果名称 `registry` 或端口 `5000` 已被占用，请删除或重命名现有测试容器，或在合适场景使用仓库提供的 `localhost:5001` 测试夹具。

在应用中创建一个使用本地端点 URL 的配置即可添加 Registry，例如 `http://localhost:5000`。仪表盘会读取所选配置的实时状态，并且只在请求或手动触发刷新时刷新仓库/标签/清单；如果实时读取失败，之前缓存的数据仍可用，并会标记为过期。

## 本地 GC 工作流

当选中的 Registry URL 是本地回环地址（例如 `localhost`、`127.0.0.1` 或 `::1`）时，仪表盘会显示“本地 GC 执行器”。远程 Registry 不会显示 GC 入口，后端也会拒绝对远程 Registry 或远程 Docker 上下文执行破坏性工作流。

运行 GC 前，应用会要求确认停机风险。确认后，桌面端会调用 `run_local_gc` 命令，并执行以下流程：

1. 校验 Registry URL 仅指向本机，并确认 Docker 上下文是本地 Docker Desktop/Engine。
2. 定位匹配端口、容器名或容器 ID 的 Registry 容器，并记录原容器状态、镜像、挂载、环境变量、重启策略和健康状态。
3. 确认 Registry 配置文件可读，并检查镜像支持 `registry garbage-collect`。
4. 停止原 Registry 容器。
5. 使用相同镜像和存储挂载启动临时容器，执行：

   ```bash
   registry garbage-collect --delete-untagged /etc/docker/registry/config.yml
   ```

6. 删除临时 GC 容器，重启原 Registry 容器，并检查 `/v2/` 健康状态。
7. 将结果写入审计日志，并把缓存中等待 GC 的清单状态更新为 `gc_completed` 或 `gc_failed`。

配置路径按以下顺序解析：容器环境变量 `REGISTRY_CONFIGURATION_PATH`、应用配置中的 `config_path`、默认路径 `/etc/docker/registry/config.yml`。如果 GC 失败，应用会尝试清理临时容器并重启原 Registry；如果自动恢复失败，请按界面和审计日志中的恢复指令手动处理。

## 安全说明

- 凭据通过操作系统钥匙串存储，不会写入明文项目文件或 SQLite 行。
- Registry 操作用于本地 Docker Engine/Desktop 上下文。破坏性工作流会拒绝远程 Docker 上下文。
- 删除和垃圾回收操作会在审计日志中记录状态和错误详情。
- 本地 GC 会造成 Registry 短暂停机；不要在镜像推送或其他写入操作进行中运行。
- 不要对生产 Registry 运行破坏性工作流。此 MVP 面向本地 Registry 维护设计。

## 删除和垃圾回收注意事项

- 清单删除按摘要执行，而不是按标签执行。标签会先解析为不可变清单摘要再删除。
- 删除清单后，相关标签或摘要会进入 `pending_gc` 状态；在本地 GC 成功完成之前，删除清单不保证释放存储空间。
- 本地存储回收只执行本机 Docker Registry 容器的真实 GC。它不会触发或管理远程 Registry 的垃圾回收。
- GC 会停止原 Registry 容器，并使用临时容器挂载同一份存储运行 `registry garbage-collect --delete-untagged`。
- 本地 GC 依赖 Registry 容器配置、存储挂载和配置路径。如果这些配置错误，命令可能失败且不会回收存储。
- GC 完成后应用会尝试重启原 Registry 并检查 `/v2/` 健康状态；失败时请参考审计日志中的恢复指令。

## 故障排查

### Docker 守护进程不可用

运行本地存储回收等依赖 Docker 的工作流前，请先启动 Docker Desktop 或 Docker Engine。在 Linux 上，可用 `docker ps` 验证守护进程。

### Docker 权限错误

确保当前用户可以访问 Docker socket。在 Linux 上，将用户加入 `docker` 组并开启新的登录会话。使用 `sudo` 可用于诊断，但不推荐在桌面应用正常使用时这样做，因为它会改变环境和凭据访问方式。

### Registry URL 不可用

确保所选手动配置指向本地 Registry v2 端点，并且 Registry 正在运行，例如：

```bash
docker run -d -p 5000:5000 --name registry registry:2
```

然后验证：

```bash
curl -fsS http://localhost:5000/v2/
```

### GC 失败

先查看应用中的 GC 时间线、实时日志和审计条目，再检查 Docker 日志：

```bash
docker logs registry
```

确认 Registry 配置路径存在于容器内，存储挂载已保留，并且镜像支持 `registry garbage-collect`。配置路径通常来自容器环境变量 `REGISTRY_CONFIGURATION_PATH`、应用配置中的 `config_path` 或默认 `/etc/docker/registry/config.yml`。

如果错误发生在停止原容器之后，应用会尝试删除临时 GC 容器并重启原 Registry。若界面显示需要手动恢复，请按提示执行 `docker start <container>`，然后再次检查健康状态。

### 重启失败

手动重启原 Registry 容器：

```bash
docker start <container>
```

然后检查健康状态：

```bash
curl -fsS http://localhost:5000/v2/
```

## 常用命令

```bash
pnpm install
pnpm tauri dev
pnpm tauri build
cargo test
pnpm vitest run
pnpm playwright test
pnpm build
docker run -d -p 5000:5000 --name registry registry:2
```
