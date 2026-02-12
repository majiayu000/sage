# Go NEVER 规则

项目标识: `go.mod` 存在
文件扩展名: `*.go`
Lint 命令: `go vet ./...` + `golangci-lint run`
测试命令: `go test ./...`
格式化命令: `gofmt -l .`

## 规则

- **GO-01**: NEVER ignore returned errors — `_ = f()` must handle error
  - 检测模式: `"_ ="` 在 err 返回的函数调用上 (排除 test/vendor)

- **GO-02**: NEVER use `interface{}` where typed interface suffices — use generics or concrete interface
  - 检测模式: `"interface\{\}"` (排除 test/vendor)

- **GO-03**: NEVER launch goroutine without lifecycle management — use context + WaitGroup/errgroup
  - 检测模式: `"go func"` 无 context 参数或 WaitGroup

- **GO-04**: NEVER use `panic()` in library code — return error instead
  - 检测模式: `"panic("` (排除 test)

- **GO-05**: NEVER use `init()` for complex logic — explicit initialization preferred
  - 检测模式: `"func init()"` 超过 5 行

- **GO-06**: NEVER use global mutable state — pass dependencies explicitly
  - 检测模式: `"var .* =.*sync.Mutex\|var .* =.*map\["` 在包级别
