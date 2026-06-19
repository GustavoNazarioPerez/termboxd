#[derive(Default, serde::Deserialize)]
#[serde(default, rename_all = "PascalCase")]
pub struct Movie {
    pub date: String,
    pub name: String,
    pub year: i32,
    #[serde(rename = "Letterboxd URI")]
    pub uri: String,
    pub rating: Option<f32>,
    pub review: Option<String>,
}
