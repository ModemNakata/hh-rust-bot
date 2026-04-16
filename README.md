# HH Rust Bot 🦀🤖

A Telegram bot designed to help Rust developers find jobs by tracking and notifying about new vacancies from [HeadHunter (HH.ru)](https://hh.ru).

## 🚀 Features

-   **Automatic Tracking**: Periodically scans HH.ru for the latest Rust vacancies.
-   **Instant Notifications**: Automatically sends new vacancies to all subscribers as they appear.
-   **On-Demand Search**: Send a number (e.g., `5`, `10`) to the bot in a private chat to get that many of the most recent vacancies immediately.
-   **Group & Private Support**: Works in direct messages and can be added to Telegram groups.
-   **Rich Formatting**: Vacancies are delivered with clear formatting, including salary (if available), company name, location, and a direct link.
-   **Smart Duplicate Prevention**: Uses a local SQLite database to track seen vacancies and ensure you never get notified about the same job twice.

## 🛠 Tech Stack

-   **Language**: [Rust](https://www.rust-lang.org/)
-   **Bot Framework**: [Teloxide](https://github.com/teloxide/teloxide)
-   **API Client**: [Reqwest](https://github.com/seanmonstar/reqwest)
-   **Database**: [SQLite](https://sqlite.org/) (via [Rusqlite](https://github.com/rusqlite/rusqlite))
-   **Runtime**: [Tokio](https://tokio.rs/)

## 📋 Prerequisites

-   Rust (latest stable version)
-   A Telegram Bot Token (obtained from [@BotFather](https://t.me/botfather))

## ⚙️ Setup & Installation

1.  **Clone the repository**:
    ```bash
    git clone https://github.com/yourusername/hh-rust-bot.git
    cd hh-rust-bot
    ```

2.  **Configure environment variables**:
    Create a `.env` file in the root directory:
    ```env
    TELOXIDE_TOKEN=your_telegram_bot_token_here
    FETCH_INTERVAL_SECS=3600
    ```
    - `TELOXIDE_TOKEN`: Your bot's API token.
    - `FETCH_INTERVAL_SECS`: How often to check for new vacancies (default: 60s for debug, 3600s recommended for production).

3.  **Run the bot**:
    ```bash
    cargo run --release
    ```

## 🤖 Bot Commands

-   `/start` - Subscribe to automatic vacancy notifications (Private chats).
-   `/start@rust_hh_jobs_bot` - Subscribe the group to automatic notifications (Groups).
-   `/start@rust_hh_jobs_bot [count]` - Fetch `[count]` latest vacancies immediately in a group.
-   `[Number]` - Send a simple number (e.g. `5`) to get the N latest vacancies (Private chats).

## 📂 Project Structure

-   `src/main.rs`: Entry point, bot initialization, and message handling logic.
-   `src/hh_api.rs`: HH.ru API integration and vacancy data structures.
-   `src/db.rs`: SQLite database management for subscribers and state.
-   `src/scheduler.rs`: Background task that handles periodic vacancy fetching.

## 📄 License

This project is licensed under the MIT License - see the LICENSE file for details.
