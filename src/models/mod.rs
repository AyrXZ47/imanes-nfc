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

    #[serde(skip_serializing_if = "Option::is_none")]
    pub activated_at: Option<DateTime>, 
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_scan_at: Option<DateTime>, 

    #[serde(default)] 
    pub exported: bool, 

    #[serde(skip_serializing_if = "Option::is_none")]
    pub lote_nombre: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub lote_fecha: Option<DateTime>,
}
