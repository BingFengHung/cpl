use crate::model::Solution;
use crate::storage::Database;
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Terminal,
};
use std::io;
use std::time::{Duration, Instant};

pub fn render_interactive(solutions: &[Solution], db: &Database, text_only: bool) -> Result<()> {
    if solutions.is_empty() {
        println!("🔍 尚無符合的 AI 對話歷史紀錄 (No past solutions found matching your query).");
        println!("💡 提示: 可執行 `cpl scan --reindex` 重新掃描本機 Copilot/AGY CLI 日誌。");
        return Ok(());
    }

    if text_only {
        render_text_list(solutions);
        return Ok(());
    }

    let start_time = Instant::now();

    // Try initializing terminal raw mode for TUI
    if enable_raw_mode().is_err() {
        render_text_list(solutions);
        return Ok(());
    }

    let mut stdout = io::stdout();
    if execute!(stdout, EnterAlternateScreen).is_err() {
        let _ = disable_raw_mode();
        render_text_list(solutions);
        return Ok(());
    }

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = match Terminal::new(backend) {
        Ok(t) => t,
        Err(_) => {
            let _ = disable_raw_mode();
            render_text_list(solutions);
            return Ok(());
        }
    };

    let mut list_state = ListState::default();
    list_state.select(Some(0));

    let res = run_tui_loop(&mut terminal, solutions, &mut list_state, db);

    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let _ = terminal.show_cursor();

    let elapsed = start_time.elapsed();

    // Fallback to text list if TUI exited too fast (<100ms) without selection
    if elapsed < Duration::from_millis(100) && res.as_ref().ok().and_then(|a| a.as_ref()).is_none() {
        render_text_list(solutions);
        return Ok(());
    }

    if let Ok(Some(action)) = res {
        match action {
            UserAction::CopyCommand(cmd) => {
                copy_to_clipboard(&cmd);
                println!("📋 已將指令複製到剪貼簿 (Copied to clipboard):\n   $ {}", cmd);
            }
            UserAction::PrintCommand(cmd) => {
                println!("💻 $ {}", cmd);
            }
        }
    }

    Ok(())
}

enum UserAction {
    CopyCommand(String),
    PrintCommand(String),
}

fn run_tui_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    solutions: &[Solution],
    list_state: &mut ListState,
    _db: &Database,
) -> io::Result<Option<UserAction>> {
    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(10), Constraint::Length(3)].as_ref())
                .split(f.size());

            // Header
            let header = Paragraph::new(format!(
                " 🔍 cpl recall - AI 解法與指令記憶庫 (共 {} 筆解法, Enter: 複製, q: 離開)",
                solutions.len()
            ))
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::ALL).title(" Copilot Plus "));
            f.render_widget(header, chunks[0]);

            // Main dual-pane view
            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(45), Constraint::Percentage(55)].as_ref())
                .split(chunks[1]);

            // Left List items
            let items: Vec<ListItem> = solutions
                .iter()
                .map(|s| {
                    let pin_icon = if s.is_pinned { "⭐ " } else { "  " };
                    let title = format!("{}{}", pin_icon, s.prompt_summary);
                    ListItem::new(title).style(Style::default().fg(Color::White))
                })
                .collect();

            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title(" 歷史解法清單 (Solutions) "))
                .highlight_style(
                    Style::default()
                        .bg(Color::Blue)
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("> ");

            f.render_stateful_widget(list, main_chunks[0], list_state);

            // Right Preview Pane
            let selected_idx = list_state.selected().unwrap_or(0);
            let preview_text = if let Some(sol) = solutions.get(selected_idx) {
                let mut text = format!(
                    "📅 時間: {}\n📂 路徑: {}\n\n💡 執行的關鍵指令:\n",
                    sol.formatted_date(),
                    sol.project_path
                );
                if sol.commands.is_empty() {
                    text.push_str("  (無直接執行的命令)\n");
                } else {
                    for cmd in &sol.commands {
                        text.push_str(&format!("  $ {}\n", cmd));
                    }
                }
                if !sol.code_snippets.is_empty() {
                    text.push_str("\n📝 程式碼片段:\n");
                    for snip in &sol.code_snippets {
                        text.push_str(&format!("--- [{}] ---\n{}\n", snip.language, snip.code));
                    }
                }
                text
            } else {
                "尚無選取的項目".to_string()
            };

            let preview = Paragraph::new(preview_text)
                .block(Block::default().borders(Borders::ALL).title(" 解法預覽 (Preview) "))
                .style(Style::default().fg(Color::Green));
            f.render_widget(preview, main_chunks[1]);

            // Footer / Keybindings help
            let footer = Paragraph::new(" [↑/↓] 移動選單  |  [Enter] 複製指令至剪貼簿  |  [q/Esc] 退出 ")
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(footer, chunks[2]);
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(None),
                    KeyCode::Down | KeyCode::Char('j') => {
                        let i = match list_state.selected() {
                            Some(i) => {
                                if i >= solutions.len() - 1 {
                                    0
                                } else {
                                    i + 1
                                }
                            }
                            None => 0,
                        };
                        list_state.select(Some(i));
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        let i = match list_state.selected() {
                            Some(i) => {
                                if i == 0 {
                                    solutions.len() - 1
                                } else {
                                    i - 1
                                }
                            }
                            None => 0,
                        };
                        list_state.select(Some(i));
                    }
                    KeyCode::Enter => {
                        if let Some(idx) = list_state.selected() {
                            if let Some(sol) = solutions.get(idx) {
                                if let Some(cmd) = sol.commands.first() {
                                    return Ok(Some(UserAction::CopyCommand(cmd.clone())));
                                }
                            }
                        }
                        return Ok(None);
                    }
                    _ => {}
                }
            }
        }
    }
}

pub fn render_text_list(solutions: &[Solution]) {
    println!("\n🔍 共找到 {} 筆歷史對話解法紀錄:\n", solutions.len());
    for (idx, sol) in solutions.iter().enumerate() {
        let pin = if sol.is_pinned { "⭐ " } else { "" };
        println!("{}. {}{}", idx + 1, pin, sol.prompt_summary);
        println!("   📅 {} | 📂 {}", sol.formatted_date(), sol.project_path);
        for cmd in &sol.commands {
            println!("   💻 $ {}", cmd);
        }
        println!();
    }
}

fn copy_to_clipboard(text: &str) {
    if let Ok(mut board) = arboard::Clipboard::new() {
        let _ = board.set_text(text);
    }
}
