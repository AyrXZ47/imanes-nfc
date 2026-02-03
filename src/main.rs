mod db;
mod models;
mod routes;

use axum::{routing::{get, post}, Router, response::Redirect};
use dotenv::dotenv;
use std::sync::Arc;
use tera::Tera; // Importamos Tera
use tower_cookies::CookieManagerLayer;


// Definimos el "Estado Global" de nuestra app
#[derive(Clone)]
pub struct AppState {
    pub db: db::DB,
    pub tera: Arc<Tera>, // Usamos Arc para compartirlo entre hilos
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    // 1. Base de Datos
    let db = match db::init_db().await {
        Ok(d) => d,
        Err(e) => {
            eprintln!("❌ Error DB: {}", e);
            std::process::exit(1);
        }
    };

    // 2. Motor de Plantillas (Tera)
    // Busca todos los archivos en la carpeta "templates" que terminen en .html
    let tera = match Tera::new("templates/**/*.html") {
        Ok(t) => t,
        Err(e) => {
            eprintln!("❌ Error cargando templates: {}", e);
            std::process::exit(1);
        }
    };

    // 3. Crear el Estado Compartido
    let state = AppState {
        db,
        tera: Arc::new(tera),
    };

    // 4. Rutas (Nota: .with_state ahora recibe 'state' completo)
    let app = Router::new()
        .route("/", get(root))
        .route("/v/:codigo", get(routes::redirect_handler))
        .route("/api/setup", post(routes::save_iman))
        .route("/login", get(routes::login_page))
        .route("/auth/login", post(routes::process_login))
        .route("/admin", get(routes::admin_dashboard))
        .route("/api/admin/generate", post(routes::generate_batch))
        .layer(CookieManagerLayer::new()) // ¡Activa cookies!
        .with_state(state);

    // 5. Servidor
    let port = std::env::var("SERVER_PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}", port);
    println!("🚀 Servidor corriendo en http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// Borra la función vieja que devolvía texto y pon esto:
async fn root() -> Redirect {
    // Si entran a la raíz, los mandamos al login directo
    Redirect::to("/login")
}
