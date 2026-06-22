# LM Studio 启动与模型预加载设计

## 目标

当 Pot 启动且已配置 LM Studio 翻译服务时，自动启动本机 LM Studio 服务并把第一个已配置的翻译模型加载到内存。该流程用于减少开机后首次翻译的等待时间，同时不能阻塞或破坏 Pot 的正常启动。

成功标准：

- Pot 启动时能找到翻译服务列表中的第一个 LM Studio 实例。
- 本机 LM Studio 服务未运行时，Pot 能通过官方 `lms` CLI 启动配置端口上的服务。
- 服务就绪后，Pot 能识别模型是否已加载，并只在需要时调用原生加载接口。
- 远程 LM Studio 地址不会触发本机进程启动。
- 启动、连接或加载失败仅写入日志，不阻塞 Pot 主界面。

## 已确认方案

采用“CLI 启动服务 + REST 显式加载模型”的方案：

1. 使用 `lms server start --port <port>` 启动 LM Studio 本地服务。
2. 使用 `GET /api/v1/models` 检查模型及其 `loaded_instances`。
3. 模型未加载时，调用 `POST /api/v1/models/load`，请求体为 `{ "model": "<配置模型>" }`。

未采用的方案：

- 启动 LM Studio GUI：开机时会出现窗口，且可执行文件路径兼容成本更高。
- 发送测试翻译触发 JIT 加载：依赖服务端自动加载设置，不能保证模型被显式加载。

## 启动流程

新增独立的 LM Studio 预热模块，并在 `src/main.jsx` 完成存储与环境初始化后以 fire-and-forget 方式调用。

预热模块按以下顺序执行：

1. 读取 `translate_service_list`。
2. 按列表顺序找到第一个服务名为 `lmstudio` 的实例。
3. 读取该实例的 `baseUrl`、`model` 和可选 `apiKey`。
4. 规范化 Base URL，并判断主机是否为 `localhost`、`127.0.0.1` 或 `::1`。
5. 探测 `GET /api/v1/models`。
6. 如果是本地地址且服务不可达，调用 Tauri 命令启动 `lms server start --port <baseUrl 端口>`。
7. 最多等待 30 秒，每 500 毫秒探测一次服务。
8. 检查目标模型的 `key` 或 `loaded_instances[].id`。
9. 已加载则结束；未加载则调用 `POST /api/v1/models/load`。

整个流程不被 `await` 到主界面渲染链中，模型加载慢时 Pot 仍应立即可用。

## Tauri 启动命令

在 Rust 命令模块中新增 `start_lmstudio_server(port)`：

- 优先执行 PATH 中的 `lms`。
- Windows 额外检查 `%USERPROFILE%\.lmstudio\bin\lms.exe`。
- 使用独立子进程执行 `lms server start --port <port>`。
- Windows 使用无控制台窗口的进程创建标志，避免开机弹出命令行窗口。
- 端口只能来自已经解析成功的 URL，并限制为有效的 `u16`。
- 找不到 CLI 或启动失败时返回可记录的错误，不尝试启动 GUI。

如果服务已经运行，JavaScript 探测会跳过该命令，因此正常路径不会重复创建服务进程。

## 地址与认证

- OpenAI 兼容 Base URL 默认为 `http://localhost:1234/v1`。
- 原生管理 API 使用相同 origin，路径分别为 `/api/v1/models` 和 `/api/v1/models/load`。
- API Token 非空时发送 `Authorization: Bearer <token>`；为空时不发送认证头。
- HTTPS、本地自定义端口和远程地址均保留用户配置。
- 远程地址仅执行探测与模型加载，不调用本机 `lms` CLI。

## 模型选择与资源边界

- 只处理翻译服务列表中的第一个 LM Studio 实例。
- 只加载该实例配置的一个模型。
- 不自动加载第二个实例或模型，避免一次占满内存或显存。
- 不设置 TTL、上下文长度或 GPU 卸载参数，沿用 LM Studio 默认加载策略。
- 不下载缺失模型；模型不存在时记录服务端错误。

## 并发与错误处理

- 使用模块级进行中 Promise，避免开发模式重载或并发调用重复启动、重复加载。
- 无 LM Studio 配置、模型为空或配置损坏时直接跳过。
- 服务启动后 30 秒仍不可达时记录警告并结束。
- `GET /api/v1/models` 返回异常结构时记录警告，不发送盲目加载请求。
- `POST /api/v1/models/load` 非成功状态时记录状态码和响应内容。
- 所有失败均被预热模块捕获，不向 `main.jsx` 抛出未处理异常。

## 测试与验证

测试先行覆盖纯逻辑：

- 从服务实例列表选择第一个 LM Studio 配置。
- 将 OpenAI Base URL 转换为 LM Studio 原生管理 API URL。
- 判断本地与远程地址。
- 从原生模型列表识别目标模型是否已加载。
- 空 Token 与非空 Token 的认证头。

实现完成后运行：

- `rtk pnpm test`
- `rtk pnpm build`
- `rtk cargo check`（在 `src-tauri` 下）

本机集成验证：停止 LM Studio 本地服务后启动 Pot，确认服务重新监听配置端口，`GET /api/v1/models` 中目标模型的 `loaded_instances` 非空，并完成一次翻译请求。该验证会短暂重启本机 LM Studio 服务，但不会修改模型文件或用户凭据。

## 改动边界

- 新增 LM Studio 预热模块和对应测试。
- 新增一个 Tauri 启动命令并在命令注册表中注册。
- 在 `src/main.jsx` 增加一次非阻塞预热调用。
- 复用已存在的 LM Studio URL 和认证辅助函数。
- 不修改 Ollama 预热行为、LM Studio 配置界面或 Windows 开机启动设置。
