//! O estilo como o TypeScript o guarda: campo a campo, com ausência e valor
//! distintos.
//!
//! O [`Style`] do `style.rs` é o estilo **resolvido**, pronto para formatar:
//! `bold: false` e "sem negrito" são a mesma coisa lá, e para desenhar isso
//! basta. Para GUARDAR não basta. No `style.ts` o `mergeStyle` empilha a
//! aba, a coluna, a linha e a célula, e um `{ bold: false }` na célula
//! **desliga** o negrito que a coluna ligou — enquanto a ausência de `bold`
//! deixa passar o da coluna. Um `Style` com `bool` perderia essa diferença, e
//! a célula que a pessoa tirou do negrito voltaria negrita ao reabrir.
//!
//! Então o que mora na planilha é isto: um mapa de nome de campo para valor,
//! do jeito do objeto JavaScript, e o [`Style`] é derivado dele quando alguém
//! vai formatar.
//!
//! Não há `serde_json` aqui — o `sheet-core` não ganha dependência (regra 11
//! do `CLAUDE.md`). Quem fala JSON é a camada de comandos, que converte.

use std::collections::BTreeMap;

use crate::celulas::style::{Align, Borders, NumberFormat, Style, VAlign};

/// Um valor de campo de estilo. É o pedaço de JSON que um estilo usa.
#[derive(Debug, Clone, PartialEq)]
pub enum Campo {
    /// No patch, apaga o campo — é o `undefined` do `mergeStyle`, que não
    /// sobrevive ao JSON e por isso chega aqui como `null`.
    Nulo,
    Logico(bool),
    Numero(f64),
    Texto(String),
    /// As bordas: um objeto dentro do estilo, trocado inteiro no merge (o
    /// `mergeStyle` do TypeScript é raso).
    Objeto(BTreeMap<String, Campo>),
}

/// O estilo guardado. Ordenado para que dois estilos iguais comparem iguais
/// e o dicionário da aba não guarde repetido.
pub type EstiloParcial = BTreeMap<String, Campo>;

/// `mergeStyle(base, patch)`: o patch por cima, e `Nulo` apaga.
pub fn mesclar(base: Option<&EstiloParcial>, patch: &EstiloParcial) -> EstiloParcial {
    let mut saida = base.cloned().unwrap_or_default();
    for (k, v) in patch {
        if matches!(v, Campo::Nulo) {
            saida.remove(k);
        } else {
            saida.insert(k.clone(), v.clone());
        }
    }
    saida
}

/// `hasStyle`: tem algum campo.
pub fn tem_estilo(e: Option<&EstiloParcial>) -> bool {
    e.is_some_and(|e| !e.is_empty())
}

fn logico(e: &EstiloParcial, k: &str) -> bool {
    matches!(e.get(k), Some(Campo::Logico(true)))
}

fn texto(e: &EstiloParcial, k: &str) -> Option<String> {
    match e.get(k) {
        Some(Campo::Texto(t)) => Some(t.clone()),
        _ => None,
    }
}

fn numero(e: &EstiloParcial, k: &str) -> Option<f64> {
    match e.get(k) {
        Some(Campo::Numero(n)) => Some(*n),
        _ => None,
    }
}

/// O formato de número pelo nome que o TypeScript usa.
pub fn formato_por_nome(nome: &str) -> NumberFormat {
    match nome {
        "number" => NumberFormat::Number,
        "integer" => NumberFormat::Integer,
        "currency" => NumberFormat::Currency,
        "scientific" => NumberFormat::Scientific,
        "accounting" => NumberFormat::Accounting,
        "duration" => NumberFormat::Duration,
        "percent" => NumberFormat::Percent,
        "date" => NumberFormat::Date,
        "datetime" => NumberFormat::Datetime,
        "time" => NumberFormat::Time,
        "text" => NumberFormat::Text,
        _ => NumberFormat::Auto,
    }
}

/// O nome que o TypeScript usa para o formato. `Auto` não tem nome: é a
/// ausência do campo.
pub fn nome_do_formato(f: NumberFormat) -> Option<&'static str> {
    Some(match f {
        NumberFormat::Number => "number",
        NumberFormat::Integer => "integer",
        NumberFormat::Currency => "currency",
        NumberFormat::Scientific => "scientific",
        NumberFormat::Accounting => "accounting",
        NumberFormat::Duration => "duration",
        NumberFormat::Percent => "percent",
        NumberFormat::Date => "date",
        NumberFormat::Datetime => "datetime",
        NumberFormat::Time => "time",
        NumberFormat::Text => "text",
        NumberFormat::Auto => return None,
    })
}

/// O estilo resolvido, para formatar e desenhar.
pub fn para_style(e: &EstiloParcial) -> Style {
    let borders = match e.get("borders") {
        Some(Campo::Objeto(b)) => Some(Borders {
            top: logico(b, "top"),
            right: logico(b, "right"),
            bottom: logico(b, "bottom"),
            left: logico(b, "left"),
            color: texto(b, "color"),
        }),
        _ => None,
    };
    Style {
        bold: logico(e, "bold"),
        italic: logico(e, "italic"),
        underline: logico(e, "underline"),
        strike: logico(e, "strike"),
        wrap: logico(e, "wrap"),
        font_size: numero(e, "fontSize").map(|n| n as f32),
        font_family: texto(e, "fontFamily"),
        decimals: numero(e, "decimals").map(|n| n.clamp(0.0, 255.0) as u8),
        borders,
        align: match texto(e, "align").as_deref() {
            Some("left") => Align::Left,
            Some("center") => Align::Center,
            Some("right") => Align::Right,
            _ => Align::Auto,
        },
        valign: match texto(e, "vAlign").as_deref() {
            Some("top") => VAlign::Top,
            Some("bottom") => VAlign::Bottom,
            _ => VAlign::Middle,
        },
        format: texto(e, "format").map(|f| formato_por_nome(&f)).unwrap_or(NumberFormat::Auto),
        color: texto(e, "color"),
        bg: texto(e, "bg"),
        rotation: numero(e, "rotation").map(|n| n as i16),
        link: texto(e, "link"),
        code: texto(e, "numFmt").filter(|c| !c.is_empty()),
    }
}

/// O caminho de volta, para quem só tem o estilo resolvido (a carga antiga de
/// arquivo). Só entra o que difere do padrão — a mesma forma que o
/// TypeScript grava.
pub fn de_style(s: &Style) -> EstiloParcial {
    let mut e = EstiloParcial::new();
    let mut liga = |k: &str, v: bool| {
        if v {
            e.insert(k.into(), Campo::Logico(true));
        }
    };
    liga("bold", s.bold);
    liga("italic", s.italic);
    liga("underline", s.underline);
    liga("strike", s.strike);
    liga("wrap", s.wrap);
    if let Some(n) = s.font_size {
        e.insert("fontSize".into(), Campo::Numero(n as f64));
    }
    if let Some(f) = &s.font_family {
        e.insert("fontFamily".into(), Campo::Texto(f.clone()));
    }
    if let Some(d) = s.decimals {
        e.insert("decimals".into(), Campo::Numero(d as f64));
    }
    if let Some(c) = &s.code {
        e.insert("numFmt".into(), Campo::Texto(c.clone()));
    }
    if let Some(b) = &s.borders {
        let mut o = BTreeMap::new();
        for (k, v) in [("top", b.top), ("right", b.right), ("bottom", b.bottom), ("left", b.left)] {
            if v {
                o.insert(k.to_string(), Campo::Logico(true));
            }
        }
        if let Some(c) = &b.color {
            o.insert("color".into(), Campo::Texto(c.clone()));
        }
        e.insert("borders".into(), Campo::Objeto(o));
    }
    match s.align {
        Align::Left => e.insert("align".into(), Campo::Texto("left".into())),
        Align::Center => e.insert("align".into(), Campo::Texto("center".into())),
        Align::Right => e.insert("align".into(), Campo::Texto("right".into())),
        Align::Auto => None,
    };
    match s.valign {
        VAlign::Top => e.insert("vAlign".into(), Campo::Texto("top".into())),
        VAlign::Bottom => e.insert("vAlign".into(), Campo::Texto("bottom".into())),
        VAlign::Middle => None,
    };
    if let Some(n) = nome_do_formato(s.format) {
        e.insert("format".into(), Campo::Texto(n.into()));
    }
    if let Some(c) = &s.color {
        e.insert("color".into(), Campo::Texto(c.clone()));
    }
    if let Some(c) = &s.bg {
        e.insert("bg".into(), Campo::Texto(c.clone()));
    }
    if let Some(r) = s.rotation {
        e.insert("rotation".into(), Campo::Numero(r as f64));
    }
    if let Some(l) = &s.link {
        e.insert("link".into(), Campo::Texto(l.clone()));
    }
    e
}

/// O formato de número do estilo, sem resolver o resto — é a pergunta que o
/// `styleFor` do `sheet.ts` faz a cada escrita.
pub fn formato(e: Option<&EstiloParcial>) -> Option<String> {
    e.and_then(|e| texto(e, "format"))
}

#[cfg(test)]
mod testes {
    use super::*;

    fn estilo(pares: &[(&str, Campo)]) -> EstiloParcial {
        pares.iter().map(|(k, v)| (k.to_string(), v.clone())).collect()
    }

    #[test]
    fn falso_na_celula_desliga_o_negrito_da_coluna() {
        // A diferença que um `Style` com `bool` perderia.
        let coluna = estilo(&[("bold", Campo::Logico(true))]);
        let celula = estilo(&[("bold", Campo::Logico(false))]);
        let final_ = mesclar(Some(&coluna), &celula);
        assert!(!para_style(&final_).bold);
        let sem_nada = mesclar(Some(&coluna), &EstiloParcial::new());
        assert!(para_style(&sem_nada).bold);
    }

    #[test]
    fn nulo_no_patch_apaga_o_campo() {
        let base = estilo(&[("bold", Campo::Logico(true)), ("color", Campo::Texto("#f00".into()))]);
        let patch = estilo(&[("color", Campo::Nulo)]);
        let r = mesclar(Some(&base), &patch);
        assert_eq!(r.len(), 1);
        assert!(r.contains_key("bold"));
    }

    #[test]
    fn as_bordas_vao_e_voltam() {
        // O leitor antigo do `.fshz` em Rust jogava a borda fora
        // (`borders: None`). Aqui ela atravessa inteira.
        let mut b = BTreeMap::new();
        b.insert("top".to_string(), Campo::Logico(true));
        b.insert("color".to_string(), Campo::Texto("#123456".into()));
        let e = estilo(&[("borders", Campo::Objeto(b))]);
        let s = para_style(&e);
        let bordas = s.borders.as_ref().unwrap();
        assert!(bordas.top && !bordas.left);
        assert_eq!(bordas.color.as_deref(), Some("#123456"));
        assert_eq!(de_style(&s), e);
    }
}
