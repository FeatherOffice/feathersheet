use crate::celulas::refs::{col_index, col_name};
pub use crate::celulas::refs::quote_sheet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedRef {


    /// Nome da aba, quando a referência é qualificada (Dados!A1).
    pub sheet: Option<String>,
    pub col_lock: bool,
    pub row_lock: bool,
    pub col: i32,
    pub row: i32,
}

/// Monta o texto de uma referência de volta, preservando os cifrões.
pub fn format_ref(ref_cell: &ParsedRef) -> String {
    if ref_cell.col < 0 || ref_cell.row < 0 {
        return "#REF!".to_string();
    }
    let prefix = match &ref_cell.sheet {
        Some(s) => format!("{}!", quote_sheet(s)),
        None => String::new(),
    };
    let col_lock = if ref_cell.col_lock { "$" } else { "" };
    let row_lock = if ref_cell.row_lock { "$" } else { "" };
    let cname = col_name(ref_cell.col as u32);
    format!("{}{}{}{}{}", prefix, col_lock, cname, row_lock, ref_cell.row + 1)
}

/// Reescreve todas as referências A1 de uma fórmula.
pub fn map_references<F>(raw: &str, mut fn_map: F) -> String
where
    F: FnMut(ParsedRef) -> String,
{
    if !raw.starts_with('=') {
        return raw.to_string();
    }

    let chars: Vec<char> = raw.chars().collect();
    let mut out = String::from("=");
    let mut i = 1;
    let mut in_string = false;

    while i < chars.len() {
        let c = chars[i];

        if in_string {
            out.push(c);
            if c == '"' {
                if i + 1 < chars.len() && chars[i + 1] == '"' {
                    out.push('"');
                    i += 2;
                    continue;
                }
                in_string = false;
            }
            i += 1;
            continue;
        }

        if c == '"' {
            in_string = true;
            out.push(c);
            i += 1;
            continue;
        }

        let qualified = match_sheet_prefix(&chars, i);
        let ref_start = if let Some(ref q) = qualified {
            i + q.len
        } else {
            i
        };

        let ref_match = match_ref_at(&chars, ref_start);
        let previous = if i > 0 { chars[i - 1] } else { '\0' };
        let glued = is_glued_char(previous);

        if let Some((col_lock, col_str, row_lock, row_str, match_len)) = ref_match {
            if qualified.is_some() || !glued {
                let after_idx = ref_start + match_len;
                let after = if after_idx < chars.len() { chars[after_idx] } else { '\0' };

                if qualified.is_none() && (after == '(' || is_name_char(after)) {
                    let matched_str: String = chars[i..after_idx].iter().collect();
                    out.push_str(&matched_str);
                    i = after_idx;
                    continue;
                }

                if let Some(col) = col_index(&col_str) {
                    if let Ok(row_num) = row_str.parse::<i32>() {
                        let parsed = ParsedRef {
                            sheet: qualified.map(|q| q.name),
                            col_lock,
                            row_lock,
                            col: col as i32,
                            row: row_num - 1,
                        };
                        out.push_str(&fn_map(parsed));
                        i = ref_start + match_len;
                        continue;
                    }
                }
            }
        }

        out.push(c);
        i += 1;
    }

    out
}

struct SheetPrefix {
    name: String,
    len: usize,
}

fn match_sheet_prefix(chars: &[char], at: usize) -> Option<SheetPrefix> {
    if at >= chars.len() {
        return None;
    }

    if chars[at] == '\'' {
        let mut i = at + 1;
        let mut name = String::new();
        while i < chars.len() {
            if chars[i] == '\'' {
                if i + 1 < chars.len() && chars[i + 1] == '\'' {
                    name.push('\'');
                    i += 2;
                    continue;
                }
                if i + 1 < chars.len() && chars[i + 1] == '!' {
                    return Some(SheetPrefix {
                        name,
                        len: (i + 2) - at,
                    });
                }
                return None;
            }
            name.push(chars[i]);
            i += 1;
        }
        return None;
    }

    if !is_name_char(chars[at]) {
        return None;
    }

    let previous = if at > 0 { chars[at - 1] } else { '\0' };
    if is_glued_char(previous) {
        return None;
    }

    let mut i = at;
    let mut name = String::new();
    while i < chars.len() && (is_name_char(chars[i]) || chars[i].is_ascii_digit() || chars[i] == '.') {
        name.push(chars[i]);
        i += 1;
    }

    if i < chars.len() && chars[i] == '!' {
        Some(SheetPrefix {
            name,
            len: (i + 1) - at,
        })
    } else {
        None
    }
}

fn match_ref_at(chars: &[char], at: usize) -> Option<(bool, String, bool, String, usize)> {
    if at >= chars.len() {
        return None;
    }

    let mut i = at;
    let col_lock = chars[i] == '$';
    if col_lock {
        i += 1;
    }

    let col_start = i;
    while i < chars.len() && chars[i].is_ascii_alphabetic() {
        i += 1;
    }
    let col_len = i - col_start;
    if col_len == 0 || col_len > 3 {
        return None;
    }
    let col_str: String = chars[col_start..i].iter().collect();

    let row_lock = i < chars.len() && chars[i] == '$';
    if row_lock {
        i += 1;
    }

    let row_start = i;
    while i < chars.len() && chars[i].is_ascii_digit() {
        i += 1;
    }
    let row_len = i - row_start;
    if row_len == 0 || row_len > 9 {
        return None;
    }
    let row_str: String = chars[row_start..i].iter().collect();

    Some((col_lock, col_str, row_lock, row_str, i - at))
}

fn is_name_char(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

/// Preserva tokens desconhecidos na reescrita textual: `1e3`, `2A1`,
/// `_A1` e nomes contendo ponto/dólar não podem ser tratados como
/// referências embutidas. O TypeScript exigia esse limite lexical.
fn is_glued_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '.' | '$')
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Row,
    Col,
}

pub struct StructureAdjustOptions<'a> {
    pub axis: Axis,
    pub at: i32,
    pub count: i32,
    pub removing: bool,
    pub sheet: &'a str,
    pub own_sheet: &'a str,
}

/// Ajusta uma fórmula depois de inserir ou remover linhas/colunas.
pub fn adjust_for_structure(raw: &str, options: StructureAdjustOptions) -> String {
    let same = |name: Option<&str>| {
        let n = name.unwrap_or(options.own_sheet);
        n.eq_ignore_ascii_case(options.sheet)
    };

    // A qualificação da primeira ponta vale para a faixa inteira, mas o
    // texto da segunda ponta continua sem prefixo. Antes, inserir uma linha
    // em Produtos ajustava só A1 de `Produtos!A1:A9`.
    use crate::formula::formula_pattern::{pattern_of, pedacos, Pedaco};
    let mut abas = Vec::new();
    let mut anterior = None;
    let mut separador = false;
    for pedaco in pedacos(&pattern_of(raw, 0, 0)) {
        match pedaco {
            Pedaco::Texto(t) => separador = t.trim() == ":",
            Pedaco::Ref(r) => {
                let aba = r.sheet.or_else(|| if separador { anterior.clone() } else { None });
                anterior = aba.clone();
                abas.push(aba);
                separador = false;
            }
        }
    }
    let mut indice = 0;
    map_references(raw, |ref_cell| {
        let aba = abas.get(indice).and_then(|s| s.as_deref());
        indice += 1;
        if !same(aba.or(ref_cell.sheet.as_deref())) {
            return format_ref(&ref_cell);
        }

        let index = if options.axis == Axis::Row {
            ref_cell.row
        } else {
            ref_cell.col
        };

        let mut next = index;
        if options.removing {
            if index >= options.at && index < options.at + options.count {
                return "#REF!".to_string();
            }
            if index >= options.at + options.count {
                next = index - options.count;
            }
        } else if index >= options.at {
            next = index + options.count;
        }

        if options.axis == Axis::Row {
            format_ref(&ParsedRef { row: next, ..ref_cell })
        } else {
            format_ref(&ParsedRef { col: next, ..ref_cell })
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_ref() {
        let r = ParsedRef {
            sheet: None,
            col_lock: false,
            row_lock: false,
            col: 0,
            row: 0,
        };
        assert_eq!(format_ref(&r), "A1");

        let r_lock = ParsedRef {
            sheet: Some("Plan1".to_string()),
            col_lock: true,
            row_lock: true,
            col: 1,
            row: 4,
        };
        assert_eq!(format_ref(&r_lock), "Plan1!$B$5");
    }

    #[test]
    fn test_quote_sheet() {
        assert_eq!(quote_sheet("Dados"), "Dados");
        assert_eq!(quote_sheet("Meu Plano"), "'Meu Plano'");
        assert_eq!(quote_sheet("D'água"), "'D''água'");
        assert_eq!(quote_sheet("Σoma"), "'Σoma'");
        assert_eq!(format_ref(&ParsedRef {
            sheet: Some("Σoma".into()), col_lock: false,
            row_lock: false, col: 0, row: 0,
        }), "'Σoma'!A1");
    }

    #[test]
    fn test_map_references() {
        let formula = "=SOMA(A1; B2; \"texto A1\"; Plan1!C3)";
        let mapped = map_references(formula, |mut ref_cell| {
            ref_cell.row += 1;
            format_ref(&ref_cell)
        });
        assert_eq!(mapped, "=SOMA(A2; B3; \"texto A1\"; Plan1!C4)");
    }

    #[test]
    fn nao_reescreve_referencia_no_meio_de_identificador_ou_numero() {
        let formula = "=1e3+2A1+_B2+f.A3+$C$4+LOG10(A1)+A1";
        let reescrita = map_references(formula, |mut r| {
            r.row += 1;
            format_ref(&r)
        });
        assert_eq!(
            reescrita,
            "=1e3+2A1+_B2+f.A3+$C$5+LOG10(A2)+A2"
        );
        assert_eq!(map_references("texto A1", |_| "X".into()), "texto A1");
    }

    #[test]
    fn referencias_qualificadas_e_strings_escapadas_sao_preservadas() {
        let original = "=A1+'O''Brien'!$B$2+\"A1 \"\" B2\"+Dados!C3";
        let atualizado = map_references(original, |mut r| {
            r.row += 1;
            format_ref(&r)
        });
        assert_eq!(
            atualizado,
            "=A2+'O''Brien'!$B$3+\"A1 \"\" B2\"+Dados!C4"
        );
        // Alterar só uma aba: não confundir referências locais e externas.
        assert_eq!(
            adjust_for_structure(
                "=A3+Outra!A3+Dados!$B$3",
                StructureAdjustOptions {
                    axis: Axis::Row,
                    at: 2,
                    count: 1,
                    removing: true,
                    sheet: "Dados",
                    own_sheet: "Dados",
                },
            ),
            "=#REF!+Outra!A3+#REF!"
        );
    }

    #[test]
    fn test_adjust_for_structure() {
        let formula = "=A1 + B5";
        let res = adjust_for_structure(
            formula,
            StructureAdjustOptions {
                axis: Axis::Row,
                at: 2,
                count: 1,
                removing: false,
                sheet: "Aba1",
                own_sheet: "Aba1",
            },
        );
        assert_eq!(res, "=A1 + B6");
    }
}
