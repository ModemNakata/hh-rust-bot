use reqwest::Client;
use serde::Deserialize;

const BASE_URL: &str = "https://api.hh.ru";
const USER_AGENT: &str = "HH-Rust-Bot/1.0 (https://t.me/rust_hh_jobs_bot)";

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Vacancy {
    pub id: Option<String>,
    pub name: Option<String>,
    pub area: Option<Area>,
    #[serde(rename = "salary_range")]
    pub salary: Option<SalaryRange>,
    pub employer: Option<Employer>,
    pub published_at: Option<String>,
    #[serde(rename = "alternate_url")]
    pub alternate_url: Option<String>,
    pub snippet: Option<Snippet>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Snippet {
    pub requirement: Option<String>,
    // pub responsibility: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Area {
    // pub id: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct SalaryRange {
    pub from: Option<i64>,
    pub to: Option<i64>,
    pub currency: Option<String>,
    // #[serde(rename = "gross")]
    // pub is_gross: Option<bool>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Employer {
    // pub id: Option<String>,
    pub name: Option<String>,
    // #[serde(rename = "logo_urls")]
    // pub logo_urls: Option<LogoUrls>,
}

// #[derive(Debug, Deserialize, Clone, Default)]
// pub struct LogoUrls {
// #[serde(rename = "90")]
// pub small: Option<String>,
// #[serde(rename = "240")]
// pub medium: Option<String>,
// pub original: Option<String>,
// }

#[derive(Debug, Deserialize, Default)]
pub struct VacanciesResponse {
    pub items: Vec<Vacancy>,
    pub found: Option<u32>,
    pub page: Option<u32>,
    pub pages: Option<u32>,
    pub per_page: Option<u32>,
}

fn format_published_time(published: &str) -> String {
    if published.len() < 19 {
        return published.to_string();
    }
    let (date_part, time_part) = published.split_at(10);
    let time = &time_part[1..8];

    let months = [
        "", "янв", "фев", "мар", "апр", "май", "июн", "июл", "авг", "сен", "окт", "ноя", "дек",
    ];

    let parts: Vec<&str> = date_part.split('-').collect();
    if parts.len() >= 3 {
        let day = parts[2];
        let month: usize = parts[1].parse().unwrap_or(0);
        let month_name = if month < 13 { months[month] } else { "" };
        let year = parts[0];
        return format!("{} {} {} {}", day, month_name, year, time);
    }
    published.to_string()
}

pub struct HhApi {
    client: Client,
}

impl HhApi {
    pub fn new() -> Self {
        let client = Client::builder()
            .default_headers({
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert(
                    reqwest::header::USER_AGENT,
                    USER_AGENT.parse().expect("Invalid User-Agent"),
                );
                headers
            })
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }

    pub async fn get_vacancies(
        &self,
        text: &str,
        per_page: u32,
    ) -> Result<VacanciesResponse, reqwest::Error> {
        let url = format!(
            "{}/vacancies?text={}&per_page={}&search_field=name",
            BASE_URL, text, per_page
        );

        println!("[HH_API] REQUEST: GET {}", url);
        println!("[HH_API] User-Agent: {}", USER_AGENT);

        let response = self.client.get(&url).send().await?;

        println!("[HH_API] RESPONSE STATUS: {}", response.status());

        let vacancies: VacanciesResponse = response.json().await?;

        println!(
            "[HH_API] RESPONSE: found={:?}, page={:?}/{:?}, per_page={:?}",
            vacancies.found, vacancies.page, vacancies.pages, vacancies.per_page
        );

        Ok(vacancies)
    }

    pub async fn get_recent_vacancies(
        &self,
        per_page: u32,
    ) -> Result<Vec<Vacancy>, reqwest::Error> {
        println!(
            "[HH_API] get_recent_vacancies(text=Rust, per_page={}, search_field=name)",
            per_page
        );

        let response = self.get_vacancies("Rust", per_page).await?;

        let items = response.items;
        println!("[HH_API] Got {} vacancies:", items.len());

        for (i, vac) in items.iter().enumerate() {
            let id = vac.id.clone().unwrap_or_else(|| "?".to_string());
            let name = vac.name.clone().unwrap_or_else(|| "?".to_string());
            let area = vac
                .area
                .as_ref()
                .and_then(|a| a.name.clone())
                .unwrap_or_else(|| "?".to_string());
            let employer = vac
                .employer
                .as_ref()
                .and_then(|e| e.name.clone())
                .unwrap_or_else(|| "?".to_string());
            let published = vac.published_at.clone().unwrap_or_else(|| "?".to_string());
            let salary = if let Some(ref s) = vac.salary {
                format!(
                    "{} - {} {}",
                    s.from
                        .map(|f| f.to_string())
                        .unwrap_or_else(|| "?".to_string()),
                    s.to.map(|t| t.to_string())
                        .unwrap_or_else(|| "?".to_string()),
                    s.currency.as_deref().unwrap_or("RUR")
                )
            } else {
                "Not specified".to_string()
            };
            let requirements = vac
                .snippet
                .as_ref()
                .and_then(|s| s.requirement.clone())
                .unwrap_or_else(|| "".to_string());

            println!("[HH_API]   [{}] id={}", i, id);
            println!("[HH_API]       name: {}", name);
            println!("[HH_API]       area: {}", area);
            println!("[HH_API]       employer: {}", employer);
            println!("[HH_API]       salary: {}", salary);
            println!(
                "[HH_API]       published_at: {}",
                format_published_time(&published)
            );
            println!(
                "[HH_API]       requirements: {}",
                requirements.chars().take(100).collect::<String>()
            );
        }

        Ok(items)
    }
}

impl Default for HhApi {
    fn default() -> Self {
        Self::new()
    }
}

pub fn format_vacancy(vacancy: &Vacancy) -> String {
    let salary = if let Some(ref s) = vacancy.salary {
        let from = s
            .from
            .map(|f| f.to_string())
            .unwrap_or_else(|| "?".to_string());
        let to =
            s.to.map(|t| t.to_string())
                .unwrap_or_else(|| "?".to_string());
        let curr = s.currency.as_deref().unwrap_or("RUR");
        format!("{} - {} {}", from, to, curr)
    } else {
        "Не указана".to_string()
    };

    let area = vacancy
        .area
        .as_ref()
        .and_then(|a| a.name.clone())
        .unwrap_or_else(|| "Unknown".to_string());
    let employer = vacancy
        .employer
        .as_ref()
        .and_then(|e| e.name.clone())
        .unwrap_or_else(|| "Unknown".to_string());

    let name = vacancy
        .name
        .clone()
        .unwrap_or_else(|| "Unknown".to_string());
    let url = vacancy
        .alternate_url
        .clone()
        .unwrap_or_else(|| "".to_string());
    let published = vacancy
        .published_at
        .clone()
        .unwrap_or_else(|| "".to_string());

    format!(
        "🎯 <b>{}</b>\n💼 {}\n📍 {}\n💰 {}\n🔗 {}\n🕐 {}",
        name,
        employer,
        area,
        salary,
        url,
        format_published_time(&published)
    )
}
