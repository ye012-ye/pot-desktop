# LM Studio 翻译服务设计

## 背景与目标

Pot 已有 OpenAI 兼容的聊天补全调用能力，但 LM Studio 只能通过通用 OpenAI 服务间接配置。此次改动新增独立的“LM Studio”翻译服务入口，让用户能直接连接 LM Studio、本地发现模型并完成翻译。

成功标准：

- 用户能从翻译服务列表添加 LM Studio 实例。
- 配置页默认连接 `http://localhost:1234/v1`。
- 配置页能读取 `/v1/models`，同时允许手动填写模型标识。
- 用户能保存可选的 API Token，并通过 `/v1/chat/completions` 完成测试翻译。
- 流式输出、Prompt List 和附加请求参数与现有 OpenAI 服务保持一致。

## 已确认方案

采用独立入口加薄适配方案。LM Studio 拥有自己的服务注册信息和配置界面，翻译请求复用现有 OpenAI 兼容调用链，不复制完整请求实现，也不重构无关服务。

未采用的方案：

- 抽取全新的通用 OpenAI 兼容客户端：边界更整齐，但会扩大现有 OpenAI 服务的改动范围。
- 复制 OpenAI 服务实现：初始实现直接，但会产生两套流式解析和错误处理逻辑。

## 组件与职责

### LM Studio 服务模块

在 `src/services/translate/lmstudio/` 下新增服务模块：

- `info.ts`：声明服务名、图标和语言映射。
- `Config.jsx`：管理 Base URL、模型、API Token、流式开关、Prompt List 和请求参数。
- `index.jsx`：把 LM Studio 配置转换为现有 OpenAI 兼容调用所需的配置，并委托现有翻译函数执行。
- 一个纯 JavaScript 辅助模块：负责 Base URL 规范化、端点拼接、认证头构造和模型响应解析，便于使用 Node 内置测试框架测试。

### 服务注册与可用性

- 在前端翻译服务导出表中注册 `lmstudio`。
- 在 Rust 内置翻译服务白名单中加入 `lmstudio`，避免已保存实例在启动检查时被清理。
- 增加中文和英文文案；其他语言使用现有 i18n 英文回退机制。
- 增加独立的 LM Studio 图标资源，不复用 OpenAI 品牌图标。

## 配置与数据流

配置结构包含：

- `baseUrl`：默认 `http://localhost:1234/v1`。
- `model`：模型标识，可从列表选择或手动输入。
- `apiKey`：可选 LM Studio API Token，默认空字符串。
- `stream`：是否启用流式输出。
- `promptList`：翻译提示词列表。
- `requestArguments`：传递给聊天补全接口的附加 JSON 参数。

打开配置页时：

1. 规范化 Base URL。
2. 请求 `GET {baseUrl}/models`。
3. API Token 非空时添加 `Authorization: Bearer <token>`。
4. 将响应中的模型标识展示为可选项。
5. 请求失败时保留手动模型输入，并展示非阻塞错误提示。

执行测试或正式翻译时：

1. 将 Base URL 转换为 `{baseUrl}/chat/completions`。
2. 把 LM Studio 配置适配为现有 OpenAI 兼容配置。
3. 委托现有翻译函数处理 Prompt 替换、请求参数、流式解析和结果返回。
4. API Token 为空时不发送 Authorization 请求头；非空时发送 Bearer Token。

## URL 处理规则

- 接受 `http://localhost:1234`、`http://localhost:1234/` 和 `http://localhost:1234/v1`。
- 统一得到不带末尾斜杠的 API Base URL，并确保只包含一个 `/v1`。
- 如果用户输入完整的 `/v1/chat/completions` 地址，先还原为 Base URL，再生成模型和聊天补全端点。
- 不静默把 `http` 改成 `https`；本地 LM Studio 默认使用 HTTP。

## 错误处理

- LM Studio 未启动、地址不可达或 `/v1/models` 返回异常：显示模型读取失败提示，不清空用户已填写的模型。
- 未选择或填写模型：测试保存时提示用户填写模型，不发送聊天请求。
- Token 无效或接口返回非成功状态：沿用现有服务测试失败提示并保留服务配置。
- 模型响应结构异常：视为模型发现失败，继续允许手动输入。

本次不负责启动 LM Studio，不自动加载或下载模型，也不修改 LM Studio 服务端设置。

## 测试与验证

采用测试先行：

- URL 规范化及 `/models`、`/chat/completions` 拼接。
- 空 Token 不生成认证头，非空 Token 生成 Bearer 认证头。
- `/v1/models` 标准响应能解析为模型标识列表，异常结构被拒绝。

仓库当前没有前端测试脚本，因此使用 Node 内置 `node:test` 运行纯函数测试，并在 `package.json` 增加最小测试命令。实现完成后运行：

- `rtk pnpm test`
- `rtk pnpm build`
- `rtk cargo check`（在 `src-tauri` 下）

如本机 LM Studio 正在运行，再进行一次手动验证：添加服务、读取模型、测试翻译并确认流式和非流式路径至少各成功一次。若本机服务不可用，应明确记录该项未执行，不以构建结果替代真实接口验证。

## 改动边界

- 只修改 LM Studio 服务、必要的服务注册、认证头兼容、双语文案、图标和测试入口。
- 不重构现有 OpenAI 配置页或其他翻译服务。
- 不修改 Ollama 自动启动与预热逻辑。
- 不提交构建产物、依赖目录或本地服务凭据。

## 中文默认提示词补充设计

LM Studio 新建服务实例使用以下中文默认提示词：

- `system`：`你是专业翻译机器人，只输出准确、自然的译文，不要解释。`
- `user`：`把以下内容从 $from 翻译成 $to：$text`

本次只修改 LM Studio 的默认提示词，不修改 OpenAI、Ollama 等其他翻译服务，也不覆盖用户已经保存的 `promptList` 自定义配置。默认提示词放在可由 Node 测试直接导入的纯 JavaScript 模块中，配置页面引用同一份常量，避免测试与实际默认值脱节。回归测试验证两个默认消息均为中文且不包含换行符；现有 LM Studio 请求适配层继续负责清理旧配置中的换行符。
