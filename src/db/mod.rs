// src/db/mod.rs
use mongodb::{options::ClientOptions, Client, Database};
use std::env;
use std::sync::Arc;

// DB será nuestro tipo "alias" para compartir la base de datos de forma segura
pub type DB = Arc<Database>;

pub async fn init_db() -> Result<DB, mongodb::error::Error> {
    // 1. Leemos la URL del .env
    let client_uri = env::var("MONGO_URI").expect("❌ No se encontró MONGO_URI en el .env");

    // 2. Parseamos la URL
    let mut client_options = ClientOptions::parse(client_uri).await?;
    
    // Configuración opcional
    client_options.app_name = Some("SouvenirBackend".to_string());

    // 3. Creamos el cliente
    let client = Client::with_options(client_options)?;
    
    // 4. Obtenemos la base de datos. 
    // OJO: Si en el .env pusiste ".../souvenir_db?...", Mongo usará esa por defecto.
    // Si no, aquí forzamos el nombre "souvenir_db".
    let db = client.database("souvenir_db");

    println!("✅ Conexión a MongoDB exitosa");

    Ok(Arc::new(db))
}
