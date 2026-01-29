mod models;
// mod db; // Lo haremos mañana
// mod routes; // Lo haremos mañana

use axum::{
    routing::{get, post},
    Router,
    extract::Path,
    response::Html,
};
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    // 1. Logs
    tracing_subscriber::fmt::init();

    // 2. Rutas
    let app = Router::new()
        .route("/", get(root))
        .route("/v/:codigo", get(handle_redirect)); // La ruta mágica del NFC

    // 3. Server
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("🚀 Servidor corriendo en http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn root() -> &'static str {
    "Souvenir Backend v1.0 - ¡Funcionando!"
}

// Mockup de la lógica de redirección (Mañana la conectamos a Mongo)
async fn handle_redirect(Path(codigo): Path<String>) -> Html<String> {
    // AQUÍ IRÁ LA LÓGICA REAL:
    // 1. Buscar 'codigo' en Mongo.
    // 2. Si tiene url -> Redirect(307, url).
    // 3. Si no tiene url -> Render(setup.html).
    
    Html(format!("<h1>Escaneaste el imán: {}</h1><p>Aquí iría el formulario si estuviera vacío, o el video si ya tuviera dueño.</p>", codigo))
}
