# 🧲 NFC Tag Manager (Rust Backend)

A high-performance backend system designed to manage programmable NFC tags. Built with **Rust** for safety and speed, utilizing **MongoDB** for atomic data persistence.

## 🚀 Key Features

* **Dynamic Redirection:** Handles NFC scans with near-zero latency, redirecting users to dynamic target URLs (TikTok, Instagram, etc.).
* **Real-Time Analytics:** Uses MongoDB atomic operations (`find_one_and_update`) to track scan counts accurately without race conditions.
* **Self-Provisioning:** "Virgin" tags render a Server-Side Rendered (SSR) HTML interface (via **Tera**) allowing users to claim and configure the tag on the fly.
* **Containerized:** Fully Dockerized with a multi-stage build process for minimal image size (Debian Slim).

## 🛠 Tech Stack

* **Language:** Rust (Edition 2021)
* **Framework:** Axum (Web), Tokio (Async Runtime)
* **Database:** MongoDB Atlas (NoSQL)
* **Templating:** Tera (Jinja2-like) + Tailwind CSS via CDN
* **Infrastructure:** Docker & Railway/Fly.io ready

## 📦 Installation

1. Clone the repo
2. Create a `.env` file with your `MONGO_URI`
3. Run `cargo run`
