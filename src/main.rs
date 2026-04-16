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

    // Start background scheduler (1 minute for debugging, change to 3600 for production)
    let debug_fetch_interval = std::env::var("FETCH_INTERVAL_SECS")
        .unwrap_or_else(|_| "60".to_string())
        .parse::<u64>()
        .unwrap_or(60);

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

    // Handle /start command
    if text.to_lowercase().starts_with("/start") {
        println!(
            "[HANDLER] Processing /start command from chat_id: {}",
            chat_id
        );

        match db.add_chat_id(chat_id, chat_type) {
            Ok(_) => println!("[HANDLER] ✓ Chat registered successfully"),
            Err(e) => println!("[HANDLER] ✗ Failed to register chat: {}", e),
        }

        let welcome_msg = "🤖 <b>HH Rust Bot</b>\n\nВы подписаны на новые вакансии Rust!\n📝 Отправьте число (например, 5) чтобы получить N последних вакансий.";
        bot.send_message(msg.chat.id, welcome_msg)
            .parse_mode(teloxide::types::ParseMode::Html)
            .await?;

        println!("[HANDLER] ✓ Welcome message sent");
        return Ok(());
    }

    // Check if message is a digit - fetch N vacancies
    if let Ok(count) = text.trim().parse::<u32>() {
        println!(
            "[HANDLER] Received digit request for {} vacancies from chat_id: {}",
            count, chat_id
        );

        let count = std::cmp::min(count, 50); // Cap at 50 to avoid API overload
        println!(
            "[HANDLER] Capped request to {} vacancies (max 50)",
            count
        );

        match api.get_recent_vacancies(count).await {
            Ok(vacs) => {
                println!(
                    "[HANDLER] ✓ Fetched {} vacancies for digit request from chat_id: {}",
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
        return Ok(());
    }

    // For non-digit, non-start messages, show a quick vacancy
    println!("[HANDLER] Fetching sample vacancy for chat_id: {}", chat_id);
    match api.get_recent_vacancies(1).await {
        Ok(vacs) => {
            println!("[HANDLER] ✓ Got {} vacancies", vacs.len());

            if let Some(vac) = vacs.first() {
                let formatted = format_vacancy(&vac);

                println!("[HANDLER] Sending vacancy to chat_id: {}", chat_id);

                bot.send_message(msg.chat.id, formatted)
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .await?;

                println!("[HANDLER] ✓ Vacancy sent");
            }
        }
        Err(e) => {
            println!("[HANDLER] ✗ Failed to fetch vacancy: {}", e);
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
