use crate::model::Solution;
use crate::storage::{AppStats, Database};
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

pub fn render_results(
    initial_solutions: &[Solution],
    db: &Database,
    interactive: bool,
    project_filter: Option<&str>,
    pinned_only: bool,
    initial_query: Option<&str>,
) -> Result<()> {
    let total_matched = db.get_total_count(initial_query, project_filter, pinned_only)?;

    if total_matched == 0 {
        println!("🔍 尚無符合的 AI 對話解法紀錄 (No past solutions found containing commands/snippets).");
        println!("💡 提示: 可執行 `cpl scan --reindex` 重新掃描本機 Copilot/AGY CLI 日誌。");
        return Ok(());
    }

    if !interactive {
        render_text_list(initial_solutions, total_matched);
        return Ok(());
    }

    // Interactive TUI rendering with Dynamic Lazy Loading
    if enable_raw_mode().is_err() {
        render_text_list(initial_solutions, total_matched);
        return Ok(());
    }

    let mut stdout = io::stdout();
    if execute!(stdout, EnterAlternateScreen).is_err() {
        let _ = disable_raw_mode();
        render_text_list(initial_solutions, total_matched);
        return Ok(());
    }

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = match Terminal::new(backend) {
        Ok(t) => t,
        Err(_) => {
            let _ = disable_raw_mode();
            render_text_list(initial_solutions, total_matched);
            return Ok(());
        }
    };

    let mut list_state = ListState::default();
    list_state.select(Some(0));

    let res = run_tui_loop(
        &mut terminal,
        db,
        &mut list_state,
        project_filter,
        pinned_only,
        initial_query,
    );

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
    db: &Database,
    list_state: &mut ListState,
    project_filter: Option<&str>,
    pinned_only: bool,
    initial_query: Option<&str>,
) -> io::Result<Option<UserAction>> {
    let mut search_query = initial_query.unwrap_or("").to_string();
    let mut solutions: Vec<Solution> = Vec::new();
    let mut total_matched = 0;
    let mut query_changed = true;

    // Flush leftover input events in stdin buffer
    while event::poll(Duration::from_millis(50)).unwrap_or(false) {
        let _ = event::read();
    }

    loop {
        // Re-query database dynamically when search query changes
        if query_changed {
            let q_param = if search_query.trim().is_empty() {
                None
            } else {
                Some(search_query.as_str())
            };
            total_matched = db.get_total_count(q_param, project_filter, pinned_only).unwrap_or(0);
            solutions = db.search_paged(q_param, project_filter, pinned_only, 0, 50).unwrap_or_default();
            query_changed = false;

            if solutions.is_empty() {
                list_state.select(None);
            } else {
                list_state.select(Some(0));
            }
        }

        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(10), Constraint::Length(3)].as_ref())
                .split(f.size());

            // Header with total count indicator
            let header = Paragraph::new(format!(
                " 🔍 搜尋 (Search): {}_  (符合 {} 筆精準解法, Enter: 複製, Esc: 離開)",
                search_query,
                total_matched
            ))
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::ALL).title(" Copilot Plus - 動態全庫搜尋 "));
            f.render_widget(header, chunks[0]);

            // Main dual-pane view
            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(45), Constraint::Percentage(55)].as_ref())
                .split(chunks[1]);

            // Left List items with index numbers [1], [2], [3]...
            let items: Vec<ListItem> = solutions
                .iter()
                .enumerate()
                .map(|(idx, s)| {
                    let pin_icon = if s.is_pinned { "⭐ " } else { "" };
                    let title = format!("[{}] {}{}", idx + 1, pin_icon, s.prompt_summary);
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
                "尚無符合搜尋條件的解法項目".to_string()
            };

            let preview = Paragraph::new(preview_text)
                .block(Block::default().borders(Borders::ALL).title(" 解法預覽 (Preview) "))
                .style(Style::default().fg(Color::Green));
            f.render_widget(preview, main_chunks[1]);

            // Footer / Keybindings help
            let footer = Paragraph::new(" [打字] 即時搜尋  |  [↑/↓] 動態捲動  |  [Backspace] 刪除  |  [Enter] 複製指令  |  [Esc] 退出 ")
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
                        query_changed = true;
                    }
                    KeyCode::Down => {
                        if let Some(i) = list_state.selected() {
                            let next_idx = i + 1;
                            // Dynamic Infinite Scroll: Lazy load next chunk when approaching end
                            if next_idx + 5 >= solutions.len() && solutions.len() < total_matched {
                                let q_param = if search_query.trim().is_empty() {
                                    None
                                } else {
                                    Some(search_query.as_str())
                                };
                                if let Ok(mut more) = db.search_paged(
                                    q_param,
                                    project_filter,
                                    pinned_only,
                                    solutions.len(),
                                    50,
                                ) {
                                    solutions.append(&mut more);
                                }
                            }

                            if next_idx < solutions.len() {
                                list_state.select(Some(next_idx));
                            }
                        }
                    }
                    KeyCode::Up => {
                        if let Some(i) = list_state.selected() {
                            if i > 0 {
                                list_state.select(Some(i - 1));
                            }
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(idx) = list_state.selected() {
                            if let Some(sol) = solutions.get(idx) {
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
                        query_changed = true;
                    }
                    _ => {}
                }
            }
        }
    }
}

pub fn render_text_list(solutions: &[Solution], total_count: usize) {
    println!("================================================================");
    println!(" 🔍 cpl recall - 歷史 AI 對話與解法紀錄 (共符合 {} 筆解法)", total_count);
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

pub fn render_stats(stats: &AppStats) {
    println!("==================================================================");
    println!(" 📊 Copilot Plus (cpl) - 開發者 AI 數據與使用儀表板 ");
    println!("==================================================================");
    println!(" 💡 索引解法總計 (Indexed Solutions): {} 筆", stats.total_solutions);
    println!(" ⭐ 星號收藏總計 (Pinned Recipes):    {} 筆", stats.pinned_solutions);
    println!(" 💻 提取 Shell 指令總數 (CLI Commands): {} 個", stats.total_commands);
    println!(" 📝 提取程式碼片段數 (Code Snippets):  {} 個", stats.total_snippets);
    println!("------------------------------------------------------------------");

    println!("\n🔥 【熱門 CLI 指令工具 Top 5】");
    let max_tool = stats.top_tools.first().map(|t| t.count).unwrap_or(1);
    if stats.top_tools.is_empty() {
        println!("  (尚無工具統計)");
    } else {
        for (idx, tool) in stats.top_tools.iter().enumerate() {
            let bar_len = (tool.count * 20) / max_tool;
            let bar = "█".repeat(bar_len.max(1));
            println!("  {}. {:<10} {:<20} ({} 次)", idx + 1, tool.name, bar, tool.count);
        }
    }

    println!("\n💻 【熱門程式語言類別 Top 5】");
    let max_lang = stats.top_languages.first().map(|l| l.count).unwrap_or(1);
    if stats.top_languages.is_empty() {
        println!("  (尚無語言統計)");
    } else {
        for (idx, lang) in stats.top_languages.iter().enumerate() {
            let bar_len = (lang.count * 20) / max_lang;
            let bar = "█".repeat(bar_len.max(1));
            println!("  {}. {:<10} {:<20} ({} 個片段)", idx + 1, lang.name, bar, lang.count);
        }
    }

    println!("\n📂 【最常使用 AI 的熱門專案 Top 5】");
    if stats.top_projects.is_empty() {
        println!("  (尚無專案統計)");
    } else {
        for (idx, proj) in stats.top_projects.iter().enumerate() {
            println!("  {}. {} ({} 筆對話)", idx + 1, proj.path, proj.count);
        }
    }

    println!("==================================================================");
}

fn copy_to_clipboard(text: &str) {
    if let Ok(mut board) = arboard::Clipboard::new() {
        let _ = board.set_text(text);
    }
}
