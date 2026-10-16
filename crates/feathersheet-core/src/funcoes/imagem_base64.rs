//! Imagem ↔ base64, para as fórmulas `IMAGEM` e `IMAGEM.BASE64` (item 12 da
//! fila de 23/09/2026). Espelho de `src/core/imagem-base64.ts`, com os mesmos
//! testes. A imagem na célula é um texto `data:image/…;base64,…`, que a grade
//! desenha no lugar do texto; o tipo vem dos primeiros bytes.

/// O tipo pelos primeiros caracteres do base64 (os primeiros bytes do arquivo).
const ASSINATURAS: &[(&str, &str)] = &[
    ("iVBORw0KGgo", "image/png"),
    ("/9j/", "image/jpeg"),
    ("R0lGOD", "image/gif"),
    ("UklGR", "image/webp"),
    ("PHN2Zy", "image/svg+xml"),
    ("PD94bWwg", "image/svg+xml"),
    ("Qk", "image/bmp"),
];

/// `^[A-Za-z0-9+/]+={0,2}$`
fn so_base64(t: &str) -> bool {
    let corpo = t.trim_end_matches('=');
    t.len() - corpo.len() <= 2 && !corpo.is_empty() && corpo.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/')
}

/// `^data:(image/[a-z0-9.+-]+);base64,(base64)$`, sem diferença de maiúscula no cabeçalho.
fn e_data_url(t: &str) -> bool {
    let Some(virgula) = t.find(',') else { return false };
    let (cabeca, corpo) = (&t[..virgula], &t[virgula + 1..]);
    let cabeca = cabeca.to_ascii_lowercase();
    let Some(tipo) = cabeca.strip_prefix("data:image/").and_then(|x| x.strip_suffix(";base64")) else {
        return false;
    };
    !tipo.is_empty()
        && tipo.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"-.+".contains(&b))
        && so_base64(corpo)
}

/// O data URL da imagem (aceita o próprio data URL ou o base64 puro, com ou
/// sem quebras de linha). `None` quando não parece imagem.
pub fn para_data_url(texto: &str) -> Option<String> {
    let t = texto.trim();
    if e_data_url(t) {
        return Some(t.to_string());
    }
    let b64: String = t.chars().filter(|c| !c.is_whitespace()).collect();
    if !so_base64(&b64) {
        return None;
    }
    let tipo = ASSINATURAS.iter().find(|(a, _)| b64.starts_with(a))?.1;
    Some(format!("data:{tipo};base64,{b64}"))
}

/// O base64 puro de uma imagem (data URL ou base64). `None` quando não é imagem.
pub fn para_base64(texto: &str) -> Option<String> {
    let url = para_data_url(texto)?;
    Some(url[url.find(',')? + 1..].to_string())
}

/// Equivalente a `ehImagemNaCelula` do modelo TypeScript.
/// Apenas URLs de imagem já canônicas, com `data:image/` minúsculo,
/// são exibidas na grade; strings base64 cruas não são tratadas como imagem.
#[inline]
pub fn eh_imagem_na_celula(valor: &str) -> bool {
    valor.starts_with("data:image/") && e_data_url(valor)
}

#[cfg(test)]
mod testes {
    use super::*;

    const PNG: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";

    #[test]
    fn base64_vira_data_url_pelo_tipo() {
        assert_eq!(para_data_url(PNG), Some(format!("data:image/png;base64,{PNG}")));
        assert_eq!(para_data_url("/9j/4AAQSkZJRg==").as_deref(), Some("data:image/jpeg;base64,/9j/4AAQSkZJRg=="));
        let quebrado = format!(" {}\n{} ", &PNG[..20], &PNG[20..]);
        assert_eq!(para_data_url(&quebrado), Some(format!("data:image/png;base64,{PNG}")));
        let url = format!("data:image/png;base64,{PNG}");
        assert_eq!(para_data_url(&url), Some(url.clone()));
        for nao in ["olá", "aGVsbG8=", "data:text/plain;base64,aGVsbG8=", ""] {
            assert_eq!(para_data_url(nao), None, "{nao}");
        }
    }

    #[test]
    fn reconhece_apenas_url_de_imagem_canonica_na_celula() {
        let imagem = format!("data:image/png;base64,{PNG}");
        assert!(eh_imagem_na_celula(&imagem));
        assert!(eh_imagem_na_celula("data:image/jpeg;base64,/9j/4AAQSkZJRg=="));
        assert!(!eh_imagem_na_celula(PNG), "base64 puro ainda não é data URL");
        assert!(!eh_imagem_na_celula("data:text/plain;base64,aGVsbG8="));
        assert!(!eh_imagem_na_celula("DATA:IMAGE/PNG;BASE64,iVBORw0KGgo="));
        assert!(!eh_imagem_na_celula("data:image/png;base64,invalido=!"));
    }

    #[test]
    fn e_volta_o_base64_puro() {
        assert_eq!(para_base64(&format!("data:image/png;base64,{PNG}")).as_deref(), Some(PNG));
        assert_eq!(para_base64(PNG).as_deref(), Some(PNG));
        assert_eq!(para_base64("texto"), None);
    }
}
