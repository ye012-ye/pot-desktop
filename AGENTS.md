# Repository guidance instruction

## Project Structure & Module Organization

This repository is a Tauri desktop app with a Vite/React frontend and a Rust backend. Frontend code lives in `src/`: reusable UI in `src/components`, React hooks in `src/hooks`, localization files in `src/i18n/locales`, integration logic in `src/services`, and window-level views in `src/window`. Native commands and platform behavior live in `src-tauri/src`. Static runtime assets are in `public`, README images are in `asset`, release/update utilities are in `updater`, and design or implementation notes are in `docs`.

## Build, Test, and Development Commands

Use `pnpm` because the repository includes `pnpm-lock.yaml`.

- `pnpm install`: install JavaScript dependencies.
- `pnpm dev`: start the Vite frontend dev server.
- `pnpm tauri dev`: run the full desktop app locally.
- `pnpm build`: build the frontend bundle into `dist`.
- `pnpm tauri build`: build packaged desktop artifacts.
- `cd src-tauri && cargo check`: validate Rust code quickly.
- `cd src-tauri && cargo test`: run Rust tests when present.

Local agent shells should prefix commands with `rtk`, for example `rtk pnpm build`.

## Coding Style & Naming Conventions

JavaScript and JSX use Prettier from `.prettierrc.json`: 4-space indentation, single quotes, semicolons, `printWidth` 120, trailing commas where valid in ES5, and LF line endings. Keep React components in PascalCase and hooks as `useSomething`. Service provider folders under `src/services/{translate,recognize,tts,collection}` use lowercase provider names matching existing patterns. Rust modules use snake_case file names and standard Rust naming conventions.

## Testing Guidelines

There is no dedicated frontend test script in `package.json`; use `pnpm build` as the baseline frontend verification. For backend changes, run `cargo check` and add or run `cargo test` where logic is testable. If a change touches app behavior, verify it through `pnpm tauri dev` and describe the tested platform and flow.

## Commit & Pull Request Guidelines

Recent history uses Conventional Commit-style prefixes such as `feat:`, `fix:`, and `debug:`. Keep commits focused and imperative, for example `fix: prevent translate window during inline translation`. Pull requests should include a short summary, linked issue or motivation, verification commands, affected platforms, and screenshots or recordings for visible UI changes.

## Security & Configuration Tips

Do not commit local secrets, API keys, generated packages, `node_modules`, `dist`, or `src-tauri/target`. Keep provider credentials in user configuration paths handled by the app, not in source files.

## Agent-Specific Instructions

Respond to this repository's user in Chinese. Preserve unrelated local changes, and inspect `git status` before editing files.
