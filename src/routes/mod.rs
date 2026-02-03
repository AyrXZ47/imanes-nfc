use futures::stream::TryStreamExt;
use mongodb::options::FindOptions;
use axum::extract::Query;

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
    let update = doc! { "$inc": { "visitas": 1 } }; // $inc es "incrementar" en Mongo

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
        Ok(None) => (StatusCode::NOT_FOUND, "❌ Imán no válido").into_response(),
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

    // Validación básica
    if !form.target_url.starts_with("http") {
        return (
            StatusCode::BAD_REQUEST,
            "❌ La URL debe empezar con http:// o https://",
        )
            .into_response();
    }

    let filter = doc! { "codigo": &form.codigo };
    let update = doc! {
        "$set": {
            "target_url": &form.target_url,
            "active": true
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

#[derive(Deserialize)]
pub struct AdminParams {
    pwd: Option<String>,
}

pub async fn admin_dashboard(
    State(state): State<AppState>,
    Query(params): Query<AdminParams>,
) -> Response {
    // 1. SEGURIDAD: Verificar contraseña maestra
    // En producción, esto vendría de una variable de entorno real
    let admin_pass = std::env::var("ADMIN_PASSWORD").unwrap_or("admin123".to_string());
    
    if params.pwd != Some(admin_pass) {
        return (StatusCode::UNAUTHORIZED, "🔒 Acceso Denegado. Contraseña incorrecta.").into_response();
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
