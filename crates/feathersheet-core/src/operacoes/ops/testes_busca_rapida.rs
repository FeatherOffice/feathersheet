//! A busca preparada (`Consulta`, texto por id do pool, número pulado)
//! tem de achar exatamente o que a conta ingênua acha: `is_hit` no
//! `to_text` de cada célula.
use super::*;

pub(super) fn ingenua(s: &Sheet, q: &str, o: SearchOptions) -> Vec<SearchMatch> {
    let mut out = Vec::new();
    for r in 0..=20 {
        for c in 0..=5 {
            let t = if o.in_formulas { s.get_raw(r, c) } else { to_text(&s.get_value(r, c)) };
            if !t.is_empty() && is_hit(&t, q, o) {
                out.push(SearchMatch { row: r, col: c });
            }
        }
    }
    out
}

#[test]
pub(super) fn acha_o_mesmo_que_a_conta_celula_a_celula() {
    let mut s = Sheet::new("A");
    let dados = [
        ["ALTO", "alto", "Álto", "12,5", "=A1&\"x\"", "VERDADEIRO"],
        ["NORMAL", "1e21", "-3", "Infinity", "=2*3", "maçã"],
        ["123", "0,000001", "SÃO PAULO", "são paulo", "=1/0", "  alto  "],
        ["-4500", "0", "-0", "999999999999999", "=-12*10", "1e15"],
    ];
    for (r, linha) in dados.iter().enumerate() {
        for (c, t) in linha.iter().enumerate() {
            s.set_cell(r as u32, c as u32, t);
        }
    }
    for q in ["alto", "ALTO", "Álto", "12", "1e", "e", "-", "inf", "são", "SÃO PAULO", "div", "x", "6", "verdadeiro", "ç", "0", "-0", "-45", "999", "120", "#num!"] {
        for match_case in [false, true] {
            for whole_cell in [false, true] {
                for in_formulas in [false, true] {
                    let o = SearchOptions { match_case, whole_cell, in_formulas };
                    assert_eq!(find_all(&s, q, o), ingenua(&s, q, o), "consulta {q:?} com {o:?}");
                }
            }
        }
    }
}
