# LM Studio Chinese Default Prompts Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 LM Studio 新建服务实例的默认 system 和 user 提示词改为中文，同时保留用户已有的自定义提示词。

**Architecture:** 在 LM Studio 的纯 JavaScript 辅助模块中导出默认提示词常量，使 Node 测试能够直接验证真实默认值；配置页面引用该常量，不增加迁移逻辑，因此已保存的 `promptList` 不会被覆盖。现有请求适配层继续移除旧提示词模板中的换行符。

**Tech Stack:** React、JavaScript、Node `node:test`、Vite、pnpm

---

### Task 1: 中文化 LM Studio 默认提示词

**Files:**
- Modify: `src/services/translate/lmstudio/api.test.js`
- Modify: `src/services/translate/lmstudio/api.js`
- Modify: `src/services/translate/lmstudio/Config.jsx`

- [ ] **Step 1: 写入失败测试**

在 `src/services/translate/lmstudio/api.test.js` 的 `./api.js` 导入列表中加入 `LM_STUDIO_DEFAULT_PROMPT_LIST`，并追加：

```js
test('uses Chinese default prompts for new LM Studio services', () => {
    assert.deepEqual(LM_STUDIO_DEFAULT_PROMPT_LIST, [
        { role: 'system', content: '你是专业翻译机器人，只输出准确、自然的译文，不要解释。' },
        { role: 'user', content: '把以下内容从 $from 翻译成 $to：$text' },
    ]);
    assert.ok(LM_STUDIO_DEFAULT_PROMPT_LIST.every(({ content }) => !/[\r\n]/.test(content)));
});
```

- [ ] **Step 2: 运行测试并确认失败原因**

Run: `rtk node --test src/services/translate/lmstudio/api.test.js`

Expected: FAIL，提示 `./api.js` 尚未导出 `LM_STUDIO_DEFAULT_PROMPT_LIST`。

- [ ] **Step 3: 添加最小实现**

在 `src/services/translate/lmstudio/api.js` 顶部添加：

```js
export const LM_STUDIO_DEFAULT_PROMPT_LIST = [
    { role: 'system', content: '你是专业翻译机器人，只输出准确、自然的译文，不要解释。' },
    { role: 'user', content: '把以下内容从 $from 翻译成 $to：$text' },
];
```

在 `src/services/translate/lmstudio/Config.jsx` 中：

1. 将辅助模块导入改为：

```js
import {
    createLmStudioHeaders,
    getModelsUrl,
    LM_STUDIO_DEFAULT_PROMPT_LIST,
    parseModelIds,
} from './api';
```

2. 删除文件内现有英文 `defaultPromptList` 常量。
3. 将 `useConfig` 默认配置中的 `promptList` 改为：

```js
promptList: LM_STUDIO_DEFAULT_PROMPT_LIST,
```

不添加配置迁移或覆盖逻辑；`useConfig` 已有持久化配置继续优先于默认值。

- [ ] **Step 4: 运行定向测试并确认通过**

Run: `rtk node --test src/services/translate/lmstudio/api.test.js`

Expected: 该文件全部测试 PASS，包括 `uses Chinese default prompts for new LM Studio services`。

- [ ] **Step 5: 运行完整验证**

Run: `rtk pnpm test`

Expected: 全部测试 PASS，0 fail。

Run: `rtk pnpm build`

Expected: Vite 构建成功；允许仓库既有的 Browserslist、`eval`、动态导入和 chunk size 警告，不允许出现新增错误。

Run: `rtk pnpm exec prettier --check src/services/translate/lmstudio/api.js src/services/translate/lmstudio/Config.jsx`

Expected: 两个生产文件符合 Prettier 格式。

Run: `rtk git diff --check`

Expected: 无空白错误。

- [ ] **Step 6: 提交实现**

```bash
rtk git add src/services/translate/lmstudio/api.js src/services/translate/lmstudio/api.test.js src/services/translate/lmstudio/Config.jsx
rtk git commit -m "feat: use Chinese LM Studio prompts"
```
