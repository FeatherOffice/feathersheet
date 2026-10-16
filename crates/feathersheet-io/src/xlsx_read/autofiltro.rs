//! `<autoFilter>` de uma tabela do `.xlsx`: os critérios de cada coluna
//! (`filterColumn`) e a ordenação (`sortState`), no JSON que a tabela guarda
//! (ver `crate::filtro_json`).
//!
//! Lidos: `filters` (lista), `colorFilter` (cor de preenchimento ou da fonte,
//! pelo `dxf`), `top10` e `dynamicFilter`. Os `customFilters` (regras) ainda
//! não viram critério; o que o Excel escondeu continua escondido (a linha vem
//! com altura zero).

use super::styles::{dxf_do_indice, XfStyle};
use super::xml::XmlNode;
use crate::filtro_json::{criterio_para_json, ordenacao_para_json};
use feathersheet_core::celulas::values::Value;
use feathersheet_core::operacoes::filtro::{AlvoCor, Criterio};
use feathersheet_core::operacoes::filtro_data::Dinamico;
use feathersheet_core::operacoes::ordenacao_ext::{ChaveOrdem, CriterioOrdem};
use serde_json::{json, Value as Json};

fn criterio_da_coluna(coluna: &XmlNode, xfs: &[XfStyle]) -> Option<Criterio> {
    if let Some(f) = coluna.child("filters") {
        let mut itens: Vec<Value> = f.children_named("filter").filter_map(|x| x.attr("val")).map(|v| Value::Text(v.to_string())).collect();
        if f.attr("blank") == Some("1") {
            itens.push(Value::Empty);
        }
        return (!itens.is_empty()).then_some(Criterio::Lista(itens));
    }
    if let Some(c) = coluna.child("colorFilter") {
        let alvo = if matches!(c.attr("cellColor"), Some("0") | Some("false")) { AlvoCor::Fonte } else { AlvoCor::Fundo };
        let dxf = c.attr("dxfId").and_then(|s| s.parse::<usize>().ok()).and_then(|i| dxf_do_indice(xfs, i));
        let cor = dxf.and_then(|d| if alvo == AlvoCor::Fundo { d.bg } else { d.color });
        return Some(Criterio::Cor { alvo, cor });
    }
    if let Some(t) = coluna.child("top10") {
        let n = t.attr("val").and_then(|v| v.parse::<f64>().ok()).filter(|n| *n >= 0.0)?;
        return Some(Criterio::Top10 {
            topo: !matches!(t.attr("top"), Some("0") | Some("false")),
            n: n as u32,
            percentual: matches!(t.attr("percent"), Some("1") | Some("true")),
        });
    }
    if let Some(d) = coluna.child("dynamicFilter") {
        return Dinamico::de_nome_xlsx(d.attr("type")?).map(Criterio::Dinamico);
    }
    None
}

/// Índice de coluna e linha (0-based) do início de uma referência `B2:B10`.
fn inicio_da_ref(r: &str) -> Option<(u32, u32)> {
    let ini = r.split(':').next()?.replace('$', "");
    let c = super::refs::parse_cell_ref(&ini)?;
    Some((u32::try_from(c.row).ok()?, u32::try_from(c.col).ok()?))
}

fn ordenacao(sort: &XmlNode, xfs: &[XfStyle], r0: u32, c0: u32) -> Option<Json> {
    let por_colunas = matches!(sort.attr("columnSort"), Some("1") | Some("true"));
    let mut chaves = Vec::new();
    for cond in sort.children_named("sortCondition") {
        let (linha, coluna) = inicio_da_ref(cond.attr("ref")?)?;
        let indice = if por_colunas { linha } else { coluna };
        let descendente = matches!(cond.attr("descending"), Some("1") | Some("true"));
        let cor_do_dxf = |alvo: AlvoCor| {
            cond.attr("dxfId")
                .and_then(|s| s.parse::<usize>().ok())
                .and_then(|i| dxf_do_indice(xfs, i))
                .and_then(|d| if alvo == AlvoCor::Fundo { d.bg } else { d.color })
        };
        let criterio = match cond.attr("sortBy") {
            Some("cellColor") => CriterioOrdem::Cor { alvo: AlvoCor::Fundo, cor: cor_do_dxf(AlvoCor::Fundo), em_cima: !descendente },
            Some("fontColor") => CriterioOrdem::Cor { alvo: AlvoCor::Fonte, cor: cor_do_dxf(AlvoCor::Fonte), em_cima: !descendente },
            Some("icon") => continue,
            _ => match cond.attr("customList") {
                Some(l) if !l.is_empty() => CriterioOrdem::Lista(l.split(',').map(|s| s.trim().to_string()).collect()),
                _ => CriterioOrdem::Valor,
            },
        };
        chaves.push(ChaveOrdem { indice, ascendente: !descendente, criterio });
    }
    // Os índices guardados são relativos à tabela: `ordenacao_para_json` desconta a origem.
    (!chaves.is_empty()).then(|| ordenacao_para_json(&chaves, por_colunas, c0, r0))
}

/// Os `filtros` e a `ordenacao` do `<autoFilter>` da tabela cuja área começa
/// em `(r0, c0)` (0-based).
pub(super) fn ler(auto: &XmlNode, xfs: &[XfStyle], r0: u32, c0: u32) -> (Vec<Json>, Option<Json>) {
    let filtros = auto
        .children_named("filterColumn")
        .filter_map(|c| {
            let col: u32 = c.attr("colId")?.parse().ok()?;
            let crit = criterio_da_coluna(c, xfs)?;
            Some(json!({"col": col, "criterio": criterio_para_json(&crit)}))
        })
        .collect();
    let ord = auto.child("sortState").and_then(|s| ordenacao(s, xfs, r0, c0));
    (filtros, ord)
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::xlsx_read::xml::parse_xml;
    use feathersheet_core::operacoes::rules::CondStyle;

    fn xfs() -> Vec<XfStyle> {
        let dxf = |bg: Option<&str>, color: Option<&str>| XfStyle {
            style: Default::default(),
            format: "general",
            dxf: Some(CondStyle { bg: bg.map(String::from), color: color.map(String::from), ..Default::default() }),
        };
        vec![
            XfStyle { style: Default::default(), format: "general", dxf: None },
            dxf(Some("#ff0000"), None),
            dxf(None, Some("#0000ff")),
        ]
    }

    #[test]
    fn le_cor_top10_dinamico_lista_e_ordenacao() {
        let xml = r#"<autoFilter ref="B2:F20">
          <filterColumn colId="0"><colorFilter dxfId="0"/></filterColumn>
          <filterColumn colId="1"><colorFilter dxfId="1" cellColor="0"/></filterColumn>
          <filterColumn colId="2"><top10 top="0" percent="1" val="25"/></filterColumn>
          <filterColumn colId="3"><dynamicFilter type="thisYear" val="46000"/></filterColumn>
          <filterColumn colId="4"><filters blank="1"><filter val="x"/></filters></filterColumn>
          <filterColumn colId="5"><customFilters><customFilter operator="equal" val="1"/></customFilters></filterColumn>
          <sortState ref="B3:F20">
            <sortCondition descending="1" sortBy="cellColor" ref="C3:C20" dxfId="0"/>
            <sortCondition sortBy="fontColor" ref="D3:D20" dxfId="1"/>
            <sortCondition ref="E3:E20" customList="seg,ter,qua"/>
          </sortState></autoFilter>"#;
        let no = parse_xml(xml).unwrap();
        let (f, o) = ler(&no, &xfs(), 1, 1);
        let crit: Vec<(u64, Criterio)> = f.iter().map(|x| (x["col"].as_u64().unwrap(), crate::filtro_json::criterio_de_json(&x["criterio"]))).collect();
        assert_eq!(crit.len(), 5, "customFilters ficam de fora");
        assert_eq!(crit[0].1, Criterio::Cor { alvo: AlvoCor::Fundo, cor: Some("#ff0000".into()) });
        assert_eq!(crit[1].1, Criterio::Cor { alvo: AlvoCor::Fonte, cor: Some("#0000ff".into()) });
        assert_eq!(crit[2].1, Criterio::Top10 { topo: false, n: 25, percentual: true });
        assert_eq!(crit[3].1, Criterio::Dinamico(Dinamico::EsteAno));
        assert_eq!(crit[4].1, Criterio::Lista(vec![Value::Text("x".into()), Value::Empty]));
        let (chaves, por_colunas) = crate::filtro_json::ordenacao_de_json(&o.unwrap(), 1, 1);
        assert!(!por_colunas);
        assert_eq!(chaves[0], ChaveOrdem { indice: 2, ascendente: false, criterio: CriterioOrdem::Cor { alvo: AlvoCor::Fundo, cor: Some("#ff0000".into()), em_cima: false } });
        assert_eq!(chaves[1].criterio, CriterioOrdem::Cor { alvo: AlvoCor::Fonte, cor: Some("#0000ff".into()), em_cima: true });
        assert_eq!(chaves[1].indice, 3);
        assert_eq!(chaves[2].criterio, CriterioOrdem::Lista(vec!["seg".into(), "ter".into(), "qua".into()]));
    }

    #[test]
    fn ordenacao_de_colunas_usa_a_linha() {
        let xml = r#"<autoFilter ref="A1:D5"><sortState columnSort="1" ref="A1:D5"><sortCondition ref="A3:D3"/></sortState></autoFilter>"#;
        let no = parse_xml(xml).unwrap();
        let (f, o) = ler(&no, &[], 0, 0);
        assert!(f.is_empty());
        let (chaves, por_colunas) = crate::filtro_json::ordenacao_de_json(&o.unwrap(), 0, 0);
        assert!(por_colunas);
        assert_eq!(chaves[0].indice, 2, "linha 3 (índice 2)");
    }
}
