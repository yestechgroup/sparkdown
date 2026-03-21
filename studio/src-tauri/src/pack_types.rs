use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeCategoryDto {
    pub pack_name: String,
    pub category_label: String,
    pub types: Vec<TypeOptionDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeOptionDto {
    pub iri: String,
    pub curie: String,
    pub label: String,
    pub description: Option<String>,
}
