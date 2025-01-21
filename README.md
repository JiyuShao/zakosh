# Zako Shell

雑魚（ざこ）Shell，具有指令纠错效果，并自带嘲讽技能。使用 Rust 编写。

## 功能

- 嘲讽功能
- 交互式命令行界面

## 使用方法

1. 构建项目：

   ```bash
   cargo build --release
   ```

2. 运行 shell：

   ```bash
   cargo run
   ```

3. 校验项目

   ```bash
   cargo clippy
   ```

4. 修复项目 & 格式化项目

   ```bash
   cargo fix
   cargo clippy --fix
   cargo fmt
   ```

## WIP

- [x] 嘲讽功能
- [ ] 支持解析器与执行器
  - [ ] 支持解析器（Parser）：解析 shell 脚本语法，将其转换为抽象语法树（AST）
  - [ ] 支持执行器（Executor）：根据 AST 执行命令和逻辑
- [ ] 支持环境管理
  - [x] 支持管理环境变量
  - [ ] 支持管理全局变量
- [ ] 支持可交互命令行
  - [x] 支持命令行编辑
  - [x] 支持管道
  - [ ] 支持多行命令
  - [ ] 支持方法
  - [ ] 支持流程控制
  - [ ] 支持作业控制
- [x] 支持外部命令：调用外部程序，通过 sh -c 实现
- [ ] 支持内置命令
  - [ ] 通过 sh -c 实现
    - [x] cat
    - [x] clear
    - [x] cp
    - [x] export
    - [x] env
    - [x] echo 环境变量能力
    - [x] git
    - [x] grep
    - [x] head
    - [x] kill
    - [x] less
    - [x] vim
    - [x] mkdir
    - [x] mv
    - [x] rm
    - [x] sort
    - [x] source
    - [ ] tail
    - [ ] touch
    - [ ] which
    - [ ] 以及其他在 bin 目录里的命令
  - [ ] 通过 rust 实现
    - [x] zako: 开启一个新的 Zako Shell
    - [x] exit
    - [ ] cd
    - [ ] alias
    - [ ] unalias
    - [ ] history
    - [ ] echo
    - [ ] unset: 删除变量和函数
