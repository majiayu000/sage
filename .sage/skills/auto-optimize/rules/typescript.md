# TypeScript/JavaScript NEVER 规则

项目标识: `package.json` 存在
文件扩展名: `*.ts`, `*.tsx`, `*.js`, `*.jsx`
Lint 命令: `npx eslint .`
测试命令: `npx vitest --run` 或 `npx jest`
格式化命令: `npx prettier --check .`
类型检查: `npx tsc --noEmit`

## 规则

- **TS-01**: NEVER use `any` type — use `unknown` + type guard
  - 检测模式: `": any\| any[\|any,"` (排除 test/d.ts)

- **TS-02**: NEVER use `==` for comparison — use `===`
  - 检测: eslint eqeqeq rule

- **TS-03**: NEVER use `innerHTML` with user input — XSS risk
  - 检测模式: `"innerHTML"` (排除 test)

- **TS-04**: NEVER disable ESLint rules inline without justification comment
  - 检测模式: `"eslint-disable"` 无后续注释说明原因

- **TS-05**: NEVER use synchronous I/O in async context — blocks event loop
  - 检测模式: `"readFileSync\|writeFileSync\|execSync"` 在非脚本文件中

- **TS-06**: NEVER use `console.log` in production code — use structured logger
  - 检测模式: `"console.log\|console.error\|console.warn"` (排除 test/script)

- **TS-07**: NEVER store secrets in client-side code — no API keys in frontend bundles
  - 检测模式: `"API_KEY\|SECRET\|PASSWORD"` 在 src/ 非 server 目录

- **TS-08**: NEVER use `new Function()` or `eval()` — code injection risk
  - 检测模式: `"new Function(\|eval("` (排除 test)
