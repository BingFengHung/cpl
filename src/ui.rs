use crate::model::Solution;
use crate::storage::Database;
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
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
use std::time::Duration;

pub fn render_results(solutions: &[Solution], db: &Database, interactive: bool) -> Result<()> {
    if solutions.is_empty() {
        println!("🔍 尚無符合的 AI 對話歷史紀錄 (No past solutions found matching your query).");
        println!("💡 提示: 可執行 `cpl scan --reindex` 重新掃描本機 Copilot/AGY CLI 日誌。");
        return Ok(());
    }

    if !interactive {
        render_text_list(solutions);
        return Ok(());
    }

    // Interactive TUI rendering
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

    match res {
        Ok(Some(UserAction::CopyCommand(cmd))) => {
            copy_to_clipboard(&cmd);
            println!("📋 已將內容複製到剪貼簿 (Copied to clipboard):\n   $ {}", cmd);
        }
        _ => {}
    }

    Ok(())
}

enum UserAction {
    CopyCommand(String),
}

fn run_tui_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    all_solutions: &[Solution],
    list_state: &mut ListState,
    _db: &Database,
) -> io::Result<Option<UserAction>> {
    let mut search_query = String::new();
    let mut filtered_solutions: Vec<Solution> = all_solutions.to_vec();

    // Flush leftover input events in stdin buffer (e.g. the Enter key pressed when launching cpl in CMD)
    while event::poll(Duration::from_millis(50)).unwrap_or(false) {
        let _ = event::read();
    }

    loop {
        let current_query = search_query.to_lowercase();
        filtered_solutions = if current_query.is_empty() {
            all_solutions.to_vec()
        } else {
            all_solutions
                .iter()
                .filter(|s| {
                    s.prompt_summary.to_lowercase().contains(&current_query)
                        || s.commands.iter().any(|c| c.to_lowercase().contains(&current_query))
                        || s.code_snippets.iter().any(|snip| snip.code.to_lowercase().contains(&current_query))
                })
                .cloned()
                .collect()
        };

        if list_state.selected().map_or(false, |i| i >= filtered_solutions.len()) {
            if filtered_solutions.is_empty() {
                list_state.select(None);
            } else {
                list_state.select(Some(0));
            }
        } else if list_state.selected().is_none() && !filtered_solutions.is_empty() {
            list_state.select(Some(0));
        }

        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(10), Constraint::Length(3)].as_ref())
                .split(f.size());

            // Top Search Input Box
            let header = Paragraph::new(format!(
                " 🔍 搜尋 (Search): {}_  (符合 {} / {} 筆, Enter: 複製, Esc: 離開)",
                search_query,
                filtered_solutions.len(),
                all_solutions.len()
            ))
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::ALL).title(" Copilot Plus - 即時搜尋 "));
            f.render_widget(header, chunks[0]);

            // Main dual-pane view
            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(45), Constraint::Percentage(55)].as_ref())
                .split(chunks[1]);

            // Left List items
            let items: Vec<ListItem> = filtered_solutions
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
            let preview_text = if let Some(sol) = filtered_solutions.get(selected_idx) {
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
                "尚無符合搜尋條件的解法項目".to_string()
            };

            let preview = Paragraph::new(preview_text)
                .block(Block::default().borders(Borders::ALL).title(" 解法預覽 (Preview) "))
                .style(Style::default().fg(Color::Green));
            f.render_widget(preview, main_chunks[1]);

            // Footer / Keybindings help
            let footer = Paragraph::new(" [直接打字] 即時過濾  |  [↑/↓] 移動選單  |  [Backspace] 刪除  |  [Enter] 複製指令  |  [Esc] 退出 ")
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(footer, chunks[2]);
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                match key.code {
                    KeyCode::Esc => return Ok(None),
                    KeyCode::Backspace => {
                        search_query.pop();
                        list_state.select(Some(0));
                    }
                    KeyCode::Down => {
                        let i = match list_state.selected() {
                            Some(i) => {
                                if filtered_solutions.is_empty() {
                                    0
                                } else if i >= filtered_solutions.len().saturating_sub(1) {
                                    0
                                } else {
                                    i + 1
                                }
                            }
                            None => 0,
                        };
                        list_state.select(Some(i));
                    }
                    KeyCode::Up => {
                        let i = match list_state.selected() {
                            Some(i) => {
                                if filtered_solutions.is_empty() {
                                    0
                                } else if i == 0 {
                                    filtered_solutions.len().saturating_sub(1)
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
                            if let Some(sol) = filtered_solutions.get(idx) {
                                if let Some(cmd) = sol.commands.first() {
                                    return Ok(Some(UserAction::CopyCommand(cmd.clone())));
                                } else if let Some(snip) = sol.code_snippets.first() {
                                    return Ok(Some(UserAction::CopyCommand(snip.code.clone())));
                                } else {
                                    return Ok(Some(UserAction::CopyCommand(sol.prompt_summary.clone())));
                                }
                            }
                        }
                    }
                    KeyCode::Char(c) => {
                        search_query.push(c);
                        list_state.select(Some(0));
                    }
                    _ => {}
                }
            }
        }
    }
}

pub fn render_text_list(solutions: &[Solution]) {
    println!("================================================================");
    println!(" 🔍 cpl recall - 歷史 AI 對話與解法紀錄 (共 {} 筆解法)", solutions.len());
    println!("================================================================");

    for (idx, sol) in solutions.iter().enumerate().take(15) {
        let pin = if sol.is_pinned { "⭐ " } else { "" };
        println!("\n[{}] {}{}", idx + 1, pin, sol.prompt_summary);
        println!("    📅 {} | 📂 {}", sol.formatted_date(), sol.project_path);
        for cmd in &sol.commands {
            println!("    💻 $ {}", cmd);
        }
        for snip in &sol.code_snippets {
            let first_line = snip.code.lines().next().unwrap_or(&snip.code);
            println!("    📝 [{}] {}", snip.language, first_line);
        }
    }
    println!("\n----------------------------------------------------------------");
    println!("💡 提示: 執行 `cpl recall -t` 可顯示純文字清單。");
    println!("================================================================");
}

fn copy_to_clipboard(text: &str) {
    if let Ok(mut board) = arboard::Clipboard::new() {
        let _ = board.set_text(text);
    }
}
