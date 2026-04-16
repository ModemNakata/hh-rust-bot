use dotenv::dotenv;
use teloxide::prelude::*;

mod hh_api;
use hh_api::{format_vacancy, HhApi};

#[tokio::main]
async fn main() {
    dotenv().ok();
    println!("[INFO] Starting HH Rust Bot...");

    let bot = Bot::from_env();
    let api = std::sync::Arc::new(HhApi::new());

    teloxide::repl(bot, move |bot: Bot, msg: Message| {
        let api = api.clone();
        async move {
            handle_message(bot, msg, api).await
        }
    })
    .await;
}

async fn handle_message(bot: Bot, msg: Message, api: std::sync::Arc<hh_api::HhApi>) -> Result<(), teloxide::RequestError> {
    let chat_id = msg.chat.id.0;
    let text = msg.text().unwrap_or("(no text)");
    
    println!("[USER_MESSAGE] chat_id: {}, text: {:?}", chat_id, text);

    if text.to_lowercase().starts_with("/start") {
        println!("[COMMAND] /start from chat_id: {}", chat_id);
    }

    println!("[API_REQUEST] Fetching vacancy from HH API...");
    let vacancies = api.get_recent_vacancies(1).await;
    
    match &vacancies {
        Ok(vacs) => println!("[API_RESPONSE] Got {} vacancies", vacs.len()),
        Err(e) => println!("[API_ERROR] Failed: {}", e),
    }

    if let Ok(vacs) = vacancies {
        if let Some(vac) = vacs.first() {
            let formatted = format_vacancy(&vac);
            
            println!("[SEND_MESSAGE] To chat_id: {}: {}", chat_id, 
                formatted.lines().next().unwrap_or("(empty)").chars().take(50).collect::<String>()
            );
            
            bot.send_message(msg.chat.id, formatted)
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?;
                
            println!("[MESSAGE_SENT] chat_id: {}", chat_id);
        }
    }
    Ok(())
}