//! Gráficos que vieram do `.xlsx` (`_excel` no JSON do gráfico, lido por
//! `feathersheet-io::xlsx_read::graficos_ancora::estilo_excel`): o SVG do
//! motor comum ganha o fundo do Excel (sólido, degradê ou transparente), o
//! contorno fino da área do gráfico e as cores de texto do arquivo.
use serde_json::{Map, Value};

fn cor_svg(hex: &str) -> (String, f32) {
    let h = hex.trim().trim_start_matches('#');
    let alfa = if h.len() == 8 { u8::from_str_radix(&h[6..8], 16).map_or(1.0, |a| f32::from(a) / 255.0) } else { 1.0 };
    (format!("#{}", &h[..h.len().min(6)]), alfa)
}

/// O fundo (com o contorno) em SVG, para o retângulo `w` x `h`.
fn fundo(excel: &Map<String, Value>, w: f32, h: f32) -> String {
    let mut defs = String::new();
    let preenchimento = match excel.get("fundoDegrade").and_then(Value::as_array).filter(|p| p.len() >= 2) {
        Some(paradas) => {
            // Ângulo do Office: 0 = esquerda→direita, 90 = cima→baixo.
            let ang = excel.get("fundoAngulo").and_then(Value::as_f64).unwrap_or(90.0).to_radians();
            let (dx, dy) = (ang.cos() / 2.0, ang.sin() / 2.0);
            defs.push_str(&format!(
                "<defs><linearGradient id=\"fundo-excel\" x1=\"{:.3}\" y1=\"{:.3}\" x2=\"{:.3}\" y2=\"{:.3}\">",
                0.5 - dx, 0.5 - dy, 0.5 + dx, 0.5 + dy
            ));
            let mut paradas: Vec<&Value> = paradas.iter().collect();
            paradas.sort_by(|a, b| a[0].as_f64().unwrap_or(0.0).total_cmp(&b[0].as_f64().unwrap_or(0.0)));
            for p in paradas {
                let pos = p[0].as_f64().unwrap_or(0.0);
                let (cor, alfa) = cor_svg(p[1].as_str().unwrap_or("#ffffff"));
                defs.push_str(&format!("<stop offset=\"{pos:.3}\" stop-color=\"{cor}\" stop-opacity=\"{alfa:.3}\"/>"));
            }
            defs.push_str("</linearGradient></defs>");
            "fill=\"url(#fundo-excel)\"".to_string()
        }
        None => match excel.get("fundo").and_then(Value::as_str) {
            Some(c) => {
                let (cor, alfa) = cor_svg(c);
                format!("fill=\"{cor}\" fill-opacity=\"{alfa:.3}\"")
            }
            None => "fill=\"none\"".to_string(),
        },
    };
    let borda = match excel.get("contorno").and_then(Value::as_str) {
        Some(c) => {
            let (cor, alfa) = cor_svg(c);
            format!(" stroke=\"{cor}\" stroke-opacity=\"{alfa:.3}\" stroke-width=\"1\"")
        }
        None => String::new(),
    };
    format!("{defs}<rect x=\"0.5\" y=\"0.5\" width=\"{}\" height=\"{}\" {preenchimento}{borda}/>", (w - 1.0).max(0.0), (h - 1.0).max(0.0))
}

/// Põe o fundo do Excel no lugar do fundo transparente que o
/// `preparar_svg_em` desenhou e troca a cor do texto.
pub(super) fn vestir(mut svg: String, excel: &Map<String, Value>, w: f32, h: f32) -> String {
    if let Some(i) = svg.find("<rect width=") {
        if let Some(fim) = svg[i..].find("/>") {
            svg.replace_range(i..i + fim + 2, &fundo(excel, w, h));
        }
    }
    svg = vestir_textos(&svg, excel);
    if let Ok(caminho) = std::env::var("FEATHERSHEET_DEPURAR_SVG") {
        let _ = std::fs::write(format!("{caminho}-{}x{}.svg", w as u32, h as u32), &svg);
    }
    svg
}

/// Atributo `nome="..."` de uma tag.
fn atributo<'a>(tag: &'a str, nome: &str) -> Option<&'a str> {
    let chave = format!(" {nome}=\"");
    let i = tag.find(&chave)? + chave.len();
    tag[i..].split('"').next()
}

/// Troca ou acrescenta um atributo na tag.
fn com_atributo(tag: &str, nome: &str, valor: &str) -> String {
    let chave = format!(" {nome}=\"");
    match tag.find(&chave) {
        Some(p) => {
            let ini = p + chave.len();
            let fim = tag[ini..].find('"').map_or(tag.len(), |f| ini + f);
            format!("{}{valor}{}", &tag[..ini], &tag[fim..])
        }
        None => format!("{} {nome}=\"{valor}\"{}", &tag[..5], &tag[5..]),
    }
}

/// Cor dos textos pelo papel de cada um no desenho do motor: rótulo do
/// eixo de categorias (a linha de baixo, centrado), do eixo de valores
/// (alinhado à direita; some com o eixo apagado), rótulo de dado (centrado
/// acima) e o resto (legenda).
fn vestir_textos(svg: &str, excel: &Map<String, Value>) -> String {
    let cor = |k: &str| excel.get(k).and_then(Value::as_str).map(|c| cor_svg(c).0);
    let geral = cor("corTexto");
    let eixo_valores = cor("corEixoValores").or_else(|| geral.clone());
    let rotulo = cor("corRotulo").or_else(|| geral.clone());
    let negrito = excel.get("negritoRotulo").and_then(Value::as_bool).unwrap_or(false);
    let mostrar_valores = excel.get("eixoValores").and_then(Value::as_bool).unwrap_or(true);
    let tags: Vec<(usize, usize)> = {
        let mut v = Vec::new();
        let mut i = 0;
        while let Some(p) = svg[i..].find("<text") {
            let ini = i + p;
            let fim = svg[ini..].find("</text>").map_or(svg.len(), |f| ini + f + 7);
            v.push((ini, fim));
            i = fim;
        }
        v
    };
    let y = |t: &str| atributo(t, "y").and_then(|v| v.parse::<f32>().ok()).unwrap_or(0.0);
    let base = tags.iter().map(|&(a, b)| &svg[a..b])
        .filter(|t| atributo(t, "text-anchor") == Some("middle") && !t.contains("rotate("))
        .map(y).fold(f32::MIN, f32::max);
    let mut saida = String::with_capacity(svg.len());
    let mut ultimo = 0;
    for (a, b) in tags {
        saida.push_str(&svg[ultimo..a]);
        ultimo = b;
        let texto = &svg[a..b];
        let fecha = texto.find('>').unwrap_or(texto.len());
        let (tag, resto) = texto.split_at(fecha);
        let ancora = atributo(tag, "text-anchor");
        let (papel_cor, rotulo_dado) = match ancora {
            Some("end") if !mostrar_valores => continue,
            Some("end") => (eixo_valores.clone(), false),
            Some("middle") if (y(tag) - base).abs() < 0.5 || tag.contains("rotate(") => (geral.clone(), false),
            Some("middle") => (rotulo.clone(), true),
            _ => (geral.clone(), false),
        };
        let mut nova = tag.to_string();
        if let Some(c) = papel_cor { nova = com_atributo(&nova, "fill", &c); }
        if rotulo_dado && negrito { nova = com_atributo(&nova, "font-weight", "bold"); }
        saida.push_str(&nova);
        saida.push_str(resto);
    }
    saida.push_str(&svg[ultimo..]);
    saida
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn fundo_em_degrade_contorno_e_texto() {
        let excel: Map<String, Value> = serde_json::from_value(serde_json::json!({
            "fundoDegrade": [[0.0, "#000000"], [1.0, "#2f559700"]], "fundoAngulo": 270.0,
            "contorno": "#898989", "corTexto": "#bfbfbf", "corRotulo": "#ffffff", "negritoRotulo": true, "eixoValores": false
        })).unwrap();
        let svg = concat!(
            "<svg><rect width=\"10\" height=\"5\" fill=\"#000000\" fill-opacity=\"0.000\"/>",
            "<text x=\"1\" y=\"9\" text-anchor=\"middle\" fill=\"#333\">mar</text>",
            "<text x=\"1\" y=\"3\" text-anchor=\"middle\" fill=\"#333\">10</text>",
            "<text x=\"1\" y=\"3\" text-anchor=\"end\" fill=\"#333\">R$ 10</text>",
            "<text y=\"2\">b</text></svg>").to_string();
        let v = vestir(svg, &excel, 10.0, 5.0);
        assert!(v.contains("linearGradient id=\"fundo-excel\" x1=\"0.500\" y1=\"1.000\" x2=\"0.500\" y2=\"0.000\""), "{v}");
        assert!(v.contains("stop-color=\"#2f5597\" stop-opacity=\"0.000\""));
        assert!(v.contains("stroke=\"#898989\""));
        assert!(v.contains("text-anchor=\"middle\" fill=\"#bfbfbf\">mar</text>"), "{v}");
        assert!(v.contains("<text font-weight=\"bold\" x=\"1\" y=\"3\" text-anchor=\"middle\" fill=\"#ffffff\">10</text>"), "{v}");
        assert!(!v.contains("R$ 10"), "eixo de valores apagado: {v}");
        assert!(v.contains("<text fill=\"#bfbfbf\" y=\"2\">b</text>"), "{v}");
        assert!(!v.contains("fill-opacity=\"0.000\"/>"));
    }
}
