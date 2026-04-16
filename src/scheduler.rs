use crate::db::Database;
use crate::hh_api::HhApi;
use std::sync::Arc;
use teloxide::Bot;
use teloxide::prelude::Requester;
use teloxide::payloads::SendMessageSetters;
use tokio::time::{sleep, Duration};

/// Background scheduler for fetching and notifying vacancies
pub struct Scheduler {
    db: Arc<Database>,
    api: Arc<HhApi>,
    bot: Bot,
    fetch_interval_secs: u64,
}

impl Scheduler {
    pub fn new(db: Arc<Database>, api: Arc<HhApi>, bot: Bot, fetch_interval_secs: u64) -> Self {
        println!("[SCHEDULER] Initialized with fetch interval: {} seconds", fetch_interval_secs);
        Scheduler {
            db,
            api,
            bot,
            fetch_interval_secs,
        }
    }

    /// Start the background scheduler task
    pub async fn start(self) {
        println!("[SCHEDULER] Starting background task");
        
        tokio::spawn(async move {
            println!("[SCHEDULER] Background task spawned");
            
            loop {
                println!("[SCHEDULER] ===== Fetch cycle started =====");
                
                // Fetch latest vacancies
                match self.fetch_and_notify().await {
                    Ok(_) => println!("[SCHEDULER] ✓ Fetch cycle completed successfully"),
                    Err(e) => println!("[SCHEDULER] ✗ Fetch cycle failed: {}", e),
                }
                
                println!("[SCHEDULER] Sleeping for {} seconds until next cycle", self.fetch_interval_secs);
                sleep(Duration::from_secs(self.fetch_interval_secs)).await;
            }
        });
    }

    /// Fetch new vacancies and notify subscribers
    async fn fetch_and_notify(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Get current latest vacancy ID
        let current_latest = self.db.get_latest_vacancy_id()?;
        println!("[SCHEDULER] Current latest vacancy ID: {}", current_latest);

        // Fetch recent vacancies
        println!("[SCHEDULER] Fetching recent Rust vacancies...");
        let vacancies = self.api.get_recent_vacancies(50).await?;
        println!("[SCHEDULER] ✓ Fetched {} vacancies", vacancies.len());

        if vacancies.is_empty() {
            println!("[SCHEDULER] No vacancies found, skipping notification");
            return Ok(());
        }

        // Find new vacancies
        let mut new_vacancies = Vec::new();
        let mut highest_id = current_latest.clone();

        for vac in &vacancies {
            if let Some(vac_id) = &vac.id {
                println!("[SCHEDULER] Checking vacancy: id={}", vac_id);
                
                if vac_id > &highest_id {
                    highest_id = vac_id.clone();
                }

                if vac_id > &current_latest {
                    println!("[SCHEDULER]   ✓ New vacancy found: {} (newer than {})", vac_id, current_latest);
                    new_vacancies.push(vac.clone());
                } else {
                    println!("[SCHEDULER]   - Vacancy {} is not new (older than or equal to {})", vac_id, current_latest);
                }
            }
        }

        if new_vacancies.is_empty() {
            println!("[SCHEDULER] No new vacancies to notify");
            return Ok(());
        }

        println!("[SCHEDULER] Found {} new vacancy(ies)", new_vacancies.len());

        // Get all subscribers
        let subscribers = self.db.get_all_chat_ids()?;
        println!("[SCHEDULER] Found {} subscriber(s) to notify", subscribers.len());

        if subscribers.is_empty() {
            println!("[SCHEDULER] No subscribers to notify");
        } else {
            // Notify each subscriber
            for (chat_id, chat_type) in &subscribers {
                println!("[SCHEDULER] Notifying chat_id: {} (type: {})", chat_id, chat_type);
                
                for vac in &new_vacancies {
                    let formatted = crate::hh_api::format_vacancy(vac);
                    let vac_id = vac.id.clone().unwrap_or_else(|| "unknown".to_string());
                    
                    match self.bot.send_message(
                        teloxide::types::ChatId(*chat_id),
                        formatted
                    )
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .await
                    {
                        Ok(_) => println!("[NOTIFICATION] ✓ Sent vacancy {} to chat_id: {}", vac_id, chat_id),
                        Err(e) => println!("[NOTIFICATION] ✗ Failed to send vacancy {} to chat_id: {} - {}", vac_id, chat_id, e),
                    }
                }
            }
        }

        // Update latest vacancy ID
        println!("[SCHEDULER] Updating latest vacancy ID to: {}", highest_id);
        self.db.update_latest_vacancy_id(&highest_id)?;
        println!("[SCHEDULER] ✓ Latest vacancy ID updated");

        Ok(())
    }
}
