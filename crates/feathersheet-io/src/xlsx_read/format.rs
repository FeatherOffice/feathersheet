//! Código de formato do Excel (`numFmtId`/`formatCode`) -> `NumberFormat` do
//! motor. Espelha `formatFromCode`/`BUILTIN` de `src/io/xlsx.ts:396-455`.

/// IDs embutidos do Excel que o motor entende (os mesmos que `xlsx.ts`
/// mapeia — não é a tabela 0-49 inteira, só o que este projeto usa).
pub fn builtin_code(id: u32) -> Option<&'static str> {
    Some(match id {
        0 => "General",
        1 => "0",
        2 => "0.00",
        3 => "#,##0",
        4 => "#,##0.00",
        9 => "0%",
        10 => "0.00%",
        11 => "0.00E+00",
        14 => "dd/mm/yyyy",
        15 => "d-mmm-yy",
        16 => "d-mmm",
        17 => "mmm-yy",
        18 => "h:mm AM/PM",
        19 => "h:mm:ss AM/PM",
        20 => "h:mm",
        21 => "h:mm:ss",
        22 => "dd/mm/yyyy hh:mm",
        37 => "#,##0 ;(#,##0)",
        38 => "#,##0 ;[Red](#,##0)",
        39 => "#,##0.00;(#,##0.00)",
        40 => "#,##0.00;[Red](#,##0.00)",
        45 => "mm:ss",
        46 => "[h]:mm:ss",
        47 => "mmss.0",
        48 => "##0.0E+0",
        49 => "@",
        _ => return None,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedFormat {
    pub format: &'static str,
    pub decimals: Option<u32>,
}

fn decimals_in(code: &str) -> u32 {
    // /\.([0#?]+)/ — primeira casa decimal declarada no código.
    if let Some(dot) = code.find('.') {
        code[dot + 1..]
            .chars()
            .take_while(|c| matches!(c, '0' | '#' | '?'))
            .count() as u32
    } else {
        0
    }
}

/// Tira grupos `[...]` do código, menos `[h]`/`[hh]`/... (marcador de hora
/// decorrida) — mesma regra de `xlsx.ts:427`.
fn strip_brackets(code: &str) -> String {
    let mut out = String::new();
    let mut chars = code.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '[' {
            let mut inner = String::new();
            let mut fechou = false;
            for c2 in chars.by_ref() {
                if c2 == ']' {
                    fechou = true;
                    break;
                }
                inner.push(c2);
            }
            if fechou {
                let bracket = format!("[{inner}]");
                let so_h = !inner.is_empty() && inner.chars().all(|c| c == 'h' || c == 'H');
                if so_h {
                    out.push_str(&bracket);
                }
                // senão: descarta o grupo, igual ao JS.
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Igual `formatFromCode` (`xlsx.ts:373-450`), mas com o nome de campo da
/// versão de exportação (`formatCodeFor`) — aqui só a direção de leitura
/// interessa (código -> formato nomeado).
pub fn format_from_code(code: &str) -> ParsedFormat {
    let clean = strip_brackets(code).trim().to_string();

    if clean.is_empty() || clean.eq_ignore_ascii_case("general") {
        return ParsedFormat { format: "auto", decimals: None };
    }
    if clean == "@" {
        return ParsedFormat { format: "text", decimals: None };
    }
    if regex_h_prefix(&clean) {
        return ParsedFormat { format: "duration", decimals: None };
    }
    if clean.to_uppercase().contains("E+") {
        return ParsedFormat { format: "scientific", decimals: Some(decimals_in(&clean)) };
    }
    if clean.contains('%') {
        return ParsedFormat { format: "percent", decimals: Some(decimals_in(&clean)) };
    }

    // Fora de texto entre aspas, `m` depois de `h` é minuto, não mês.
    let bare = strip_quoted_and_escapes(&clean);
    let has_date = bare.chars().any(|c| c == 'y' || c == 'Y' || c == 'd' || c == 'D');
    let has_time = bare.chars().any(|c| c == 'h' || c == 'H' || c == 's' || c == 'S') || minute_before_colon(&bare);
    if has_date && has_time {
        return ParsedFormat { format: "datetime", decimals: None };
    }
    if has_date {
        return ParsedFormat { format: "date", decimals: None };
    }
    if has_time {
        return ParsedFormat { format: "time", decimals: None };
    }

    let money = clean.contains("R$") || clean.contains('$') || clean.contains('€') || clean.contains('£');
    if money {
        return if clean.contains('*') || clean.contains("_(") {
            ParsedFormat { format: "accounting", decimals: Some(decimals_in(&clean)) }
        } else {
            ParsedFormat { format: "currency", decimals: Some(decimals_in(&clean)) }
        };
    }

    let decimals = decimals_in(&clean);
    if decimals == 0 {
        ParsedFormat { format: "integer", decimals: None }
    } else {
        ParsedFormat { format: "number", decimals: Some(decimals) }
    }
}

fn regex_h_prefix(clean: &str) -> bool {
    // /^\[h+\]/i — só sobra depois do strip_brackets se era só "h"s.
    let lower = clean.to_lowercase();
    lower.starts_with('[') && lower[1..].chars().take_while(|&c| c != ']').all(|c| c == 'h') && lower.contains(']')
}

/// Tira texto entre aspas e `\X` (caractere literal escapado), pra não
/// confundir `m`/`d`/`h` de dentro de um texto literal com token de data.
fn strip_quoted_and_escapes(code: &str) -> String {
    let mut out = String::new();
    let mut chars = code.chars().peekable();
    let mut in_quotes = false;
    while let Some(c) = chars.next() {
        if in_quotes {
            if c == '"' {
                in_quotes = false;
            }
            continue;
        }
        if c == '"' {
            in_quotes = true;
            continue;
        }
        if c == '\\' {
            chars.next(); // pula o caractere escapado
            continue;
        }
        out.push(c);
    }
    out
}

/// `\bm+:` — um ou mais `m` seguidos de `:`, com fronteira de palavra antes.
fn minute_before_colon(bare: &str) -> bool {
    let chars: Vec<char> = bare.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == 'm' || chars[i] == 'M' {
            let inicio_de_palavra = i == 0 || !chars[i - 1].is_alphanumeric();
            if inicio_de_palavra {
                let mut j = i;
                while j < chars.len() && (chars[j] == 'm' || chars[j] == 'M') {
                    j += 1;
                }
                if j < chars.len() && chars[j] == ':' {
                    return true;
                }
            }
        }
        i += 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codigos_basicos() {
        assert_eq!(format_from_code("General").format, "auto");
        assert_eq!(format_from_code("@").format, "text");
        assert_eq!(format_from_code("dd/mm/yyyy").format, "date");
        assert_eq!(format_from_code("dd/mm/yyyy hh:mm").format, "datetime");
        assert_eq!(format_from_code("hh:mm:ss").format, "time");
        assert_eq!(format_from_code("[h]:mm:ss").format, "duration");
        assert_eq!(format_from_code("0.00%").format, "percent");
        assert_eq!(format_from_code("0.00E+00").format, "scientific");
        assert_eq!(format_from_code("\"R$\" #,##0.00").format, "currency");
        let n = format_from_code("#,##0.000");
        assert_eq!(n.format, "number");
        assert_eq!(n.decimals, Some(3));
        let i = format_from_code("#,##0");
        assert_eq!(i.format, "integer");
        assert_eq!(i.decimals, None);
    }

    #[test]
    fn mmm_yy_e_data_nao_minuto() {
        assert_eq!(format_from_code("mmm-yy").format, "date");
    }

    #[test]
    fn builtin_conhecidos() {
        assert_eq!(builtin_code(14), Some("dd/mm/yyyy"));
        assert_eq!(builtin_code(9), Some("0%"));
        assert_eq!(builtin_code(9999), None);
    }
}
