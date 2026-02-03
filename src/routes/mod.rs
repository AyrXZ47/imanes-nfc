use futures::stream::TryStreamExt;
use mongodb::options::FindOptions;
use tower_cookies::{Cookies, Cookie};
use tokio::time::{sleep, Duration};
use time::OffsetDateTime;
use axum::http::header;
use mongodb::bson::DateTime;
use chrono::{Datelike, Utc, TimeZone};

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

    // Traemos TODOS los imanes para hacer conteo en memoria
    let mut cursor = collection.find(doc! {}, None).await.unwrap();
    let mut all_imanes: Vec<Iman> = Vec::new();
    while let Ok(Some(iman)) = cursor.try_next().await {
        all_imanes.push(iman);
    }
    
    let total_imanes = all_imanes.len();
    let mut activos_total = 0;
    let mut activos_este_mes = 0;
    let mut activos_mes_pasado = 0;
    let mut top_imanes = Vec::new();

    let now = Utc::now();
    let current_month = now.month();
    let current_year = now.year();
    
    // Calculamos mes pasado
    let (prev_month, prev_month_year) = if current_month == 1 {
        (12, current_year - 1)
    } else {
        (current_month - 1, current_year)
    };

    let mut history_counts = vec![0; 6]; // [mes-5, mes-4, ... mes-actual]
    let mut month_labels = Vec::new();
    
    // Generar etiquetas de meses (Ej: "Ago", "Sep", "Oct"...)
    for i in (0..6).rev() {
        let d = Utc::now() - chrono::Duration::days(i * 30);
        month_labels.push(d.format("%b").to_string());
    }

    for iman in &all_imanes {
        
        if iman.active {
            activos_total += 1;
            
            // Checamos fecha de activación
            if let Some(fecha_activacion) = iman.activated_at {
                // Convertimos la fecha de Mongo a Chrono manualmente
                if let Some(fecha) = chrono::DateTime::from_timestamp_millis(fecha_activacion.timestamp_millis()) {
                    if fecha.month() == current_month && fecha.year() == current_year {
                        activos_este_mes += 1;
                    } else if fecha.month() == prev_month && fecha.year() == prev_month_year {
                        activos_mes_pasado += 1;
                    }

                    // Chart logic
                    let months_diff = (now.year() - fecha.year()) * 12 + (now.month() - fecha.month()) as i32;
                    if months_diff >= 0 && months_diff < 6 {
                        // Invertimos el índice porque history_counts[5] es el mes actual
                        let index = 5 - months_diff as usize;
                        history_counts[index] += 1;
                    }
                }
            }
        }

        if iman.visitas > 0 {
            top_imanes.push(iman.clone());
        }
    }

    // Ordenamos el vector para sacar el Top 10 manual
    top_imanes.sort_by(|a, b| b.visitas.cmp(&a.visitas));
    let top_10_raw: Vec<Iman> = top_imanes.into_iter().take(10).collect();
    
    // Convertir a formato amigable para Tera (View Model)
    // Esto evita errores con bson::DateTime en el template
    let top_10_view: Vec<serde_json::Value> = top_10_raw.into_iter().map(|iman| {
        let last_scan_iso = iman.last_scan_at
            .and_then(|dt| chrono::DateTime::from_timestamp_millis(dt.timestamp_millis()))
            .map(|dt| dt.to_rfc3339()); // Convertir a string ISO

        serde_json::json!({
            "codigo": iman.codigo,
            "target_url": iman.target_url,
            "visitas": iman.visitas,
            "last_scan_at": last_scan_iso
        })
    }).collect();

    let virgenes = total_imanes - activos_total;

    // Pasamos datos a la plantilla
    let mut context = tera::Context::new();
    context.insert("total", &total_imanes);
    context.insert("activos", &activos_total);
    context.insert("virgenes", &virgenes);
    context.insert("top_imanes", &top_10_view);
    context.insert("activos_mes", &activos_este_mes);      
    context.insert("activos_mes_ant", &activos_mes_pasado); 
    context.insert("chart_data", &history_counts);
    context.insert("chart_labels", &month_labels);
    
    // Pasamos el dominio base para facilitar la grabación de NFCs
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
            exported: false,
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
    
    // 1. FILTRO: Solo dame los que NO están activos Y NO han sido exportados
    // (O usamos $ne: true para incluir los que no tienen el campo todavía)
    let filter = doc! { 
        "active": false,
        "exported": { "$ne": true } 
    };

    let mut cursor = collection.find(filter.clone(), None).await.unwrap();
    let mut csv_content = String::from("codigo,url_completa\n");
    let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
    let mut count = 0;

    while let Ok(Some(iman)) = cursor.try_next().await {
        let linea = format!("{},{}/v/{}\n", iman.codigo, base_url, iman.codigo);
        csv_content.push_str(&linea);
        count += 1;
    }

    if count == 0 {
        return (StatusCode::OK, "⚠️ No hay imanes nuevos para exportar. Genera un lote primero.").into_response();
    }

    // 2. ACTUALIZACIÓN MASIVA (Atomic Update)
    // Marcamos TODOS los que acabamos de encontrar como exported: true
    // Así la próxima vez, el filtro de arriba ya no los encontrará.
    let update = doc! { "$set": { "exported": true } };
    collection.update_many(filter, update, None).await.ok();

    // 3. Devolver CSV
    (
        [
            (header::CONTENT_TYPE, "text/csv"),
            (header::CONTENT_DISPOSITION, "attachment; filename=\"lote_produccion_nuevo.csv\""),
        ],
        csv_content,
    ).into_response()
}
