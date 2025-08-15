use futures::StreamExt;
use pdf2image::{image, RenderOptionsBuilder, PDF};
use serde::{Deserialize, Serialize};
use std::env;
use std::io::Cursor;
use anyhow::Result;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, EnvFilter};
use base64::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceInfo {
    pub payer_name: String, 
    pub payer_inn: String,
    pub payer_address: String,
    pub receiver_name: String,
    pub receiver_inn: String,
    pub receiver_address: String,
    pub receiver_account: String,
    pub receiver_bank_name: String,
    pub receiver_bank_bic: String,
    pub receiver_bank_corr_account: String,
    pub amount: f64,
    pub purpose: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub model: String,
    pub prompt: String,
    pub images: Vec<String>,
}

async fn send_prompt_with_image(
    model: &str,
    prompt: &str,
    file_path: &str,
) -> Result<InvoiceInfo> {
    let client = reqwest::Client::new();
    
    let (_, ext, _) = extract_file_info(file_path).map_err(|err| anyhow::anyhow!("Error extract file info: {}", err))?;
    
    let images_data = match ext.clone().unwrap_or_default().as_str() {
        "jpg" | "jpeg" => tokio::fs::read(file_path).await.map_err(|err| anyhow::anyhow!("Error reading file: {}", err)).map(|it| vec!(it))?,
        "pdf" => {
            let pdf = PDF::from_file(file_path)?;
                let pages = pdf.render(
                    pdf2image::Pages::Single(0),
                    RenderOptionsBuilder::default().pdftocairo(true).build()?,
                )?;
                pages.iter().map(|it| {
                    let mut bytes: Vec<u8> = Vec::new();
                    it.write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Jpeg).unwrap();
                    bytes
                }).map(|it| it.to_vec()).collect::<Vec<Vec<u8>>>()
        },
        _ => return Err(anyhow::anyhow!("Unsupported file extension: {}", ext.unwrap_or_default())),
    };

    let request = Request {
        model: model.to_string(),
        prompt: prompt.to_string(),
        images: images_data.iter().map(|it| BASE64_STANDARD.encode(it)).collect::<Vec<String>>(),
    };

    let response = client
        .post("http://localhost:11434/api/generate")
        .json(&request)
        .send()
        .await?;

    let mut full_response = String::new();
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        let chunk_str = String::from_utf8_lossy(&chunk);
        
        tracing::debug!("chunk_str: {}", chunk_str);
        
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&chunk_str) {
            if let Some(text) = json["response"].as_str() {
                full_response.push_str(text);
            } else if let Some(error) = json["error"].as_str() {
                return Err(anyhow::anyhow!("API error: {}", error));
            }
        } else {
            return Err(anyhow::anyhow!("Invalid JSON response: {}", chunk_str));
        }
    }

    let full_response = full_response.replace("```json", "").replace("```", "").trim().to_string();
    let invoice = serde_json::from_str::<InvoiceInfo>(&full_response).map_err(|err| anyhow::anyhow!("Error deserialize response: {}\n{}", err, full_response))?;

    Ok(invoice)
}

#[tokio::main]
async fn main() -> Result<()> {
     let args: Vec<String> = env::args().collect();
     
     if args.len() != 2 {
        eprintln!("Usage: invoice-scan-test <image_path>");
        return Err(anyhow::anyhow!("Invalid arguments"));
    }
    
    setup_logging();
    
    let image_path = &args[1];
    
    let response = send_prompt_with_image(
        "qwen2.5vl:7b", 
        r##"Ты — ИИ-ассистент для обработки финансовых документов.
        Пользователь предоставляет скан счёта на оплату. 
        
        **Задача:**
        Извлеки только следующие данные в формате JSON без пояснений, комментариев или форматирования:
        
        {
          "payerName": "Полное наименование плательщика",
          "payerInn": "ИНН плательщика (только цифры)",
          "payerAddress": "Юридический адрес плательщика, только адрес и ничего лишнего",
          "receiverName": "Полное наименование получателя платежа",
          "receiverInn": "ИНН получателя платежа (только цифры)",
          "receiverAddress": "Юридический адрес получателя платежа, только адрес и ничего лишнего",
          "receiverAccount": "Счет получателя платежа",
          "receiverBankName": "Нименование банка получателя платежа",
          "receiverBankBic": "БИК банка получателя платежа",
          "receiverBankCorrAccount": "Корреспондентский счет банка получателя платежа, начинается с цифрт 301 и имеет длину 20 символов",
          "amount": Сумма платежа в float (например 10000.0),
          "purpose": "Назначение платежа"
        }
        
        **Критические требования:**
        1. Выводи ТОЛЬКО готовый JSON-объект. Никакого текста до или после. Не используй markdown.
        2. Для ненайденных данных используй:
           - Пустую строку "" для текстовых полей
           - 0.0 для amount
        3. Преобразуй сумму в float (разделитель - точка)
        4. Убери лишние пробелы, кавычки и спецсимволы в извлечённых данных
        5. Для ИНН — только 10 или 12 цифр (без пробелов/знаков)
        
        **Как искать данные:**
        - Плательщик: блок "Плательщик", "Покупатель", "Отправитель", "Заказчик"
        - Получатель платежа: блок "Исполнитель", "Поставщик"
        - Назначение: поле "Назначение платежа", "Основание платежа"
        - Сумма: "Итого к оплате", "Сумма счёта", "К оплате"
        
        **Важно:** Если в документе несколько сумм — используй итоговую к оплате. Для адресов используй полные юридические адреса."##,
        image_path
    ).await?;

    println!("{}", serde_json::to_string_pretty(&response)?);
    
    Ok(())
}

use std::path::Path;
use mime_guess::MimeGuess;

fn extract_file_info(file_path: &str) -> Result<(String, Option<String>, String)> {
    let path = Path::new(file_path);
    
    let file_name = path.file_name()
        .and_then(|n| n.to_str())
        .ok_or(anyhow::anyhow!("file name not found"))?
        .to_string();

    let extension = path.extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.to_lowercase());

    let mime_type = extension.as_ref()
        .map_or_else(
            || "application/octet-stream".to_string(),
            |ext| {
                MimeGuess::from_ext(ext)
                    .first_or_octet_stream()
                    .to_string()
            }
        );

    tracing::debug!("file_name={} extension={} mime_type={}", file_name, extension.clone().unwrap_or_default(), mime_type);

    Ok((file_name, extension, mime_type))
}

fn setup_logging() {
    let filter = EnvFilter::from_default_env();
    
    let stdout_layer = fmt::layer().pretty();
    
    tracing_subscriber::registry()
        .with(stdout_layer)
        .with(filter)
        .init();
}
