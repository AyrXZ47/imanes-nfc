// src/models/mod.rs
use serde::{Deserialize, Serialize};
use mongodb::bson::oid::ObjectId;
use mongodb::bson::DateTime;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Iman {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub codigo: String,
    pub target_url: Option<String>,
    pub active: bool,
    pub visitas: u32,

    // --- CAMPOS NUEVOS (El Historial) ---
    // Usamos Option porque los imanes viejos no tienen este dato
    // y para no romper la compatibilidad.
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activated_at: Option<DateTime>, // ¿Cuándo se configuró por primera vez?
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_scan_at: Option<DateTime>, // ¿Cuándo fue la última vez que alguien lo tocó?
}
