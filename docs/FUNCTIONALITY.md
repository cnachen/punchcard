> 仔细思考下文给出的工作流，考虑真实场景，按需设计一些没有给出的命令行参数。
> 命令行的参数要具有一致性，用户能够轻松使用。
> 错误提示要优雅，人类可读，你写的代码中，必要的部分要写注释，pub的项目要写文档注释。

---

# 一、应具备的功能（按真实工作流分层）

## 1) 介质与编码

* **80 列卡片模型**：每张卡固定 80 列；支持**卡片注释/颜色/分隔卡**元数据（逻辑分组、装订点位等）。
* **编码**：支持 **Hollerith**（12/11/0–9 区域打孔）、**EBCDIC**、**ASCII** 映射；支持**正负号 overpunch**、区域孔（zone punch）、多孔异常。
* **列保护**：可对特定列加“**保护/常量**”（如 73–80 列序号区）以防误改。
* **列模板**：FORTRAN、COBOL、JCL 常用模板（如 FORTRAN 的 col 1 注释、6 续行、7–72 语句、73–80 序号）。

## 2) 键盘打孔与校对（Keypunch / Verify）

* **打孔模式**：逐行键入 -> 形成卡片；支持**退格、删除（补丁卡）**、**跳列**、**自动填充**（空格或模板字符）。
* **校对模式（Verify Pass）**：以第二遍输入与首遍打孔比对，发现差异高亮；可设**强制校对**策略。
* **容错/故障注入**：可配置随机**重孔/漏孔/串键**概率，复刻机械误差。

## 3) 读卡、解释与打印（Reader / Interpreter）

* **虚拟读卡机**：模拟 2540/3505 之类速度参数（卡/分钟），可**顺序读**、**跳读**、**回读**。
* **解释器（Interpreter）**：把卡面内容**打印在卡顶端**（历史真机会在卡缘打字），便于人工检查；也可生成**可视化卡面**（文本或图像）。
* **列表与转储**：输出十六进制/打孔位图/编码字符三视图。

## 4) 组装与管理（Deck Management）

* **卡片盒（Deck）操作**：新建、插入、删除、复制、分割、合并、洗牌（重排）、抽取某范围。
* **序号管理**：在 73–80 列**打序号**、**重编号**、**按序号归并**（掉落卡可恢复顺序）。
* **分段**：按“分隔卡”“程序/数据/JCL 类别卡”管理。
* **批量导入/导出**：从 80 列文本、CSV、JSON、制卡脚本导入；导出为**纯文本**、**.deck（JSONL）**、**PUNCH（位图）**、**图像（PNG/SVG）**、**打印清单**。

## 5) 语种与特殊卡支持

* **语言模板**：FORTRAN IV、COBOL、Assembler（H）、JCL 卡片类型。
* **续行/区域规则**：自动检查列规则并给出**版式告警**。
* **JCL 卡检查**：// 开头、作业名、CLASS、MSGCLASS 等字段对齐提示。

## 6) 审计与可重复性

* **作业日志**：记录谁在何时以何命令修改了哪些卡（增删改）。
* **校验与签名**：对 deck 生成**哈希**；支持**只读锁**。
* **时间旅行**：基于操作日志回滚到任意版本。

---

# 二、命令行接口设计（示例：`punch` 主命令 + 子命令）

> 约定：
>
> * 资源：**deck 文件**（.deck = JSONL，或 .cards = 80 列文本）
> * `CARDSET` 表示一个 deck 路径或目录
> * `--encoding` 默认为 `hollerith`，可选 `ebcdic` / `ascii`
> * 带 `@` 的参数表示从文件或 stdin 读取

```text
punch
├── deck        # 卡片盒生命周期与集合操作
├── card        # 单卡操作（查看、打孔、替换、解释）
├── encode      # 字符串 <-> 打孔位 映射工具
├── verify      # 校对流程
├── seq         # 序号相关（打序号/重排/恢复）
├── render      # 可视化与打印清单
├── jcl         # 针对 JCL 的检查/生成功能
├── template    # 语言模板与列规则
└── audit       # 审计与哈希
```

## 1) `deck`（创建/导入/导出/合并）

* 新建空 deck（FORTRAN 模板 + 序号保护）
  `punch deck init prog.deck --language fortran --protect 73-80`
* 从 80 列文本导入为 deck
  `punch deck import prog.cards --out prog.deck --encoding ascii`
* 合并多个 deck
  `punch deck merge a.deck b.deck --out ab.deck`
* 抽取范围/类型
  `punch deck slice prog.deck --range 1..10,25,40..$ --out part.deck`
* 导出为可打印文本
  `punch deck export prog.deck --format text80 --out prog.cards`

常用选项：

* `--encoding [hollerith|ebcdic|ascii]`
* `--readonly`（导入时给 deck 加锁）

## 2) `card`（单卡打孔/查看/补丁/解释）

* 新增一张卡（按模板自动对齐）
  `punch card add prog.deck --text "      PROGRAM HELLO" --template fortran`
* 交互式打孔（逐列输入；支持退格/跳列）
  `punch card type prog.deck`
* 替换第 12 张卡
  `punch card replace prog.deck --index 12 --from @line.txt`
* 查看与解释（显示字符+打孔位+顶部印字预览）
  `punch card show prog.deck --index 12 --interpret`
* 打补丁卡（不改原卡，生成“更正卡”记录）
  `punch card patch prog.deck --index 12 --cols 7-20="CONTINUE"`

选项：

* `--protect 73-80`（此操作不改保护列）
* `--allow-multipunch`（允许多孔）

## 3) `encode`（编码/位图工具）

* 文本转打孔位（可选显示 12/11/0-9）
  `punch encode text "ABC-123" --to punches`
* 打孔位转字符
  `punch encode punches @bits.txt --to text --encoding ebcdic`

## 4) `verify`（校对）

* 准备校对影子副本
  `punch verify start prog.deck`
* 第二遍输入与比对（从 stdin 读 80 列文本流）
  `punch verify pass prog.deck --from @retype.cards`
* 查看差异报告
  `punch verify report prog.deck --format unified`

选项：

* `--strict`（有差异即失败，阻止后续导出）
* `--mask 73-80`（忽略序号列差异）

## 5) `seq`（序号与顺序恢复）

* 在 73–80 列打序号（步长 10）
  `punch seq number prog.deck --range 1..$ --start 10 --step 10`
* 按 73–80 列序号排序（掉卡恢复）
  `punch seq sort prog.deck`
* 重编号
  `punch seq renumber prog.deck --start 1000 --step 5`

## 6) `render`（渲染与打印）

* 生成卡面PNG（含打孔、印字、颜色）
  `punch render image prog.deck --out imgs/ --dpi 300`
* 生成打印清单（字符视图 + 位图视图）
  `punch render listing prog.deck --out listing.txt`
* 生成“解释器”样式顶端印字文本
  `punch render interpret prog.deck --out interp.cards`

选项：

* `--style [plain|interpreter|keypunch]`
* `--pagesize A4`（清单分页）

## 7) `jcl`（JCL 专用）

* 检查 JCL 语法/列规/字段对齐
  `punch jcl lint job.deck`
* 生成常用 JCL 框架
  `punch jcl init --jobname HELLO --class A --msgclass X --out job.deck`
* 合并 JCL + 程序 deck 形成作业包
  `punch jcl bundle job.deck prog.deck --out submit.deck`

## 8) `template`

* 列出模板
  `punch template list`
* 查看模板列规（例如 FORTRAN）
  `punch template show fortran`
* 自定义模板导入/导出
  `punch template import mytpl.yaml`

## 9) `audit`

* 生成 deck 哈希与签名
  `punch audit hash prog.deck`
* 查看操作日志/回滚
  `punch audit log prog.deck`
  `punch audit revert prog.deck --to 2025-10-29T10:15:00`

---

# 三、文件与数据格式建议

* **`.deck`（JSONL）**：一行一张卡，含：

  * `text`（80 字符，或 `null` + `punches` 位图）
  * `encoding`、`protected_cols`、`type`（code/data/jcl/separator）
  * `seq`（73–80 列当前序号）、`meta`（颜色、批注）
  * `audit`（作者、时间、命令）
* **`.cards`**：80 列定宽纯文本（便于 diff）
* **渲染输出**：`PNG/SVG`；清单 `txt/pdf`

---

# 四、典型工作流示例

```bash
# 1) 初始化 FORTRAN deck，保护 73-80 列
punch deck init hello.deck --language fortran --protect 73-80

# 2) 逐卡打孔（按模板自动对齐）
punch card type hello.deck    # 交互式，输入若干行，结束用 Ctrl+D

# 3) 给所有卡打序号/重编号
punch seq number hello.deck --start 10 --step 10

# 4) 二次校对
punch verify start  hello.deck
punch verify pass   hello.deck --from @hello_retype.cards
punch verify report hello.deck

# 5) 生成“解释器”风格顶端印字与PNG卡面
punch render interpret hello.deck --out hello.interp.cards
punch render image     hello.deck --out imgs/

# 6) 与 JCL 捆绑为作业包
punch jcl init --jobname HELLO --out job.deck
punch jcl bundle job.deck hello.deck --out submit.deck

# 7) 导出 80 列文本备份
punch deck export submit.deck --format text80 --out submit.cards
```

---

# 五、实现要点/细节建议

* **编码表**：内置 Hollerith ↔ 字符映射表（含 zone punch、overpunch），可覆盖微妙字符差异。
* **版式校验**：可插拔“规则引擎”，对各语言列位进行 Lint。
* **可测试性**：对每个子命令提供“干跑（--dry-run）”和确定性随机种子。
* **英语**：CLI 文案及注释用英语；代码中的命名要有一致性；错误信息包含列号与可视化卡面片段。
* **速度与批量**：所有子命令支持 `stdin/stdout` 管线；`punch deck import -` 兼容流式处理。

---
