<div align="center">

<a id="top"></a>

# 🧱 Substrate Data

### Одна модель данных, текст, бинарь, кадры и схемы: для файлов, сохранений и сообщений программы

[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)
[![Rust 1.92+](https://img.shields.io/badge/Rust-1.92%2B-orange.svg)](Cargo.toml)
[![crates.io](https://img.shields.io/crates/v/substrate-data.svg)](https://crates.io/crates/substrate-data)

🇺🇸 [English](README.md) | 🇷🇺 **Русский**

[Обзор](#-обзор) • [Подключение](#-подключение) • [Архитектура](#-архитектура) • [Разработка](#-разработка) • [Лицензия](#-лицензия)

</div>

---

## 📖 Обзор

Формат описывается один раз как тип Rust. Его можно читать из текста, написанного руками (каждая ошибка со строкой и путём), править на месте без потери комментариев, компилировать в компактный бинарь, передавать по сокету поле за полем и описывать редактору, который строит по нему таблицу свойств. Библиотека не знает ни о каком продукте: типы предметной области (пользователи, документы, файлы, проекты) живут в коде тех, кто её использует.

| Часть | Где | Что даёт |
|---|---|---|
| Модель | [`substrate-data`](crates/substrate-data) | значения JSON и типизированные ссылки `@kind:id`, байты |
| Текст | [`substrate-data`](crates/substrate-data) | JSON с комментариями, запятыми в конце и ключами без кавычек; позиция каждого значения; правка на месте |
| Бинарь | [`substrate-data`](crates/substrate-data) | компактная самоописывающаяся форма |
| Кадры | [`substrate-data`](crates/substrate-data) | упакованная форма для сообщений, схему которых знают обе стороны |
| Схемы | [`substrate-data`](crates/substrate-data) | формат как тип: чтение с путём каждой проблемы, запись, описание для редакторов |
| Derive | [`substrate-data-derive`](crates/substrate-data-derive) | `#[derive(Schema)]` |

Спецификация формата: [docs/format.md](docs/format.md) (на английском). Что можно менять в типе между версиями (проверено тестами): [docs/evolution.md](docs/evolution.md) (на английском). Независимые от языка тестовые векторы для портов: [conformance/](conformance/README.md).

## 🚀 Подключение

```toml
# Cargo.toml
[dependencies]
substrate-data = "0.4"
```

Признаки `substrate-data`: `json` (JSON ⇄ модель), `toml` (чтение TOML в модель с позициями), `tokio` (кадры поверх асинхронных потоков, `io::asynchronous`).

```rust
use substrate_data::{Id, Schema};

substrate_data::kind!(File = "file", User = "user": u64);

/// A file attached to something.
#[derive(Schema, Debug, PartialEq)]
struct Attachment {
    /// What is attached.
    file: Id<File>,
    #[data(default)]
    owner: Option<Id<User>>,
    #[data(range = "0..=16", default)]
    priority: u8,
}

let attachment: Attachment = substrate_data::from_text(r#"{
    // The logo for the front page.
    file: "@file:images/logo.png",
    owner: "@user:42",
}"#)
.unwrap()
.value;
assert_eq!(attachment.priority, 0);

let bytes = substrate_data::to_bytes(&attachment); // compact, self-describing
let frame = substrate_data::frame::to_frame(&attachment); // packed, schema-known
let shape = Attachment::shape().describe(); // a typed ShapeDoc, for editors and docs
```

## 🏛️ Архитектура

Оба крейта относятся к общему ядру (контекст `data`): они не зависят от других крейтов проекта.

```text
substrate-data-derive   #[derive(Schema)]: разбирает тип и порождает реализацию
        ▲
substrate-data          модель → текст / бинарь / кадры → схемы и диагностика
```

`substrate-data` строится от модели к формам: текст, бинарь и кадры читают и пишут одну и ту же модель, схема поверх неё даёт типизированное чтение с диагностикой. Подробности (грамматика текста, теги бинаря, раскладка кадров, ключи описания формы, коды диагностики) приведены в [docs/format.md](docs/format.md).

## 🛠️ Разработка

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo bench --no-run
python3 <craftbook>/tools/craftcheck check . --baseline .craftcheck-baseline
python3 <craftbook>/tools/craftcheck deps . --baseline .craftcheck-deps-baseline
```

Стандарт разработки: [Craftbook](https://github.com/F000NKKK/craftbook). Изменения: [CHANGELOG.md](CHANGELOG.md).

## 📄 Лицензия

На ваш выбор: [Apache License 2.0](LICENSE-APACHE) или [лицензия MIT](LICENSE-MIT).

---

<div align="center">

**[⬆ Наверх](#top)**

</div>
