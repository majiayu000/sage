# Python NEVER 规则

项目标识: `pyproject.toml` 或 `setup.py` 或 `requirements.txt` 存在
文件扩展名: `*.py`
Lint 命令: `ruff check .` 或 `flake8 .`
测试命令: `pytest --tb=short`
格式化命令: `ruff format --check .` 或 `black --check .`
类型检查: `mypy .` 或 `pyright .`

## 规则

- **PY-01**: NEVER use bare `except:` or `except Exception:` — catch specific exceptions
  - 检测模式: `"except:\s*$\|except Exception:"` (排除 test)

- **PY-02**: NEVER use mutable default arguments — `def f(x=[])` is a shared-state bug
  - 检测模式: `"def.*=\[\]\|def.*=\{\}"` 在函数签名中

- **PY-03**: NEVER use `eval()`/`exec()` on user input — code injection risk
  - 检测模式: `"eval(\|exec("` (排除 test)

- **PY-04**: NEVER use `import *` — namespace pollution, hidden dependencies
  - 检测模式: `"from .* import \*"`

- **PY-05**: NEVER concatenate SQL strings — use parameterized queries
  - 检测模式: `f".*SELECT\|".*SELECT.*" +\|'.*SELECT.*' +`

- **PY-06**: NEVER ignore type hints in public APIs — all public functions need annotations
  - 检测: mypy/pyright strict mode

- **PY-07**: NEVER use `os.system()` — use `subprocess.run()` with shell=False
  - 检测模式: `"os.system("` (排除 test)

- **PY-08**: NEVER use `pickle.load()` on untrusted data — deserialization attack
  - 检测模式: `"pickle.load\|pickle.loads"` 追踪数据来源
