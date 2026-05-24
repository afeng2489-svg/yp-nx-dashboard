use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageManifest {
    pub page_name: String,
    pub routes: Vec<RouteDefinition>,
    pub components: Vec<ComponentSpec>,
    pub data_models: Vec<DataModelSpec>,
    pub imports: Vec<ImportMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteDefinition {
    pub path: String,
    pub component_name: String,
    #[serde(default)]
    pub layout: Option<String>,
    #[serde(default)]
    pub auth_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentSpec {
    pub name: String,
    pub file_path: String,
    #[serde(default)]
    pub props: Vec<String>,
    #[serde(default)]
    pub children: Vec<String>,
    #[serde(default)]
    pub uses_data_model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataModelSpec {
    pub name: String,
    #[serde(default)]
    pub fields: Vec<DataField>,
    #[serde(default)]
    pub methods: Vec<String>,
    #[serde(default)]
    pub mock_data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataField {
    pub name: String,
    pub field_type: String,
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportMapping {
    pub from_module: String,
    pub import_names: Vec<String>,
}
