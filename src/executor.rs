use crate::model::Solution;
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use std::io::{self, Write};
use std::process::Command;
use std::time::Duration;

pub fn prompt_and_execute_solution(sol: &Solution) -> Result<()> {
    println!("================================================================");
    println!(" ⚡ cpl 0.001s 本機極速快取解法 (Local Fast Cache Hit)");
    println!("================================================================");
    println!(" 📌 問題: {}", sol.prompt_summary);
    println!(" 📅 時間: {} | 📂 路徑: {}", sol.formatted_date(), sol.project_path);
    println!("----------------------------------------------------------------");

    let cmd_to_run = if let Some(cmd) = sol.commands.first() {
        cmd.clone()
    } else if let Some(snip) = sol.code_snippets.first() {
        snip.code.clone()
    } else {
        println!("⚠️ 該紀錄中無可執行的指令或程式碼片段。");
        return Ok(());
    };

    println!(" 💻 建議執行指令:");
    println!("    $ {}\n", cmd_to_run);
    println!("----------------------------------------------------------------");
    println!(" 👉 請按 [Enter] 直接執行  |  按 [c] 複製到剪貼簿  |  按 [Esc/q] 取消");

    if enable_raw_mode().is_ok() {
        loop {
            if event::poll(Duration::from_millis(100)).unwrap_or(false) {
                if let Ok(Event::Key(key)) = event::read() {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }

                    match key.code {
                        KeyCode::Enter => {
                            let _ = disable_raw_mode();
                            println!("\n🚀 正在執行指令: {}\n", cmd_to_run);
                            run_shell_command(&cmd_to_run)?;
                            return Ok(());
                        }
                        KeyCode::Char('c') | KeyCode::Char('C') => {
                            let _ = disable_raw_mode();
                            copy_to_clipboard(&cmd_to_run);
                            println!("\n📋 已成功將指令複製到剪貼簿！");
                            return Ok(());
                        }
                        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                            let _ = disable_raw_mode();
                            println!("\n已取消執行。");
                            return Ok(());
                        }
                        _ => {}
                    }
                }
            }
        }
    } else {
        // Fallback standard stdin line prompt
        print!("請選擇 [y:執行 / c:複製 / n:取消]: ");
        let _ = io::stdout().flush();
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let choice = input.trim().to_lowercase();
        if choice == "y" || choice == "yes" || choice.is_empty() {
            println!("\n🚀 正在執行指令: {}\n", cmd_to_run);
            run_shell_command(&cmd_to_run)?;
        } else if choice == "c" {
            copy_to_clipboard(&cmd_to_run);
            println!("\n📋 已成功將指令複製到剪貼簿！");
        } else {
            println!("\n已取消執行。");
        }
    }

    Ok(())
}

pub fn fallback_cloud_ai(query: &str) -> Result<()> {
    println!("🌐 本機快取未命中，正在調用 Copilot CLI 雲端 AI 推理...");
    println!("   $ gh copilot suggest \"{}\"\n", query);

    let output = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(["/C", "gh", "copilot", "suggest", query])
            .status()
    } else {
        Command::new("gh")
            .args(["copilot", "suggest", query])
            .status()
    };

    if output.is_err() {
        println!("⚠️ 未安裝 GitHub Copilot CLI (gh copilot) 或無法調用。");
        println!("💡 您可以使用 `cpl` 查看過去本機的完整 AI 對話歷史。");
    }

    Ok(())
}

pub fn run_shell_command(cmd_str: &str) -> Result<()> {
    let status = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(["/C", cmd_str])
            .status()
    } else {
        Command::new("sh")
            .args(["-c", cmd_str])
            .status()
    };

    match status {
        Ok(code) => {
            if !code.success() {
                println!("\n⚠️ 指令執行返回錯誤碼: {:?}", code.code());
            }
        }
        Err(e) => {
            println!("\n❌ 無法啟動 Shell 執行指令: {}", e);
        }
    }

    Ok(())
}

fn copy_to_clipboard(text: &str) {
    if let Ok(mut board) = arboard::Clipboard::new() {
        let _ = board.set_text(text);
    }
}
