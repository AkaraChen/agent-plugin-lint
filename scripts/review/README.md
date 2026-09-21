# astra 独立黑盒验收

这些脚本由设计/review 方编写，调用真实 `ap-lint` 进程，只在临时目录创建夹具。当前只在 Linux 验证；符号链接、chmod 和 FIFO 测试不能证明其他平台行为。无第三方 Python 依赖。

在仓库根执行：

```sh
cargo build -p agent-plugin-lint
python3 scripts/review/cli.py
python3 scripts/review/filesystem.py full
python3 scripts/review/s2b.py
python3 scripts/review/skills.py
python3 scripts/review/mcp.py
python3 scripts/review/json_edges.py
python3 scripts/review/extensions.py
python3 scripts/review/coverage.py
python3 scripts/review/coverage_edges.py
AP_LINT_CORPUS=/path/to/eric-way/plugins python3 scripts/review/cli.py full
AP_LINT_CORPUS=/path/to/eric-way/plugins python3 scripts/review/runtime.py
```

完整 cli 验收要求已准备好 research/PROVENANCE.md 中的历史语料及子模块；未提供环境变量就失败。runtime 验收的语料项仅在提供环境变量时执行。脚本不会下载或修改语料，不把未执行当通过。MCP 脚本只验证静态配置，不启动服务或连接端点。

实际执行结果和退回历史见根目录 REVIEW.md。

`runtime.py` 额外需要 Linux 的 `strace`，把二进制复制到临时目录后运行，观察网络系统调用并确认不会执行被检命令；它不创建网络命名空间，也不能证明所有未覆盖输入的行为。
