//! Funções de banco de dados em Rust: BDSOMA, BDMÉDIA, BDCONTAR, BDCONTARA, BDEXTRAIR, BDMÁX, BDMÍN, BDMULTIPL,
//! BDEST, BDESTP, BDVAREST, BDVAREST.P.
//!
//! O banco de dados é um intervalo com cabeçalho na primeira linha; o campo é o nome da coluna ou o número dela;
//! os critérios são outro intervalo com cabeçalho — cada linha de critérios é uma alternativa (OU) e as colunas
//! de uma mesma linha valem juntas (E).

use crate::celulas::values::{to_text, Computed, Value};
use crate::formula::ast::Node;
use crate::formula::evaluator::Evaluator;
use crate::funcoes::fn_helpers::{
    concluir, e_div0, e_num, e_valor, has, make_predicate, matriz_de, numero, Predicate, Res,
};

/// Linhas do banco que atendem aos critérios, e o índice da coluna do campo.
struct Selecao {
    /// A coluna do campo (None quando o campo foi omitido, só aceito por BDCONTAR/BDCONTARA).
    coluna: Option<usize>,
    linhas: Vec<Vec<Value>>,
}

/// Critério de uma célula de critérios: texto simples casa pelo começo (regra dos bancos
/// de dados do Excel); com operador ou curinga vale como em CONT.SE.
fn criterio(v: &Value) -> Predicate {
    if let Value::Text(t) = v {
        let aparado = t.trim();
        let tem_operador = aparado.starts_with(['=', '<', '>']);
        if !tem_operador && !aparado.contains(['*', '?']) {
            let prefixo = aparado.to_lowercase();
            return Box::new(move |x| match x {
                Value::Text(s) => s.to_lowercase().starts_with(&prefixo),
                _ => false,
            });
        }
    }
    make_predicate(v)
}

fn selecionar(args: &[Node], ev: &Evaluator<'_>, campo_opcional: bool) -> Res<Selecao> {
    if args.len() != 3 {
        return Err(e_valor());
    }
    let banco = matriz_de(ev, &args[0])?;
    let regras = matriz_de(ev, &args[2])?;
    if banco.height() < 1 || banco.width() < 1 {
        return Err(e_valor());
    }
    let cabecalho: Vec<String> = banco.rows()[0].iter().map(|v| to_text(v).to_lowercase()).collect();
    let coluna = if !has(args, 1) {
        if campo_opcional { None } else { return Err(e_valor()) }
    } else {
        match ev.value(&args[1]) {
            Value::Error(er) => return Err(er),
            Value::Number(n) => {
                let i = n.trunc();
                if i < 1.0 || i as usize > cabecalho.len() {
                    return Err(e_valor());
                }
                Some(i as usize - 1)
            }
            Value::Text(t) => {
                let alvo = t.trim().to_lowercase();
                Some(cabecalho.iter().position(|c| *c == alvo).ok_or_else(e_valor)?)
            }
            _ => return Err(e_valor()),
        }
    };

    // Colunas dos critérios que apontam para colunas do banco; None = cabeçalho sem par.
    let colunas_criterio: Vec<Option<usize>> = regras.rows()[0]
        .iter()
        .map(|v| {
            let nome = to_text(v).to_lowercase();
            cabecalho.iter().position(|c| *c == nome)
        })
        .collect();
    // Predicados por linha de critério (só células não vazias).
    let mut alternativas: Vec<Vec<(Option<usize>, Predicate)>> = Vec::new();
    for linha in regras.rows().iter().skip(1) {
        let mut condicoes = Vec::new();
        for (j, v) in linha.iter().enumerate() {
            if matches!(v, Value::Empty) || matches!(v, Value::Text(t) if t.is_empty()) {
                continue;
            }
            condicoes.push((colunas_criterio.get(j).copied().flatten(), criterio(v)));
        }
        alternativas.push(condicoes);
    }

    let mut linhas = Vec::new();
    for linha in banco.rows().iter().skip(1) {
        let casa = alternativas.is_empty()
            || alternativas.iter().any(|conds| {
                conds.iter().all(|(col, p)| match col {
                    Some(c) => p(linha.get(*c).unwrap_or(&Value::Empty)),
                    None => false,
                })
            });
        if casa {
            linhas.push(linha.clone());
        }
    }
    Ok(Selecao { coluna, linhas })
}

fn numeros(s: &Selecao) -> Vec<f64> {
    let c = s.coluna.unwrap_or(0);
    s.linhas
        .iter()
        .filter_map(|l| match l.get(c) {
            Some(Value::Number(n)) => Some(*n),
            _ => None,
        })
        .collect()
}

fn com_numeros(args: &[Node], ev: &Evaluator<'_>, conta: impl Fn(&[f64]) -> Res<f64>) -> Computed {
    concluir(selecionar(args, ev, false).and_then(|s| numero(conta(&numeros(&s))?)))
}

fn variancia(v: &[f64], amostra: bool) -> Res<f64> {
    let n = v.len() as f64;
    if v.is_empty() || (amostra && v.len() < 2) {
        return Err(e_div0());
    }
    let m = v.iter().sum::<f64>() / n;
    let ss: f64 = v.iter().map(|x| (x - m) * (x - m)).sum();
    Ok(ss / if amostra { n - 1.0 } else { n })
}

/// `=BDSOMA(banco; campo; critérios)` / `=DSUM(database; field; criteria)`
pub fn fn_dsum(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    com_numeros(args, ev, |v| Ok(v.iter().sum()))
}

/// `=BDMÉDIA(banco; campo; critérios)` / `=DAVERAGE(database; field; criteria)`
pub fn fn_daverage(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    com_numeros(args, ev, |v| {
        if v.is_empty() { Err(e_div0()) } else { Ok(v.iter().sum::<f64>() / v.len() as f64) }
    })
}

/// `=BDCONTAR(banco; [campo]; critérios)` / `=DCOUNT(database; [field]; criteria)`
pub fn fn_dcount(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    concluir(selecionar(args, ev, true).and_then(|s| {
        numero(match s.coluna {
            None => s.linhas.len(),
            Some(_) => numeros(&s).len(),
        } as f64)
    }))
}

/// `=BDCONTARA(banco; [campo]; critérios)` / `=DCOUNTA(database; [field]; criteria)`
pub fn fn_dcounta(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    concluir(selecionar(args, ev, true).and_then(|s| {
        numero(match s.coluna {
            None => s.linhas.len(),
            Some(c) => s
                .linhas
                .iter()
                .filter(|l| !matches!(l.get(c), None | Some(Value::Empty)))
                .count(),
        } as f64)
    }))
}

/// `=BDEXTRAIR(banco; campo; critérios)` / `=DGET(database; field; criteria)`
pub fn fn_dget(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    concluir(selecionar(args, ev, false).and_then(|s| match s.linhas.len() {
        0 => Err(e_valor()),
        1 => Ok(s.linhas[0].get(s.coluna.unwrap_or(0)).cloned().unwrap_or(Value::Empty)),
        _ => Err(e_num()),
    }))
}

/// `=BDMÁX(banco; campo; critérios)` / `=DMAX(database; field; criteria)`
pub fn fn_dmax(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    com_numeros(args, ev, |v| Ok(v.iter().cloned().fold(None, |a: Option<f64>, x| Some(a.map_or(x, |m| m.max(x)))).unwrap_or(0.0)))
}

/// `=BDMÍN(banco; campo; critérios)` / `=DMIN(database; field; criteria)`
pub fn fn_dmin(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    com_numeros(args, ev, |v| Ok(v.iter().cloned().fold(None, |a: Option<f64>, x| Some(a.map_or(x, |m| m.min(x)))).unwrap_or(0.0)))
}

/// `=BDMULTIPL(banco; campo; critérios)` / `=DPRODUCT(database; field; criteria)`
pub fn fn_dproduct(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    com_numeros(args, ev, |v| Ok(if v.is_empty() { 0.0 } else { v.iter().product() }))
}

/// `=BDEST(banco; campo; critérios)` / `=DSTDEV(database; field; criteria)`
pub fn fn_dstdev(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    com_numeros(args, ev, |v| variancia(v, true).map(f64::sqrt))
}

/// `=BDESTP(banco; campo; critérios)` / `=DSTDEVP(database; field; criteria)`
pub fn fn_dstdevp(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    com_numeros(args, ev, |v| variancia(v, false).map(f64::sqrt))
}

/// `=BDVAREST(banco; campo; critérios)` / `=DVAR(database; field; criteria)`
pub fn fn_dvar(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    com_numeros(args, ev, |v| variancia(v, true))
}

/// `=BDVARP(banco; campo; critérios)` / `=DVARP(database; field; criteria)`
pub fn fn_dvarp(args: &[Node], ev: &Evaluator<'_>) -> Computed {
    com_numeros(args, ev, |v| variancia(v, false))
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::planilha::sheet::Sheet;

    /// Árvores, alturas e lucros da documentação do Excel, com os critérios ao lado.
    fn planilha() -> Sheet {
        let mut s = Sheet::new("Plan1");
        let dados: [[&str; 5]; 7] = [
            ["Árvore", "Altura", "Idade", "Produção", "Lucro"],
            ["Maçã", "18", "20", "14", "105"],
            ["Pera", "12", "12", "10", "96"],
            ["Cereja", "13", "14", "9", "105"],
            ["Maçã", "14", "15", "10", "75"],
            ["Pera", "9", "8", "8", "76,8"],
            ["Maçã", "8", "9", "6", "45"],
        ];
        for (r, linha) in dados.iter().enumerate() {
            for (c, v) in linha.iter().enumerate() {
                s.set_cell_raw(r as u32, c as u32, (*v).into(), None);
            }
        }
        for (c, v) in ["Árvore", "Altura"].iter().enumerate() {
            s.set_cell_raw(9, c as u32, (*v).into(), None);
        }
        s.set_cell_raw(10, 0, "=\"=Maçã\"".into(), None);
        s.set_cell_raw(10, 1, ">10".into(), None);
        s
    }

    #[test]
    fn exemplos_de_banco_de_dados() {
        let mut s = planilha();
        let mut v = |f: &str| {
            s.set_cell_raw(20, 0, format!("={f}"), None);
            s.get_value(20, 0)
        };
        assert_eq!(v("DSUM(A1:E7; \"Lucro\"; A10:B11)"), Value::Number(180.0));
        assert_eq!(v("DCOUNT(A1:E7; \"Idade\"; A10:B11)"), Value::Number(2.0));
        assert_eq!(v("DMAX(A1:E7; \"Lucro\"; A10:B11)"), Value::Number(105.0));
        assert_eq!(v("DAVERAGE(A1:E7; 3; A10:B11)"), Value::Number(17.5));
    }
}
