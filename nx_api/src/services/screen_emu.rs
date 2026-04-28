//! 终端屏幕模拟器
//!
//! 用 `vte` 解析 PTY 字节流，按 VT100/xterm 协议在内存里维护一块"屏幕"（行列表 + 光标），
//! 处理光标定位、清屏、清行、CR/LF 等指令。
//!
//! 用途：把 claude code 等 TUI 程序的 PTY 输出"渲染"成最终用户能看到的纯文本，
//! 用于聊天气泡显示。**不影响** xterm.js 终端面板的原始流。
//!
//! 关键 API：
//! - `feed(&[u8])`：喂入 PTY 字节
//! - `drain_committed() -> String`：取已"凝固"的行（光标已离开，不会再被覆盖）

use vte::{Params, Parser, Perform};

/// 屏幕模拟器
pub struct ScreenEmu {
    /// 所有行（动态增长，不限制 24 行高度）
    rows: Vec<String>,
    /// 光标行（0-based）
    cursor_row: usize,
    /// 光标列（0-based）
    cursor_col: usize,
    /// 已发送给消费者的行数（即 rows[..emitted] 已经被 drain 过）
    emitted_rows: usize,
    /// 内嵌的 vte 解析器
    parser: Parser,
}

impl Default for ScreenEmu {
    fn default() -> Self {
        Self::new()
    }
}

impl ScreenEmu {
    pub fn new() -> Self {
        Self {
            rows: vec![String::new()],
            cursor_row: 0,
            cursor_col: 0,
            emitted_rows: 0,
            parser: Parser::new(),
        }
    }

    /// 喂入一段 PTY 字节流，触发屏幕状态更新
    pub fn feed(&mut self, data: &[u8]) {
        // vte::Parser::advance 需要 borrow self 两次（parser + state）
        // 用 std::mem::take 临时把 parser 拿出来，避开借用冲突
        let mut parser = std::mem::take(&mut self.parser);
        for &byte in data {
            parser.advance(self, byte);
        }
        self.parser = parser;
    }

    /// 取出已"凝固"的行（光标已经经过且向下移动了，不会再被覆盖）
    /// 调用后这些行从 emit 队列中移除
    pub fn drain_committed(&mut self) -> String {
        let mut out = String::new();
        while self.emitted_rows < self.cursor_row {
            let line = self.rows[self.emitted_rows].trim_end();
            out.push_str(line);
            out.push('\n');
            self.emitted_rows += 1;
        }
        out
    }

    /// 在 PTY 关闭时调用：把光标当前行也算作已凝固，flush 出来
    pub fn drain_remaining(&mut self) -> String {
        let mut out = self.drain_committed();
        if self.emitted_rows < self.rows.len() {
            for i in self.emitted_rows..self.rows.len() {
                let line = self.rows[i].trim_end();
                if !line.is_empty() {
                    out.push_str(line);
                    out.push('\n');
                }
            }
            self.emitted_rows = self.rows.len();
        }
        out
    }

    fn ensure_row(&mut self) {
        while self.rows.len() <= self.cursor_row {
            self.rows.push(String::new());
        }
    }

    /// 在光标位置写一个字符（覆盖该位置原有字符）
    fn write_char_at_cursor(&mut self, c: char) {
        self.ensure_row();
        // 把字符串解码成 Vec<char>，按列号定位
        let mut chars: Vec<char> = self.rows[self.cursor_row].chars().collect();
        // 不足光标列时用空格 pad
        while chars.len() < self.cursor_col {
            chars.push(' ');
        }
        if self.cursor_col < chars.len() {
            chars[self.cursor_col] = c;
        } else {
            chars.push(c);
        }
        self.rows[self.cursor_row] = chars.into_iter().collect();
        self.cursor_col += 1;
    }

    /// 解析 CSI 第 i 个参数，默认值 default
    fn csi_param(params: &Params, i: usize, default: usize) -> usize {
        params
            .iter()
            .nth(i)
            .and_then(|p| p.first().copied())
            .map(|v| v as usize)
            .filter(|v| *v > 0)
            .unwrap_or(default)
    }
}

impl Perform for ScreenEmu {
    fn print(&mut self, c: char) {
        // 过滤已知的 TUI 装饰字符（不影响光标推进，避免布局错位）
        let cp = c as u32;
        let is_decoration = (0x2500..=0x259F).contains(&cp)  // box drawing + block
            || (0x2800..=0x28FF).contains(&cp)               // braille spinner
            || c == '\u{FFFD}'; // 替换字符
        if is_decoration {
            // 仍然推进光标位置，否则后续字符会错位
            self.cursor_col += 1;
            return;
        }
        self.write_char_at_cursor(c);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' => {
                // LF：换行 + 回行首（终端默认 ONLCR：\n 等同于 \r\n）
                self.cursor_row += 1;
                self.cursor_col = 0;
                self.ensure_row();
            }
            b'\r' => {
                // CR：光标回到当前行行首
                self.cursor_col = 0;
            }
            b'\t' => {
                // TAB：跳到下一个 8 倍数列
                self.cursor_col = (self.cursor_col / 8 + 1) * 8;
            }
            0x08 => {
                // Backspace
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                }
            }
            _ => {}
        }
    }

    fn csi_dispatch(
        &mut self,
        params: &Params,
        _intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        match action {
            // CUU - 光标上移
            'A' => {
                let n = Self::csi_param(params, 0, 1);
                self.cursor_row = self.cursor_row.saturating_sub(n);
            }
            // CUD - 光标下移
            'B' => {
                let n = Self::csi_param(params, 0, 1);
                self.cursor_row += n;
                self.ensure_row();
            }
            // CUF - 光标右移
            'C' => {
                let n = Self::csi_param(params, 0, 1);
                self.cursor_col += n;
            }
            // CUB - 光标左移
            'D' => {
                let n = Self::csi_param(params, 0, 1);
                self.cursor_col = self.cursor_col.saturating_sub(n);
            }
            // CUP / HVP - 光标定位
            'H' | 'f' => {
                let row = Self::csi_param(params, 0, 1).saturating_sub(1);
                let col = Self::csi_param(params, 1, 1).saturating_sub(1);
                // 光标不能"回退"到已发送行之前（保护 emit 队列）
                self.cursor_row = row.max(self.emitted_rows);
                self.cursor_col = col;
                self.ensure_row();
            }
            // ED - 清屏
            'J' => {
                let mode = Self::csi_param(params, 0, 0);
                match mode {
                    0 => {
                        // 光标到屏幕末尾：删除 cursor_row 之后所有行 + cursor_row 中 cursor_col 之后的内容
                        self.rows.truncate(self.cursor_row + 1);
                        self.ensure_row();
                        let chars: Vec<char> = self.rows[self.cursor_row].chars().collect();
                        let kept: String = chars.iter().take(self.cursor_col).collect();
                        self.rows[self.cursor_row] = kept;
                    }
                    1 => {
                        // 屏幕开头到光标：清空 emitted 之后到 cursor_row 之前的所有行
                        for i in self.emitted_rows..self.cursor_row {
                            self.rows[i].clear();
                        }
                        // 当前行光标之前的字符变空格
                        self.ensure_row();
                        let chars: Vec<char> = self.rows[self.cursor_row].chars().collect();
                        let mut new_chars = vec![' '; self.cursor_col.min(chars.len())];
                        new_chars.extend(chars.iter().skip(self.cursor_col));
                        self.rows[self.cursor_row] = new_chars.into_iter().collect();
                    }
                    2 | 3 => {
                        // 整屏：保留已 emit 的，清掉未 emit 的
                        self.rows.truncate(self.emitted_rows);
                        self.rows.push(String::new());
                        self.cursor_row = self.emitted_rows;
                        self.cursor_col = 0;
                    }
                    _ => {}
                }
            }
            // EL - 清行
            'K' => {
                self.ensure_row();
                let mode = Self::csi_param(params, 0, 0);
                let chars: Vec<char> = self.rows[self.cursor_row].chars().collect();
                let new_str: String = match mode {
                    // 光标到行尾
                    0 => chars.iter().take(self.cursor_col).collect(),
                    // 行首到光标
                    1 => {
                        let pad = self.cursor_col.min(chars.len());
                        let mut s: String = " ".repeat(pad);
                        s.extend(chars.iter().skip(self.cursor_col));
                        s
                    }
                    // 整行
                    2 => String::new(),
                    _ => self.rows[self.cursor_row].clone(),
                };
                self.rows[self.cursor_row] = new_str;
            }
            _ => {
                // 其他 CSI 不处理（颜色 SGR/m、滚动 r、等等）
            }
        }
    }

    // 以下方法用默认空实现（不处理 OSC/DCS/ESC dispatch 等）
    fn hook(&mut self, _: &Params, _: &[u8], _: bool, _: char) {}
    fn put(&mut self, _: u8) {}
    fn unhook(&mut self) {}
    fn osc_dispatch(&mut self, _: &[&[u8]], _: bool) {}
    fn esc_dispatch(&mut self, _: &[u8], _: bool, _: u8) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_passes_through() {
        let mut emu = ScreenEmu::new();
        emu.feed(b"hello\n");
        assert_eq!(emu.drain_committed(), "hello\n");
    }

    #[test]
    fn cursor_right_preserves_space() {
        // 模拟 "Welcome\x1b[1Cback!" → 应该是 "Welcome back!"
        let mut emu = ScreenEmu::new();
        emu.feed(b"Welcome\x1b[1Cback!\n");
        assert_eq!(emu.drain_committed(), "Welcome back!\n");
    }

    #[test]
    fn carriage_return_overwrites() {
        // 模拟 spinner: "Loading..." \r "Loaded!  " \n → 最后只剩 "Loaded!"
        let mut emu = ScreenEmu::new();
        emu.feed(b"Loading...\rLoaded!\n");
        // CR 后从列 0 开始覆盖 "Loadi"，留下 "Loaded!ng..."？不对
        // CR 把光标设到 0，然后写 "Loaded!" 7 字符 → 第 7 列起还是 "..."
        // 输出应该是 "Loaded!..."
        assert_eq!(emu.drain_committed(), "Loaded!...\n");
    }

    #[test]
    fn clear_line_works() {
        // "Loading..." \r \x1b[2K "Done\n"
        let mut emu = ScreenEmu::new();
        emu.feed(b"Loading...\r\x1b[2KDone\n");
        assert_eq!(emu.drain_committed(), "Done\n");
    }

    #[test]
    fn box_drawing_filtered() {
        let mut emu = ScreenEmu::new();
        emu.feed("╭───╮\n│ Hi │\n╰───╯\nReal text\n".as_bytes());
        // Box drawing 字符被过滤但占位空格保留
        let out = emu.drain_committed();
        assert!(out.contains("Real text"));
        assert!(!out.contains('╭'));
        assert!(!out.contains('│'));
    }

    #[test]
    fn drain_does_not_include_cursor_row() {
        // committed 行 = emitted..cursor_row（不含 cursor_row）
        let mut emu = ScreenEmu::new();
        emu.feed(b"line1\nline2"); // line2 没换行，cursor 还在 line2
        let out = emu.drain_committed();
        assert_eq!(out, "line1\n"); // 只有 line1 凝固
    }

    #[test]
    fn drain_remaining_includes_cursor_row() {
        let mut emu = ScreenEmu::new();
        emu.feed(b"line1\nline2");
        let out = emu.drain_remaining();
        assert_eq!(out, "line1\nline2\n");
    }
}
