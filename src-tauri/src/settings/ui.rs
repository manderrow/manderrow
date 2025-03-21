#[derive(serde::Serialize)]
pub struct Settings {
    pub sections: &'static [Section],
}

#[derive(serde::Serialize)]
pub struct Section {
    pub id: &'static str,
    pub settings: &'static [Setting],
}

#[derive(serde::Serialize)]
pub struct Setting {
    pub key: &'static str,
    pub input: Input,
}

#[derive(serde::Serialize)]
#[serde(tag = "type")]
pub enum Input {
    Toggle,
    Text,
}
