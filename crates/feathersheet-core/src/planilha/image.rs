//! Configuração de imagens flutuantes na planilha (G11 - image.rs).

#[derive(Debug, Clone, PartialEq)]
pub struct ImagePosition {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImageConfig {
    pub id: String,
    pub src: String,
    pub name: Option<String>,
    pub position: ImagePosition,
    pub opacity: Option<f64>,
    pub aspect_ratio: Option<f64>,
}

impl ImageConfig {
    pub fn new(id: impl Into<String>, src: impl Into<String>, x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            id: id.into(),
            src: src.into(),
            name: None,
            position: ImagePosition { x, y, width, height },
            opacity: None,
            aspect_ratio: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_config() {
        let img = ImageConfig::new("img1", "data:image/png;base64,...", 10.0, 20.0, 300.0, 200.0);
        assert_eq!(img.id, "img1");
        assert_eq!(img.position.width, 300.0);
    }
}
