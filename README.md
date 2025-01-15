# Zago Shell

雑魚（ざこ）Shell，具有指令纠错效果，并自带嘲讽技能。使用 Rust 编写。

## 功能

- 交互式命令行界面
- 命令历史记录
- 支持执行系统命令
- 使用 'exit' 命令退出

## 使用方法

1. 构建项目：

   ```bash
   cargo build --release
   ```

2. 运行 shell：

   ```bash
   cargo run
   ```

## 命令

- `exit`: 退出 shell
- 其他所有命令都会被传递给系统执行
