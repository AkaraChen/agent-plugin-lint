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
AP_LINT_CORPUS=/path/to/eric-way/plugins python3 scripts/review/cli.py full
```

最后一条要求已准备好 PROVENANCE.md 中的历史语料及子模块；脚本不会下载或修改语料。未提供环境变量就失败，不把未执行当通过。MCP 脚本只验证静态配置，不启动服务或连接端点。

实际执行结果和退回历史见根目录 REVIEW.md。
