# 仓库指导说明

## 项目结构及模块组织

以下是使用Tauri开发的桌面应用程序。前端使用了Vite和React技术构建，后端则采用了Rust语言。前端代码可以在`src/`目录中找到：可重用的UI组件位于`src/components`下，React钩子在`src/hooks`，国际化文件在`src/i18n/locales`，集成逻辑在`src/services`，而窗口级别的视图则在`src/window`。源代码命令和针对不同平台的行为存储在`src-tauri/src`目录中。静态运行时资源存放在`public`目录下，README中提到的图片则保留在`asset`文件夹内。更新/发布工具可以在`updater`目录找到，设计或实现说明则保留在`docs`目录内。

## Build, Test, and Development Commands

Use `pnpm` because the repository includes `pnpm-lock.yaml`.

- `pnpm install`: 安装JavaScript依赖项。
- `pnpm dev`: 启动Vite前端开发服务器。
- `pnpm tauri dev`: 在本地运行全屏桌面应用程序。
- `pnpm build`:将前端打包结果构建到 `dist`。
- `pnpm tauri build`: 创作一个包装好的桌面艺术品。
- `cd src-tauri && cargo check`: 快速验证Rust代码。
- `cd src-tauri && cargo test`: 当存在时运行Rust测试。

本地代理壳应将命令前缀为`rtk`，例如`rtk pnpm build`。

## 编程风格和命名约定

JavaScript and JSX use Prettier from `.prettierrc.json`: 4-space indentation, single quotes, semicolons, `printWidth` 120, trailing commas where valid in ES5, and LF line endings. Keep React components in PascalCase and hooks as `useSomething`. Service provider folders under `src/services/{translate,recognize,tts,collection}` use lowercase provider names matching existing patterns. Rust modules use snake_case file names and standard Rust naming conventions.

## Testing Guidelines

There is no dedicated frontend test script in `package.json`; use `pnpm build` as the baseline frontend verification. For backend changes, run `cargo check` and add or run `cargo test` where logic is testable. If a change touches app behavior, verify it through `pnpm tauri dev` and describe the tested platform and flow.

## Commit & Pull Request Guidelines

Recent history uses Conventional Commit-style prefixes such as `feat:`, `fix:`, and `debug:`. Keep commits focused and imperative, for example `fix: prevent translate window during inline translation`. Pull requests should include a short summary, linked issue or motivation, verification commands, affected platforms, and screenshots or recordings for visible UI changes.

## Security & Configuration Tips

Do not commit local secrets, API keys, generated packages, `node_modules`, `dist`, or `src-tauri/target`. Keep provider credentials in user configuration paths handled by the app, not in source files.

## Agent-Specific Instructions

Respond to this repository's user in Chinese. Preserve unrelated local changes, and inspect `git status` before editing files.
