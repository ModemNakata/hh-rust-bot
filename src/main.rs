use dotenv::dotenv;
use std::sync::Arc;
use teloxide::prelude::*;

mod db;
mod hh_api;
mod scheduler;

use db::Database;
use hh_api::{HhApi, format_vacancy};
use scheduler::Scheduler;

#[tokio::main]
async fn main() {
    dotenv().ok();
    println!("[MAIN] ========================================");
    println!("[MAIN] Starting HH Rust Bot...");
    println!("[MAIN] ========================================");

    // Initialize database
    let db = match Database::new("subscribers.db") {
        Ok(db) => {
            println!("[MAIN] ✓ Database initialized");
            Arc::new(db)
        }
        Err(e) => {
            println!("[MAIN] ✗ Failed to initialize database: {}", e);
            panic!("Database initialization failed");
        }
    };

    // Initialize HH API client
    let api = Arc::new(HhApi::new());
    println!("[MAIN] ✓ HH API client initialized");

    // Initialize bot
    let bot = Bot::from_env();
    println!("[MAIN] ✓ Telegram bot initialized");
    println!("[MAIN] Bot username: @rust_hh_jobs_bot");

    // Start background scheduler (1 minute for debugging, change to 3600 for production)
    let debug_fetch_interval = std::env::var("FETCH_INTERVAL_SECS")
        .unwrap_or_else(|_| "3600".to_string())
        .parse::<u64>()
        .unwrap();
    // .unwrap_or(60 * 60); // 1 hour

    println!("[MAIN] Fetch interval: {} seconds", debug_fetch_interval);

    let scheduler = Scheduler::new(db.clone(), api.clone(), bot.clone(), debug_fetch_interval);
    scheduler.start().await;
    println!("[MAIN] ✓ Background scheduler started");

    // Start message handler
    println!("[MAIN] ✓ Starting message handler...");
    println!("[MAIN] Bot is ready to receive messages");
    println!("[MAIN] ========================================");

    teloxide::repl(bot, move |bot: Bot, msg: Message| {
        let db = db.clone();
        let api = api.clone();
        async move { handle_message(bot, msg, db, api).await }
    })
    .await;
}

async fn handle_message(
    bot: Bot,
    msg: Message,
    db: Arc<Database>,
    api: Arc<HhApi>,
) -> Result<(), teloxide::RequestError> {
    let chat_id = msg.chat.id.0;
    let text = msg.text().unwrap_or("(no text)");
    let chat_type = determine_chat_type(&msg.chat);

    println!(
        "[HANDLER] Received message from chat_id: {}, type: {}, text: {:?}",
        chat_id, chat_type, text
    );

    // Try to parse /start@rust_hh_jobs_bot [count] format
    if let Some(count) = parse_bot_command(text) {
        println!(
            "[HANDLER] Parsed /start@rust_hh_jobs_bot command with count: {}",
            count
        );

        println!(
            "[HANDLER] Processing /start@rust_hh_jobs_bot command from chat_id: {}",
            chat_id
        );

        // If command had a digit argument, also fetch those vacancies
        if count > 0 {
            println!(
                "[HANDLER] /start@rust_hh_jobs_bot command included vacancy count: {}",
                count
            );
            return fetch_and_send_vacancies(bot, msg, api, chat_id, count).await;
        } else {
            match db.add_chat_id(chat_id, chat_type) {
                Ok(_) => println!("[HANDLER] ✓ Chat registered successfully"),
                Err(e) => println!("[HANDLER] ✗ Failed to register chat: {}", e),
            }

            let welcome_msg = "🤖 Вы подписаны на новые вакансии Rust!";
            bot.send_message(msg.chat.id, welcome_msg)
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?;

            println!("[HANDLER] ✓ Welcome message sent");
        }

        return Ok(());
    }

    // Handle /start command (direct messages only)
    if text.to_lowercase().starts_with("/start") && chat_type == "private" {
        println!(
            "[HANDLER] Processing /start command from private chat: {}",
            chat_id
        );

        match db.add_chat_id(chat_id, chat_type) {
            Ok(_) => println!("[HANDLER] ✓ Chat registered successfully"),
            Err(e) => println!("[HANDLER] ✗ Failed to register chat: {}", e),
        }

        let welcome_msg = "🤖 Вы подписаны на новые вакансии Rust!";
        bot.send_message(msg.chat.id, welcome_msg)
            .parse_mode(teloxide::types::ParseMode::Html)
            .await?;

        println!("[HANDLER] ✓ Welcome message sent");
        return Ok(());
    }

    // Check if message is a digit - fetch N vacancies (only in private chats)
    if chat_type == "private" {
        if let Ok(count) = text.trim().parse::<u32>() {
            println!(
                "[HANDLER] Received digit request for {} vacancies from chat_id: {}",
                count, chat_id
            );

            return fetch_and_send_vacancies(bot, msg, api, chat_id, count).await;
        }
    }

    // Non-digit, non-command message in private chat - remind user to send digit
    if chat_type == "private" {
        println!(
            "[HANDLER] Non-digit message received from chat_id: {}",
            chat_id
        );
        println!("[HANDLER] Reminding user to send digit for vacancies");

        let reminder_msg = "📢 <b>Отправьте число</b>, чтобы получить вакансии!\n\nПримеры:\n• <code>5</code> - получить 5 последних вакансий\n• <code>10</code> - получить 10 вакансий\n• <code>1</code> - получить 1 вакансию\n\nМаксимум: 50 вакансий";
        bot.send_message(msg.chat.id, reminder_msg)
            .parse_mode(teloxide::types::ParseMode::Html)
            .await?;

        println!("[HANDLER] ✓ Reminder message sent");
    } else {
        // In group chat, only respond to /start@rust_hh_jobs_bot format
        println!(
            "[HANDLER] Ignoring non-command message in group chat (must use /start@rust_hh_jobs_bot)"
        );
    }

    Ok(())
}

async fn fetch_and_send_vacancies(
    bot: Bot,
    msg: Message,
    api: Arc<HhApi>,
    chat_id: i64,
    count: u32,
) -> Result<(), teloxide::RequestError> {
    let count = std::cmp::min(count, 50); // Cap at 50 to avoid API overload
    println!("[HANDLER] Capped request to {} vacancies (max 50)", count);

    match api.get_recent_vacancies(count).await {
        Ok(vacs) => {
            println!(
                "[HANDLER] ✓ Fetched {} vacancies for request from chat_id: {}",
                vacs.len(),
                chat_id
            );

            if vacs.is_empty() {
                println!("[HANDLER] No vacancies found");
                let no_vac_msg = "❌ К сожалению, вакансии не найдены.";
                bot.send_message(msg.chat.id, no_vac_msg).await?;
            } else {
                println!(
                    "[HANDLER] Sending {} vacancy(ies) to chat_id: {}",
                    vacs.len(),
                    chat_id
                );

                for (idx, vac) in vacs.iter().enumerate() {
                    let formatted = format_vacancy(&vac);
                    let vac_id = vac.id.clone().unwrap_or_else(|| "unknown".to_string());

                    println!(
                        "[HANDLER]   Sending vacancy {}/{}: id={}",
                        idx + 1,
                        vacs.len(),
                        vac_id
                    );

                    bot.send_message(msg.chat.id, formatted)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .await?;

                    println!("[HANDLER]   ✓ Vacancy {}/{} sent", idx + 1, vacs.len());
                }

                println!(
                    "[HANDLER] ✓ All {} vacancies sent to chat_id: {}",
                    vacs.len(),
                    chat_id
                );
            }
        }
        Err(e) => {
            println!("[HANDLER] ✗ Failed to fetch vacancies: {}", e);
            let error_msg = "❌ Ошибка при получении вакансий. Попробуйте позже.";
            bot.send_message(msg.chat.id, error_msg).await?;
        }
    }
    Ok(())
}

fn determine_chat_type(chat: &teloxide::types::Chat) -> &'static str {
    match chat.kind {
        teloxide::types::ChatKind::Private(_) => "private",
        teloxide::types::ChatKind::Public(_) => "public",
    }
}

/// Parse /start@rust_hh_jobs_bot [count] format
/// Only responds to THIS bot's username, ignores other bots
/// Returns count if matched (0 if no count), otherwise None
fn parse_bot_command(text: &str) -> Option<u32> {
    let text = text.trim();
    const BOT_USERNAME: &str = "rust_hh_jobs_bot";

    // Match pattern like /start@rust_hh_jobs_bot or /start@rust_hh_jobs_bot 5
    if text.starts_with("/start@") {
        // Extract the part after /start@
        let rest = &text[7..]; // Skip "/start@"

        // Check if it starts with our bot username
        if rest.starts_with(BOT_USERNAME) {
            let after_username = &rest[BOT_USERNAME.len()..];

            println!("[PARSER] ✓ Detected /start@{} command", BOT_USERNAME);

            // Check if there's a space-separated number after bot username
            if after_username.starts_with(' ') {
                let number_str = after_username.trim();
                if let Ok(count) = number_str.parse::<u32>() {
                    println!(
                        "[PARSER] ✓ Parsed /start@{} with digit: {}",
                        BOT_USERNAME, count
                    );
                    return Some(count);
                }
            } else if after_username.is_empty() {
                // Just /start@rust_hh_jobs_bot with no arguments
                println!(
                    "[PARSER] ✓ Parsed /start@{} without arguments",
                    BOT_USERNAME
                );
                return Some(0);
            }
        } else {
            // Different bot username - ignore
            let mentioned_bot = rest.split_whitespace().next().unwrap_or("?");
            println!(
                "[PARSER] ✗ Ignoring command for different bot: /start@{}",
                mentioned_bot
            );
            return None;
        }
    }

    None
}
