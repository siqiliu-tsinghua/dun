# 真终端 TUI 测试方案（tmux + vt100）

面向：负责落地测试基础设施的 agent。

目标：在**真实终端**中自动化验证一个基于 ratatui 的 TUI 编辑器（macOS，Terminal.app / iTerm2）的渲染结果——UI 位置、尺寸、配色、键盘交互——并与 `msedit`（不支持 PTY、启动即渲染）做 diff test。

已具备：基于 PTY 的测试（`portable-pty` + `vt100`），主要覆盖交互逻辑与鼠标 SGR 序列解析。本方案**不替代**它，而是补上"真终端 + 布局/配色/跨程序 diff"这一层。

---

## 0. 核心思路

真终端里需要的是屏幕上每个 cell 的 `(字符, 前景色, 背景色, 属性)`。抓取这个状态**不用 PTY**，而是把被测程序跑在 **tmux pane** 里——tmux 自己就是一个终端仿真器，因此对被测程序**是否支持 PTY 协议没有要求**（这正是 `msedit` 能用的原因）。

流程：

1. `tmux new-session` 以**固定尺寸**起 pane，在其中运行被测二进制。
2. `tmux send-keys` 注入键盘序列。
3. `tmux capture-pane -pe` 抓取**带颜色/属性**的屏幕快照。
4. 把快照当成一段终端流喂给 `vt100::Parser`，读出规范化网格。
5. 断言 / diff。

关键收益：PTY 测试与真终端测试**复用同一套 `vt100` 解析与断言原语**，diff test 两端也统一。

---

## 1. 依赖与前置

- `tmux`（`brew install tmux`）。CI runner 同样安装即可，Linux runner 也能跑（不需要真实 GUI 终端）。
- Rust 侧：复用现有 `vt100` crate 作为解析器；用 `std::process::Command` 调 tmux，或 `portable-pty` 起 tmux 亦可（起 tmux 本身不需要，普通 `Command` 足够）。
- 被测二进制与 `msedit` 二进制路径通过环境变量或测试常量注入。

约定：**所有测试固定 pane 尺寸**（如 `100x30`），尺寸是断言的一部分，绝不依赖当前窗口大小。

---

## 2. tmux 抓屏原语

命令速查：

- 起 session（后台、指定尺寸）：
  `tmux new-session -d -s <name> -x <cols> -y <rows> <bin> [args...]`
- 注入按键：
  `tmux send-keys -t <name> <keys...>`
  - 字面字符直接给；特殊键用名字：`Escape` `Enter` `Up` `Down` `Left` `Right` `C-c`（Ctrl-C）`S-Left`（Shift-Left）等。
  - 发十六进制原始字节：`tmux send-keys -t <name> -H 1b 5b ...`（用于注入无法用键名表达的原始序列）。
- 抓带色快照：
  `tmux capture-pane -t <name> -pe`
  - `-p` 输出到 stdout；`-e` **保留** SGR 转义序列（颜色 + 属性）。**必须带 `-e`，否则拿不到配色。**
- 抓指定行范围：`capture-pane -pe -S <start> -E <end>`（`-S -N` 可回溯历史，测正文一般用可见区即可）。
- 结束：`tmux kill-session -t <name>`。

**时序**：TUI 启动和每次按键后需要时间重绘。不要固定 `sleep`。做**轮询**：反复 `capture-pane` 直到屏幕稳定（连续两次快照相同）或出现预期锚点文本，设超时上限。把"等待稳定"封装成 harness 里的一个函数。

---

## 3. 解析：capture-pane 输出 → 规范化网格

把 `capture-pane -pe` 的字节流直接喂给 `vt100::Parser`（`parser.process(&bytes)`），然后遍历：

```
for row in 0..rows {
  for col in 0..cols {
    let cell = screen.cell(row, col);
    // cell.contents(): String（字符）
    // cell.fgcolor(): vt100::Color（Default / Idx(u8) / Rgb(r,g,b)）
    // cell.bgcolor()
    // cell.bold() / cell.inverse() / cell.underline() 等属性
  }
}
```

产出统一结构（PTY 测试、真终端测试、diff test 三处共用）：

```rust
struct Cell { ch: char, fg: Color, bg: Color, bold: bool, inverse: bool, underline: bool }
type Grid = Vec<Vec<Cell>>;
```

规范化规则（**在比较前统一施加，减少无意义 diff**）：

- 空 cell 的字符统一成空格。
- 颜色只保留**语义层**：`Color::Idx` 或 `Color::Rgb` 原样保留（这是程序请求的色）；不要在这一层把 palette 索引映射成具体 RGB（那是终端主题的事，见 §6）。
- 尾随空格按需裁剪（做整行文本比较时）。

---

## 4. 断言：位置与尺寸

固定 pane 尺寸后，位置/尺寸断言 = "特定边框/内容字符落在预期行列"。

- **尺寸**：断言面板边框字符（`─ │ ┌ ┐ └ ┘ ├ ┤` 等）出现在特定行列；或分割线在第 N 列/行。
- **位置**：状态栏在最后一行、标题在第 0 行、某分割在 `col == width/2` 等——直接查对应行/列内容。
- **响应式**：用不同 `-x/-y` 起多个 session（如 `80x24` 与 `120x40`），断言布局随尺寸变化符合预期。

提供辅助断言：
- `assert_text_at(grid, row, col, "expected")`
- `find_border_box(grid) -> Rect`（扫描连续边框字符推断面板矩形，用于尺寸回归）
- `assert_line_contains(grid, row, substr)`

---

## 5. 断言：配色（先分清测哪一层）

**关键分叉，落地前必须明确：**

- **语义色**（程序请求的颜色，如关键字 `SetForegroundColor(Cyan)`）：`capture-pane -e` 抓到的 SGR 就是程序发出的，解析成 palette 索引或 truecolor RGB，**稳定、可断言、diff test 比这一层**。
- **实际像素色**（终端主题把 `Cyan` 映射成的具体 RGB）：取决于 Terminal.app / iTerm2 的配色方案，换主题就变。**这层不用本方案测**，它测的是终端配置而非程序正确性。只有截图能拿到（见 §8，仅作极少数视觉回归）。

因此配色断言只针对**程序发出的色**：
- `assert_fg(grid, row, col, Color::Idx(6))`（或对应 Rgb）
- 语法高亮：断言"某 token 的所有 cell 带某一类前景色 / 某属性"，**比对分类，不比对具体 RGB**（除非你的程序本就发 truecolor）。

---

## 6. 与 msedit 做 diff test（本方案重点）

`msedit` 不支持 PTY 但启动即渲染 —— 正好适合 tmux，因为 tmux 不要求被测程序懂 PTY。

**做法：**

1. **对齐输入**：同一 pane 尺寸、同一份输入文件、同一串 `send-keys` 序列，分别喂给你的编辑器和 `msedit`。
2. **各自抓屏**：`capture-pane -pe` → 解析成规范化 `Grid`。
3. **只在语义等价子集上 diff**（**不要全屏精确 diff**）。两个编辑器 UI 不可能逐 cell 相同——边框风格、状态栏文案、logo、快捷键提示都不同。要 diff 的是：
   - **正文区内容**：受控行列范围内，同样编辑操作后正文文本是否一致。
   - **光标位置**：行列是否一致（光标可从 `capture-pane` 的光标位置或反显 cell 推断；必要时通过程序状态旁路获取）。
   - **选区范围**：哪些 cell 带 `inverse`/选中背景 —— 比对**覆盖的 cell 集合**，不比对具体色值。
   - **语法高亮 token 边界**：同一个词两边是否都被着成"某一类"色 —— 比对**分类与边界**，不比对 RGB。

**diff test 的形状定义**：在**受控子区域**上，**规范化后的属性类别一致**。即：
- 定义一个 `Region { row_range, col_range }`（正文区）。
- 定义每个 cell 的**比较投影**，例如 `(ch, is_selected, token_class)`，刻意丢弃边框、丢弃具体 RGB、丢弃状态栏。
- 只 diff 投影后的子网格。

否则（像素级或全屏级）维护成本会爆炸。

**建议目录/结构**：
```
tests/difftest/
  cases/<case>/input.txt        # 输入文件
  cases/<case>/keys.txt         # send-keys 序列（每行一个 token）
  cases/<case>/region.toml      # 正文区行列范围 + 比较投影配置
  runner.rs                     # 跑双端、抓屏、投影、diff
```
每个 case 对两个二进制各跑一次，产出投影子网格，断言相等；不等时打印两侧网格的**并排文本 dump**（字符层）+ 差异 cell 列表，便于人工核对。

---

## 7. 鼠标：本方案不负责（明确边界）

`send-keys` 发键很顺，但**鼠标**在 tmux 里很别扭：tmux 自己也想吃鼠标事件，与程序的 DECSET 1000/1006 捕获会打架，常需直接往 pane 的 tty 写 SGR 鼠标序列或用 `send-keys -H` 发十六进制，脆且啰嗦。

**分工原则：**
- **鼠标类用例**（选中、右键粘贴等）→ 继续走已有的 **PTY 测试**，那里注入 SGR 鼠标最干净、可完整断言。
- **tmux 这套** → 专门负责**真终端的键盘流 + 布局 + 配色 + 与 msedit 的 diff**。

不要强行让 tmux 干鼠标。

补充坑（若确需在真终端测粘贴）：终端粘贴走 **bracketed paste**，内容被 `\x1b[200~ ... \x1b[201~` 包裹。程序需开 `EnableBracketedPaste`，注入时用 `send-keys -H` 发这段包裹序列。

---

## 8. 像素级视觉回归（可选，不进 CI 主流程）

只有真实字体渲染 + 真实主题配色需要验证时才用：AppleScript 截 Terminal.app 窗口存 PNG，再做像素比对。

**极脆**：字体、DPI、主题、抗锯齿全影响。仅建议作为**极少数**手动触发的视觉回归用例，**不纳入 CI**。绝大多数配色/布局诉求用 §4–§6 的语义层解决。

---

## 9. Harness 骨架（Rust）

```rust
use std::process::Command;

struct Tmux { name: String }

impl Tmux {
    fn start(name: &str, cols: u16, rows: u16, bin: &str, args: &[&str]) -> Self {
        let mut a = vec!["new-session","-d","-s",name,
                         "-x",&cols.to_string(),"-y",&rows.to_string(), bin];
        a.extend(args);
        Command::new("tmux").args(&a).status().unwrap();
        Tmux { name: name.into() }
    }
    fn send(&self, keys: &[&str]) {
        let mut a = vec!["send-keys","-t",&self.name];
        a.extend(keys);
        Command::new("tmux").args(&a).status().unwrap();
    }
    /// 抓带色快照原始字节
    fn capture(&self) -> Vec<u8> {
        Command::new("tmux")
            .args(["capture-pane","-t",&self.name,"-pe"])
            .output().unwrap().stdout
    }
    /// 轮询直到屏幕稳定（连续两次相同）或超时
    fn capture_stable(&self, timeout: std::time::Duration) -> Vec<u8> {
        let start = std::time::Instant::now();
        let mut last = self.capture();
        loop {
            std::thread::sleep(std::time::Duration::from_millis(50));
            let now = self.capture();
            if now == last || start.elapsed() > timeout { return now; }
            last = now;
        }
    }
}
impl Drop for Tmux {
    fn drop(&mut self) {
        let _ = Command::new("tmux").args(["kill-session","-t",&self.name]).status();
    }
}

/// 复用 PTY 测试里现成的 vt100 解析，产出规范化 Grid
fn parse(bytes: &[u8], cols: u16, rows: u16) -> Grid {
    let mut p = vt100::Parser::new(rows, cols, 0);
    p.process(bytes);
    let screen = p.screen();
    // 遍历 screen.cell(r,c) → Grid（见 §3）
    grid_from_screen(screen, cols, rows)
}

#[test]
fn layout_80x24() {
    let t = Tmux::start("layout80", 80, 24, env!("EDITOR_BIN"), &["input.txt"]);
    let g = parse(&t.capture_stable(std::time::Duration::from_secs(2)), 80, 24);
    assert_line_contains(&g, 23, "NORMAL");        // 状态栏在最后一行
    assert_border_box(&g, /* 预期矩形 */);         // 尺寸回归
    assert_fg(&g, /*row*/1, /*col*/2, vt100::Color::Idx(6)); // 关键字色（语义层）
}
```

diff test 版：起两个 `Tmux`（你的 bin 与 `msedit`），各自 `capture_stable` → `parse` → 施加**比较投影**取正文子网格 → `assert_eq!`。

---

## 10. 落地检查清单

- [ ] harness：`Tmux` 封装 + `capture_stable` 轮询（禁止裸 `sleep`）。
- [ ] `parse()` 复用现有 `vt100`，产出规范化 `Grid`；PTY 测试与真终端测试共用。
- [ ] 位置/尺寸断言辅助：`assert_text_at` / `assert_line_contains` / `find_border_box`。
- [ ] 配色断言只针对**程序发出的语义色**；文档注明不测终端主题映射。
- [ ] 多尺寸布局用例（如 `80x24` / `120x40`）。
- [ ] diff test：对齐输入 → 双端抓屏 → **比较投影**（正文/光标/选区/token 分类）→ 子网格 diff；失败时并排 dump。
- [ ] 鼠标用例保留在 PTY 测试，不进 tmux 这套。
- [ ] CI 安装 tmux；确认无需 GUI 终端即可运行。
- [ ] （可选）像素视觉回归独立、手动触发、不进 CI。

---

## 附：为什么用 tmux 而不是别的

- **不要求被测程序支持 PTY**：tmux 本身是终端，`msedit` 这类"启动即渲染、不走 PTY"的程序照样跑，diff test 才成立。
- **尺寸完全可控**：`-x/-y` 固定，布局断言才稳定可复现。
- **带色抓屏**：`capture-pane -e` 直接拿到程序发出的 SGR，配色可断言。
- **与现有 PTY 测试共用解析层**：都靠 `vt100`，断言原语一致，维护成本低。
