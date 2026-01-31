// src/models/mod.rs
use serde::{Deserialize, Serialize};
use mongodb::bson::oid::ObjectId;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Iman {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub codigo: String,             // Ejemplo: "TAMA-001"
    pub target_url: Option<String>, // Ejemplo: "https://tiktok.com/..."
    pub active: bool,               // true si ya tiene dueño
    pub visitas: u32,               // Contador de scans
}
