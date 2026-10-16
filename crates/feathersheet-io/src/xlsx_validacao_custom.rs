//! Regras de validação do motor que o Excel não tem como tipo próprio
//! (caixa de seleção, e-mail, URL, "contém", "começa com") são gravadas como
//! validação **personalizada** (`type="custom"`) com uma fórmula equivalente,
//! relativa à primeira célula da regra. A leitura reconhece exatamente estas
//! fórmulas e devolve a regra original; fórmula personalizada de outra origem
//! continua sem equivalente no motor.
//!
//! E-mail e URL são aproximações: a fórmula confere os mesmos pontos
//! principais (um `@`, ponto no domínio, sem espaço), sem a checagem de letras
//! do final que o motor faz.
use feathersheet_core::operacoes::validation::{Regra, TextoOp};

/// Texto entre aspas numa fórmula do Excel (`"` dobra).
fn literal(texto: &str) -> String {
    format!("\"{}\"", texto.replace('"', "\"\""))
}

/// Valor da caixa como o Excel o compara: `TRUE`/`FALSE` quando o texto é o
/// booleano do motor, senão o literal.
fn valor_caixa(texto: &str) -> String {
    match texto.trim().to_uppercase().as_str() {
        "VERDADEIRO" | "TRUE" => "TRUE".into(),
        "FALSO" | "FALSE" => "FALSE".into(),
        _ => literal(texto),
    }
}

/// A fórmula (inglês, sem `=`) que faz o papel da regra, para a célula `c`.
pub(crate) fn formula_custom(regra: &Regra, c: &str) -> Option<String> {
    Some(match regra {
        Regra::Texto { op: TextoOp::Contem, valor } => {
            format!("ISNUMBER(SEARCH({},{c}))", literal(valor.as_deref().unwrap_or("")))
        }
        Regra::Texto { op: TextoOp::Comeca, valor } => {
            let v = literal(valor.as_deref().unwrap_or(""));
            format!("LEFT({c},LEN({v}))={v}")
        }
        Regra::Texto { op: TextoOp::Email, .. } => format!(
            "IFERROR(AND(ISERROR(FIND(\" \",{c})),LEN({c})-LEN(SUBSTITUTE({c},\"@\",\"\"))=1,FIND(\"@\",{c})>1,ISNUMBER(FIND(\".\",{c},FIND(\"@\",{c})+2)),RIGHT({c},1)<>\".\"),FALSE)"
        ),
        Regra::Texto { op: TextoOp::Url, .. } => {
            format!("AND(ISERROR(FIND(\" \",{c})),ISNUMBER(FIND(\".\",{c})),RIGHT({c},1)<>\".\")")
        }
        Regra::Caixa { marcado, desmarcado } => {
            format!("OR({c}={},{c}={})", valor_caixa(marcado), valor_caixa(desmarcado))
        }
        _ => return None,
    })
}

/// Lê `"texto"` (com `""` dobrado) no começo de `s`; devolve o texto e o resto.
fn ler_literal(s: &str) -> Option<(String, &str)> {
    let s = s.strip_prefix('"')?;
    let mut saida = String::new();
    let mut it = s.char_indices();
    while let Some((i, ch)) = it.next() {
        if ch == '"' {
            if s[i + 1..].starts_with('"') {
                saida.push('"');
                it.next();
            } else {
                return Some((saida, &s[i + 1..]));
            }
        } else {
            saida.push(ch);
        }
    }
    None
}

fn texto_caixa(s: &str) -> Option<(String, &str)> {
    if let Some(r) = s.strip_prefix("TRUE") {
        return Some(("VERDADEIRO".into(), r));
    }
    if let Some(r) = s.strip_prefix("FALSE") {
        return Some(("FALSO".into(), r));
    }
    ler_literal(s)
}

/// O inverso de [`formula_custom`]: reconhece a fórmula gravada pelo motor.
pub(crate) fn regra_de_custom(formula: &str, c: &str) -> Option<Regra> {
    let f = formula.trim().strip_prefix('=').unwrap_or(formula.trim());
    for op in [TextoOp::Email, TextoOp::Url] {
        let r = Regra::Texto { op, valor: None };
        if formula_custom(&r, c).as_deref() == Some(f) {
            return Some(r);
        }
    }
    if let Some(resto) = f.strip_prefix("ISNUMBER(SEARCH(") {
        let (v, resto) = ler_literal(resto)?;
        return (resto == format!(",{c}))")).then(|| Regra::Texto { op: TextoOp::Contem, valor: Some(v) });
    }
    if let Some(resto) = f.strip_prefix(&format!("LEFT({c},LEN(")) {
        let (v, resto) = ler_literal(resto)?;
        let resto = resto.strip_prefix("))=")?;
        let (v2, fim) = ler_literal(resto)?;
        return (v == v2 && fim.is_empty()).then(|| Regra::Texto { op: TextoOp::Comeca, valor: Some(v) });
    }
    if let Some(resto) = f.strip_prefix(&format!("OR({c}=")) {
        let (marcado, resto) = texto_caixa(resto)?;
        let resto = resto.strip_prefix(&format!(",{c}="))?;
        let (desmarcado, resto) = texto_caixa(resto)?;
        return (resto == ")").then_some(Regra::Caixa { marcado, desmarcado });
    }
    None
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn toda_formula_gravada_volta_como_a_regra() {
        let regras = [
            Regra::Texto { op: TextoOp::Contem, valor: Some("a\"b,(c)".into()) },
            Regra::Texto { op: TextoOp::Comeca, valor: Some("PRE".into()) },
            Regra::Texto { op: TextoOp::Email, valor: None },
            Regra::Texto { op: TextoOp::Url, valor: None },
            Regra::Caixa { marcado: "Sim".into(), desmarcado: "Não".into() },
            Regra::Caixa { marcado: "VERDADEIRO".into(), desmarcado: "FALSO".into() },
        ];
        for r in regras {
            let f = formula_custom(&r, "B7").unwrap();
            assert_eq!(regra_de_custom(&f, "B7"), Some(r.clone()), "{f}");
            assert_eq!(regra_de_custom(&format!("={f}"), "B7"), Some(r));
        }
        assert_eq!(formula_custom(&Regra::Lista { valores: vec![] }, "A1"), None);
        assert_eq!(regra_de_custom("A1>5", "A1"), None);
        assert_eq!(regra_de_custom("OR(B7=\"x\",B7=\"y\")", "A1"), None, "outra célula");
    }
}
