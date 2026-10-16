//! Aparar espaços, bordas e casas decimais.

use crate::celulas::estilo_parcial::{Campo, EstiloParcial};
use crate::celulas::refs::Rect;
use crate::planilha::sheet::Sheet;
use super::comum::{espaco_js, trim_js};

/// `trimSpaces`: tira os espaços das pontas e junta os repetidos no meio,
/// nas células de texto (fórmula fica). O estilo fica — `set_cell` o
/// preserva, como o `styleFor`. Devolve quantas mudaram.
pub fn trim_spaces(sheet: &mut Sheet, area: Rect) -> usize {
    let r = area.normalized();
    let max_r = r.r1.min(sheet.max_row_ts());
    // Além da última coluna usada não há texto: não muda o resultado, e
    // numa linha inteira evita 16 mil colunas vazias por linha.
    let c1 = r.c1.min(sheet.max_col_ts());
    let mut mudaram = 0;
    sheet.abrir_transacao();
    for row in r.r0..=max_r {
        for col in r.c0..=c1 {
            let raw = sheet.get_raw(row, col);
            if raw.is_empty() || raw.starts_with('=') {
                continue;
            }
            let mut junto = String::with_capacity(raw.len());
            let mut no_espaco = false;
            for ch in trim_js(&raw).chars() {
                if espaco_js(ch) {
                    if !no_espaco {
                        junto.push(' ');
                    }
                    no_espaco = true;
                } else {
                    junto.push(ch);
                    no_espaco = false;
                }
            }
            if junto != raw {
                sheet.set_cell(row, col, &junto);
                mudaram += 1;
            }
        }
    }
    sheet.fechar_transacao();
    mudaram
}

/// `applyBorders`. `kind` é o nome do TypeScript (`all`, `outer`, `none`,
/// `top`, `bottom`, `left`, `right`, `inner-horizontal`, `inner-vertical`).
/// Cada célula guarda os seus quatro lados, então "externas" só marca o lado
/// que dá para fora do retângulo. Vai até a linha 1000 mesmo em planilha
/// menor, como lá.
pub fn apply_borders(sheet: &mut Sheet, area: Rect, kind: &str, color: Option<&str>) {
    let r = area.normalized();
    let max_r = r.r1.min(sheet.max_row_ts().max(1000));
    sheet.abrir_transacao();
    for row in r.r0..=max_r {
        for col in r.c0..=r.c1 {
            let (top, bottom, left, right) = (row == r.r0, row == max_r, col == r.c0, col == r.c1);
            let lados: Option<Vec<(&str, bool)>> = match kind {
                "all" => Some(vec![("top", true), ("right", true), ("bottom", true), ("left", true)]),
                "outer" => Some(vec![("top", top), ("right", right), ("bottom", bottom), ("left", left)]),
                "top" => Some(vec![("top", top)]),
                "bottom" => Some(vec![("bottom", bottom)]),
                "left" => Some(vec![("left", left)]),
                "right" => Some(vec![("right", right)]),
                "inner-horizontal" => Some(vec![("bottom", !bottom)]),
                "inner-vertical" => Some(vec![("right", !right)]),
                _ => None,
            };
            let bordas = lados.and_then(|lados| {
                // Só grava os lados marcados: o resto fica como estava.
                let mut b: std::collections::BTreeMap<String, Campo> =
                    lados.into_iter().filter(|&(_, v)| v).map(|(k, _)| (k.to_string(), Campo::Logico(true))).collect();
                if let Some(cor) = color {
                    b.insert("color".into(), Campo::Texto(cor.to_string()));
                }
                (!b.is_empty() || color.is_some()).then_some(b)
            });
            let mut patch = EstiloParcial::new();
            patch.insert("borders".into(), bordas.map_or(Campo::Nulo, Campo::Objeto));
            sheet.set_style(&Rect::new(row, col, row, col), &patch);
        }
    }
    sheet.fechar_transacao();
}

/// `stepDecimals`: mais ou menos casas decimais, a partir das da primeira
/// célula. Texto e data não têm casas: passam a número.
pub fn step_decimals(sheet: &mut Sheet, area: Rect, step: i32) {
    let r = area.normalized();
    let estilo = sheet.estilo_mesclado(r.r0, r.c0);
    let atual = crate::celulas::style::decimals_of(estilo.as_ref().map(crate::celulas::estilo_parcial::para_style).as_ref()) as i32;
    let proxima = (atual + step).clamp(0, 10);
    let coluna_inteira = r.r0 == 0 && r.r1 >= 9_999_999;
    let alvo = if coluna_inteira { r } else { Rect::new(r.r0, r.c0, r.r1.min(sheet.max_row_ts().max(1000)), r.c1) };
    let formato = match crate::celulas::estilo_parcial::formato(estilo.as_ref()) {
        Some(f) if f != "auto" => f,
        _ => "number".to_string(),
    };
    let mut patch = EstiloParcial::new();
    patch.insert("decimals".into(), Campo::Numero(f64::from(proxima)));
    patch.insert("format".into(), Campo::Texto(formato));
    sheet.set_style(&alvo, &patch);
}

