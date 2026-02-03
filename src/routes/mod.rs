use futures::stream::TryStreamExt;
use mongodb::options::FindOptions;
use tower_cookies::{Cookies, Cookie};
use tokio::time::{sleep, Duration};
use time::OffsetDateTime;
use axum::http::header;
use mongodb::bson::DateTime;

use axum::{
    extract::{Form, Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use mongodb::{
    bson::doc,
    options::{FindOneAndUpdateOptions, ReturnDocument},
};
use serde::Deserialize;

use crate::{models::Iman, AppState};

#[derive(Deserialize)]
pub struct SetupForm {
    codigo: String,
    target_url: String,
}

// Maneja GET /v/:codigo
pub async fn redirect_handler(
    State(state): State<AppState>,
    Path(codigo): Path<String>,
) -> Response {
    let collection = state.db.collection::<Iman>("imanes");

    // LÓGICA PRO: "Busca y actualiza" atómicamente
    // Si encuentra el código, le suma 1 a "visitas" automáticamente.
    let filter = doc! { "codigo": &codigo };
    let update = doc! { 
        "$inc": { "visitas": 1 },
        "$set": { "last_scan_at": DateTime::now() } 
    };


    // Opciones: Queremos el documento *después* de actualizarse
    let options = FindOneAndUpdateOptions::builder()
        .return_document(ReturnDocument::After)
        .build();

    match collection
        .find_one_and_update(filter, update, options)
        .await
    {
        Ok(Some(iman)) => {
           // ¡Encontrado y contador actualizado! ✅
            if iman.active {
                if let Some(url) = iman.target_url {
                    if !url.is_empty() {
                        // Si tiene URL -> Redirigir
                        return Redirect::temporary(&url).into_response();
                    }
                }
            }

            // Si no está activo o no tiene URL -> Renderizar Setup
            // (Nota: Si quieres que cuente visitas incluso si no está configurado, déjalo así.
            //  Si solo quieres contar cuando ya redirige, mueve la lógica de update dentro del if).
            
            let mut context = tera::Context::new();
            context.insert("codigo", &codigo);

            match state.tera.render("setup.html", &context) {
                Ok(html) => Html(html).into_response(),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Template error: {}", e),
                )
                    .into_response(),
            }
        }
        Ok(None) => {
            // CASO B: El imán NO existe -> Mostrar plantilla 404 bonita
            let context = tera::Context::new(); 
            // (Podrías pasar variables si quisieras, por ahora vacío)
            
            match state.tera.render("404.html", &context) {
                Ok(html) => (StatusCode::NOT_FOUND, Html(html)).into_response(),
                Err(_) => (StatusCode::NOT_FOUND, "❌ Imán no válido").into_response()
            }
        },
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("DB error: {}", e),
        )
            .into_response(),
    }
}

// Maneja POST /api/setup
pub async fn save_iman(
    State(state): State<AppState>,
    Form(form): Form<SetupForm>,
) -> Response {
    let collection = state.db.collection::<Iman>("imanes");

    // 1. LIMPIEZA DE INPUT (Sanitization)
    let url_limpia = form.target_url.trim();

    // 2. VALIDACIÓN DE SEGURIDAD (Anti-Porno / Anti-Phishing básico)
    // Forzamos HTTPS y bloqueamos dominios raros si quisieras
    if !url_limpia.starts_with("https://") {
         return (StatusCode::BAD_REQUEST, "❌ Por seguridad, solo aceptamos enlaces seguros (https://)").into_response();
    }
    
    // Validación extra: Que parezca una URL real de redes sociales (Opcional pero recomendado)
    // Esto evita que pongan "https://mi-sitio-de-virus.com"
    let dominios_permitidos = ["instagram.com", "tiktok.com", "facebook.com", "youtube.com", "twitter.com", "x.com"];
    let es_seguro = dominios_permitidos.iter().any(|d| url_limpia.contains(d));

    if !es_seguro {
        // OJO: Puedes quitar esto si quieres permitir cualquier web, 
        // pero dejarlo reduce riesgo de sitios maliciosos.
        return (StatusCode::BAD_REQUEST, "⚠️ Por ahora solo permitimos redes sociales reconocidas (TikTok, Instagram, Youtube, etc).").into_response();
    }

    // 3. ACTUALIZACIÓN EN MONGO
    let filter = doc! { "codigo": &form.codigo };

    let update = doc! { 
        "$set": {
            "target_url": url_limpia,
            "active": true,
            "activated_at": DateTime::now(), // ¡Marca de tiempo actual!
            "last_scan_at": DateTime::now()  // También cuenta como primer scan
        }
    };

    match collection.update_one(filter, update, None).await {
        Ok(_) => {
            let ruta_magica = format!("/v/{}", form.codigo);
            Redirect::to(&ruta_magica).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error guardando: {}", e),
        )
            .into_response(),
    }
}

// Estructura para recibir el form del login
#[derive(Deserialize)]
pub struct LoginForm {
    password: String,
}

// 1. Mostrar pantalla de Login (GET /login)
pub async fn login_page(State(state): State<AppState>) -> Response {
    let context = tera::Context::new();
    Html(state.tera.render("login.html", &context).unwrap()).into_response()
}

// 2. Procesar Login (POST /auth/login)
pub async fn process_login(
    cookies: Cookies,
    Form(form): Form<LoginForm>,
) -> Response {
    let admin_pass = std::env::var("ADMIN_PASSWORD").unwrap_or("admin123".to_string());

    if form.password == admin_pass {
        let mut cookie = Cookie::new("admin_session", "activa");
        cookie.set_path("/");
        cookie.set_http_only(true);
        cookie.set_secure(true); // Solo viaja por HTTPS
        
        // ⏱️ AQUÍ ESTÁ LA MAGIA: La sesión muere en 1 hora (3600 segundos)
        // El navegador la borrará automáticamente aunque no cierren la ventana.
        cookie.set_max_age(time::Duration::hours(1)); 
        
        cookies.add(cookie);

        Redirect::to("/admin").into_response()
    } else {
        // 🛡️ DEFENSA CONTRA FUERZA BRUTA
        // Hacemos esperar al atacante 2 segundos artificialmente
        sleep(Duration::from_secs(2)).await;
        
        // Contraseña incorrecta: Volver al login con error
        Redirect::to("/login?error=1").into_response()
    }
}

// 3. Modifica tu dashboard para usar COOKIES en vez de ?pwd
pub async fn admin_dashboard(
    cookies: Cookies, // <--- Inyectamos Cookies
    State(state): State<AppState>,
) -> Response {
    
    // VERIFICACIÓN DE SEGURIDAD
    let auth_cookie = cookies.get("admin_session");
    if auth_cookie.is_none() {
        // Si no hay cookie, ¡fuera! Al login.
        return Redirect::to("/login").into_response();
    }
    
    let collection = state.db.collection::<Iman>("imanes");

    // 2. MÉTRICAS GENERALES (KPIs)
    // Total de imanes fabricados (registrados en DB)
    let total_imanes = collection.count_documents(doc! {}, None).await.unwrap_or(0);
    
    // Imanes "Vivos" (Ya comprados y configurados)
    let filter_activos = doc! { "active": true };
    let imanes_activos = collection.count_documents(filter_activos, None).await.unwrap_or(0);

    // Imanes "En Stock" (Aun vírgenes)
    let imanes_virgenes = total_imanes - imanes_activos;

    // 3. TOP 10 VIRALES (Los más escaneados)
    let find_options = FindOptions::builder()
        .sort(doc! { "visitas": -1 }) // Orden descendente (Mayor a menor)
        .limit(10)
        .build();

    let mut cursor = collection.find(doc! { "visitas": { "$gt": 0 } }, find_options).await.unwrap();
    
    let mut top_imanes = Vec::new();
    while let Ok(Some(iman)) = cursor.try_next().await {
        top_imanes.push(iman);
    }

    // 4. RENDERIZAR EL DASHBOARD
    let mut context = tera::Context::new();
    context.insert("total", &total_imanes);
    context.insert("activos", &imanes_activos);
    context.insert("virgenes", &imanes_virgenes);
    context.insert("top_imanes", &top_imanes);
    
    // Pasamos el dominio base para facilitar la grabación de NFCs
    // (Esto debería venir de ENV, pero por ahora lo calculamos o hardcodeamos)
    context.insert("base_url", "https://imanes-nfc-production.up.railway.app"); 

    match state.tera.render("admin.html", &context) {
        Ok(html) => Html(html).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
    }
}



// POST /api/admin/generate_batch
pub async fn generate_batch(
    cookies: Cookies,
    State(state): State<AppState>,
) -> Response {
    // 1. Seguridad (Cookie Check)
    if cookies.get("admin_session").is_none() {
        return Redirect::to("/login").into_response();
    }

    let collection = state.db.collection::<Iman>("imanes");
    let mut docs = Vec::new();
    
    // Usamos un prefijo y números aleatorios para evitar colisiones
    // (Simple implementation)
    // O usa timestamp simple:
    let lote_id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();

    for i in 1..=50 {
        let codigo = format!("LOTE{}-NUM{:03}", lote_id, i); // Ej: LOTE17823-NUM001
        
        docs.push(Iman {
            id: None,
            codigo,
            target_url: None, // Vírgen
            active: false,    // Inactivo hasta que se configure (o true si quieres que ya funcionen)
            visitas: 0,
            activated_at: None,
            last_scan_at: None,
        });
    }

    // Insertar en Mongo
    if let Err(e) = collection.insert_many(docs, None).await {
         return (StatusCode::INTERNAL_SERVER_ERROR, format!("Error DB: {}", e)).into_response();
    }

    // Redirigir al dashboard con mensaje de éxito
    Redirect::to("/admin").into_response()
}


// GET /auth/logout
pub async fn logout(cookies: Cookies) -> Response {
    // Creamos una cookie vacía con fecha de expiración en el pasado
    let mut cookie = Cookie::new("admin_session", "");
    cookie.set_path("/");
    cookie.set_expires(OffsetDateTime::now_utc() - time::Duration::days(1));
    cookies.add(cookie);

    Redirect::to("/login").into_response()
}


// GET /api/admin/export_csv
pub async fn export_csv(
    cookies: Cookies,
    State(state): State<AppState>,
) -> Response {
    if cookies.get("admin_session").is_none() {
        return Redirect::to("/login").into_response();
    }

    let collection = state.db.collection::<Iman>("imanes");
    
    // Buscamos solo los que NO tienen URL (los vírgenes para producción)
    // OJO: Ajusta el filtro según tu lógica. Aquí busco los que active: false
    let filter = doc! { "active": false };
    let mut cursor = collection.find(filter, None).await.unwrap();

    let mut csv_content = String::from("codigo,url_completa\n"); // Encabezado CSV

    // Base URL (Hardcodeada o de ENV, por ahora la ponemos fija para producción)
    let base_url = "https://imanes-nfc-production.up.railway.app/v/";

    while let Ok(Some(iman)) = cursor.try_next().await {
        let linea = format!("{},{}{}\n", iman.codigo, base_url, iman.codigo);
        csv_content.push_str(&linea);
    }

    // Devolver como archivo descargable
    (
        [
            (header::CONTENT_TYPE, "text/csv"),
            (header::CONTENT_DISPOSITION, "attachment; filename=\"lote_imanes.csv\""),
        ],
        csv_content,
    ).into_response()
}
