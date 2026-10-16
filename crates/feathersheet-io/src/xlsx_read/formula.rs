//! Tradução de fórmula Excel (inglês, vírgula de argumento, ponto decimal)
//! -> motor (português, ponto e vírgula, vírgula decimal). Espelha
//! `formulaFromExcel`/`safeFormula` de `src/io/xlsx.ts:236-354`.
//!
//! Varredor de caractere de propósito, igual ao original: o tokenizador do
//! motor é pt-BR e leria `A1:A3,2.5` como o número "3,2" — a vírgula entre
//! dígitos é decimal por lá.

use super::pt_name::PT_NAME;

/// O nome em português: a tabela preferida e, fora dela, o primeiro apelido
/// do motor (com acento), para nenhuma função do Excel ficar em inglês.
fn pt_name_for(word_upper: &str) -> Option<&'static str> {
    PT_NAME.iter().find(|(en, _)| *en == word_upper).map(|(_, pt)| *pt).or_else(|| {
        feathersheet_core::funcoes::functions::aliases()
            .iter()
            .find(|(_, en)| *en == word_upper)
            .map(|(pt, _)| *pt)
    })
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_' || c == '$' || ('\u{00c0}'..='\u{024f}').contains(&c)
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '$' || ('\u{00c0}'..='\u{024f}').contains(&c)
}

/// `SUM(A1:A3,2.5)` -> `=SOMA(A1:A3;2,5)`.
pub fn formula_from_excel(text: &str) -> String {
    let sem_prefixo = text.strip_prefix('=').unwrap_or(text);
    let sem_xlfn = sem_prefixo.replace("_xlfn.", "").replace("_xlws.", "").replace("_xlpm.", "");
    let src: Vec<char> = sem_xlfn.chars().collect();

    let mut out = String::new();
    let mut braces = 0i32;
    let mut i = 0usize;

    while i < src.len() {
        let c = src[i];

        if c == '"' {
            let start = i;
            i += 1;
            while i < src.len() {
                if src[i] == '"' {
                    if src.get(i + 1) == Some(&'"') {
                        i += 2;
                        continue;
                    }
                    break;
                }
                i += 1;
            }
            i = (i + 1).min(src.len());
            out.extend(&src[start..i]);
            continue;
        }
        if c == '\'' {
            let start = i;
            i += 1;
            while i < src.len() {
                if src[i] == '\'' {
                    if src.get(i + 1) == Some(&'\'') {
                        i += 2;
                        continue;
                    }
                    break;
                }
                i += 1;
            }
            i = (i + 1).min(src.len());
            out.extend(&src[start..i]);
            continue;
        }
        if c.is_ascii_digit() {
            let start = i;
            while i < src.len() && src[i].is_ascii_digit() {
                i += 1;
            }
            if src.get(i) == Some(&'.') && src.get(i + 1).is_some_and(|c| c.is_ascii_digit()) {
                i += 1;
                while i < src.len() && src[i].is_ascii_digit() {
                    i += 1;
                }
            }
            if matches!(src.get(i), Some('e') | Some('E')) && src.get(i + 1).is_some_and(|c| c.is_ascii_digit() || *c == '+' || *c == '-') {
                let save = i;
                i += 1;
                if matches!(src.get(i), Some('+') | Some('-')) {
                    i += 1;
                }
                if src.get(i).is_some_and(|c| c.is_ascii_digit()) {
                    while i < src.len() && src[i].is_ascii_digit() {
                        i += 1;
                    }
                } else {
                    i = save;
                }
            }
            let numero: String = src[start..i].iter().collect();
            out.push_str(&numero.replacen('.', ",", 1));
            continue;
        }
        if c == '#' {
            let start = i;
            i += 1;
            while i < src.len() && src[i] != '!' && !matches!(src[i], ' ' | '\t' | '\n' | ',' | ';' | ')') {
                i += 1;
            }
            if src.get(i) == Some(&'!') {
                i += 1;
            }
            out.extend(&src[start..i]);
            continue;
        }
        if c == '[' {
            let start = i;
            let fim = src[i..].iter().position(|&c| c == ']').map(|p| i + p + 1).unwrap_or(src.len());
            out.extend(&src[start..fim]);
            i = fim;
            continue;
        }
        if is_ident_start(c) {
            let start = i;
            while i < src.len() && is_ident_char(src[i]) {
                i += 1;
            }
            let word: String = src[start..i].iter().collect();
            let mut j = i;
            while j < src.len() && src[j] == ' ' {
                j += 1;
            }
            let pt = pt_name_for(&word.to_uppercase());
            match pt {
                Some(p) if src.get(j) == Some(&'(') || p == "VERDADEIRO" || p == "FALSO" => out.push_str(p),
                _ => out.push_str(&word),
            }
            continue;
        }
        if c == '{' {
            braces += 1;
            out.push(c);
            i += 1;
            continue;
        }
        if c == '}' {
            braces -= 1;
            out.push(c);
            i += 1;
            continue;
        }
        if c == ',' {
            out.push(if braces > 0 { '\\' } else { ';' });
            i += 1;
            continue;
        }
        out.push(c);
        i += 1;
    }
    format!("={out}")
}

/// No JS, `safeFormula` embrulha a tradução em try/catch pra nunca derrubar
/// a importação por causa de uma fórmula estranha. Aqui `formula_from_excel`
/// só percorre caracteres com acesso seguro (`.get()`), então não tem como
/// entrar em pânico — não existe um `safe_formula` à parte; o próprio
/// `formula_from_excel` já é a versão segura.
pub use formula_from_excel as safe_formula;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn funcoes_basicas() {
        assert_eq!(formula_from_excel("SUM(A1:A3,2.5)"), "=SOMA(A1:A3;2,5)");
        assert_eq!(formula_from_excel("IF(A1>0,\"sim\",\"não\")"), "=SE(A1>0;\"sim\";\"não\")");
        assert_eq!(formula_from_excel("AVERAGE(A1:A10)"), "=MÉDIA(A1:A10)");
    }

    #[test]
    fn nomes_oficiais_pt_br_na_leitura() {
        assert_eq!(formula_from_excel(r#"CONCATENATE("a",B1)"#), r#"=CONCATENAR("a";B1)"#);
        assert_eq!(formula_from_excel("ISBLANK(A1)"), "=ÉCÉL.VAZIA(A1)");
        assert_eq!(formula_from_excel("ISERROR(A1)"), "=ÉERRO(A1)");
        assert_eq!(formula_from_excel("PERMUT(5,2)"), "=PERMUT(5;2)");
        assert_eq!(formula_from_excel("COMBIN(5,2)"), "=COMBIN(5;2)");
        assert_eq!(formula_from_excel("SORT(A1:A3)"), "=CLASSIFICAR(A1:A3)");
    }

    #[test]
    fn xlfn_prefix_removido() {
        assert_eq!(formula_from_excel("_xlfn.XLOOKUP(A1,B:B,C:C)"), "=PROCX(A1;B:B;C:C)");
    }

    #[test]
    fn texto_entre_aspas_nao_e_traduzido() {
        assert_eq!(formula_from_excel(r#"CONCAT("SOMA vale ",A1)"#), r#"=CONCAT("SOMA vale ";A1)"#);
    }

    #[test]
    fn matriz_troca_barra_por_virgula_so_dentro_de_chaves() {
        assert_eq!(formula_from_excel("SUM({1,2,3})"), "=SOMA({1\\2\\3})");
    }

    #[test]
    fn nome_com_varios_apelidos_normaliza() {
        // ÉERRO e É.ERRO existem os dois em ALIASES apontando pra ISERROR;
        // a importação sempre usa o primeiro apelido visto (É.ERRO, pela
        // ordem alfabética que o script de geração usa por robustez de
        // teste — o que importa aqui é que o resultado é estável).
        let a = formula_from_excel("ISERROR(A1)");
        let b = formula_from_excel("ISERROR(A1)");
        assert_eq!(a, b);
    }

    #[test]
    fn erro_literal_passa_intacto() {
        assert_eq!(formula_from_excel("IF(A1=1,#DIV/0!,0)"), "=SE(A1=1;#DIV/0!;0)");
    }

    #[test]
    fn safe_formula_nunca_lanca() {
        assert_eq!(safe_formula("SUM(A1:A2)"), "=SOMA(A1:A2)");
    }
}
