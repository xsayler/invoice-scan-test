# Распознавание счетов на оплату

Это приложение на Rust извлекает структурированную информацию из платежных счетов с использованием оптического распознавания символов (OCR) и ИИ-обработки.

## Возможности

- Извлечение ключевых платежных реквизитов из счетов
- Поддержка форматов JPEG и PDF
- Вывод структурированных данных в JSON
- Использование локального ИИ для конфиденциальности данных

## Требования

Перед использованием необходимо установить:

### 1. Ollama
- Скачайте и установите Ollama с [ollama.com](https://ollama.com/)
- Запустите сервис Ollama после установки

### 2. ИИ-модель
- Установите необходимую модель:
  ```bash
  ollama pull qwen2.5vl:7b
  ```
  
### 3. Poppler Utilities (для обработки PDF)
- Linux (Debian/Ubuntu):  
  ```bash
  sudo apt-get install poppler-utils
  ```
- macOS (через Homebrew):
  ```bash
  brew install poppler
  ```
- Windows:
  Скачайте с [poppler-windows](https://github.com/oschwartz10612/poppler-windows)
  
## Установка
  ```bash
  cargo build --release
  ```
  
## Запуск
  ```bash
  ./target/release/invoice-scan-test <путь-к-файлу-счета>
  ```
### Пример:  
  ```bash
  ./target/release/invoice-scan-test invoice.pdf
  ```

## Вывод:
Приложение выведет структурированную информацию в формате JSON:
  ```json
  {
    "payerName": "ООО \"Компания Плательщик\"",
    "payerInn": "770123456789",
    "payerAddress": "г. Москва, ул. Платежная, д. 1",
    "receiverName": "ООО 'Компания Получатель'",
    "receiverInn": "770987654321",
    "receiverAddress": "г. Санкт-Петербург, Невский пр-т, д. 100",
    "receiverAccount": "40702810123450123456",
    "receiverBankName": "ПАО \"Банк Получателя\"",
    "receiverBankBic": "044525999",
    "receiverBankCorrAccount": "30101810200000000999",
    "amount": 15000.0,
    "purpose": "Оплата по договору №123 от 01.01.2023"
  }
  ```