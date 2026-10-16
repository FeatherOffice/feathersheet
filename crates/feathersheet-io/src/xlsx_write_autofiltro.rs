//! `<autoFilter>` com os critérios de coluna (`filterColumn`) e a
//! ordenação (`sortState`) da tabela.
//!
//! Critérios gravados: lista (`filters`), cor (`colorFilter` + `dxf`),
//! 10 primeiros (`top10`) e período/média (`dynamicFilter`). Os demais
//! (regras de texto/número, `CONT.SE`) não têm gravação ainda e saem sem
//! `filterColumn` — as linhas escondidas continuam escondidas.

use crate::filtro_json::{criterio_de_json, ordenacao_de_json};
use crate::xlsx_write::escape_xml;
use feathersheet_core::celulas::refs::col_name;
use feathersheet_core::celulas::values::Value;
use feathersheet_core::operacoes::filtro::{AlvoCor, Criterio};
use feathersheet_core::operacoes::ordenacao_ext::CriterioOrdem;
use feathersheet_core::operacoes::rules::CondStyle;
use serde_json::Value as Json;
use std::fmt::Write as _;

fn dxf_da_cor(alvo: AlvoCor, cor: &Option<String>, dxf: &mut dyn FnMut(&CondStyle) -> Option<usize>) -> String {
    let Some(c) = cor else { return String::new() };
    let estilo = match alvo {
        AlvoCor::Fundo => CondStyle { bg: Some(c.clone()), ..Default::default() },
        AlvoCor::Fonte => CondStyle { color: Some(c.clone()), ..Default::default() },
    };
    dxf(&estilo).map(|i| format!(" dxfId=\"{i}\"")).unwrap_or_default()
}

fn texto(v: &Value) -> String {
    feathersheet_core::operacoes::filtro::texto_js(v)
}

/// O XML de um critério de coluna, ou `None` se não tem forma no `.xlsx`.
fn criterio_xml(c: &Criterio, dxf: &mut dyn FnMut(&CondStyle) -> Option<usize>) -> Option<String> {
    Some(match c {
        Criterio::Lista(l) => {
            let mut itens = String::new();
            let mut vazio = false;
            for v in l {
                let t = texto(v);
                if t.is_empty() {
                    vazio = true;
                } else {
                    let _ = write!(itens, "<filter val=\"{}\"/>", escape_xml(&t));
                }
            }
            let blank = if vazio { " blank=\"1\"" } else { "" };
            format!("<filters{blank}>{itens}</filters>")
        }
        Criterio::Cor { alvo, cor } => {
            let d = dxf_da_cor(*alvo, cor, dxf);
            let celula = if *alvo == AlvoCor::Fonte { " cellColor=\"0\"" } else { "" };
            format!("<colorFilter{d}{celula}/>")
        }
        Criterio::Top10 { topo, n, percentual } => {
            let topo = if *topo { "" } else { " top=\"0\"" };
            let pct = if *percentual { " percent=\"1\"" } else { "" };
            format!("<top10{topo}{pct} val=\"{n}\"/>")
        }
        Criterio::Dinamico(d) => format!("<dynamicFilter type=\"{}\"/>", d.nome_xlsx()),
        _ => return None,
    })
}

fn ref_celula(r: u32, c: u32) -> String {
    format!("{}{}", col_name(c), r + 1)
}

/// `<autoFilter ref=…>` da tabela `t` (JSON), área `(r0, c0, r1, c1)`.
pub(crate) fn autofiltro_xml(
    area: (u32, u32, u32, u32),
    t: &Json,
    dxf: &mut dyn FnMut(&CondStyle) -> Option<usize>,
) -> String {
    let (r0, c0, r1, c1) = area;
    let mut dentro = String::new();
    if let Some(fs) = t["filtros"].as_array() {
        for f in fs {
            let Some(rel) = f["col"].as_u64().and_then(|x| u32::try_from(x).ok()) else { continue };
            if c0 + rel > c1 {
                continue;
            }
            if let Some(x) = criterio_xml(&criterio_de_json(&f["criterio"]), dxf) {
                let _ = write!(dentro, "<filterColumn colId=\"{rel}\">{x}</filterColumn>");
            }
        }
    }
    if t["ordenacao"].is_object() {
        let (chaves, por_colunas) = ordenacao_de_json(&t["ordenacao"], c0, r0);
        let primeira = r0 + u32::from(t["temCabecalho"].as_bool().unwrap_or(true));
        if !chaves.is_empty() && primeira <= r1 {
            let ref_area = ref_celula(if por_colunas { r0 } else { primeira }, c0) + ":" + &ref_celula(r1, c1);
            let mut conds = String::new();
            for k in &chaves {
                let (ini, fim) = if por_colunas {
                    (ref_celula(k.indice, c0), ref_celula(k.indice, c1))
                } else {
                    (ref_celula(primeira, k.indice), ref_celula(r1, k.indice))
                };
                let mut attrs = String::new();
                match &k.criterio {
                    CriterioOrdem::Valor => {
                        if !k.ascendente { attrs.push_str(" descending=\"1\""); }
                    }
                    CriterioOrdem::Lista(l) => {
                        if !k.ascendente { attrs.push_str(" descending=\"1\""); }
                        let _ = write!(attrs, " customList=\"{}\"", escape_xml(&l.join(",")));
                    }
                    CriterioOrdem::Cor { alvo, cor, em_cima } => {
                        if !em_cima { attrs.push_str(" descending=\"1\""); }
                        let por = if *alvo == AlvoCor::Fundo { "cellColor" } else { "fontColor" };
                        let _ = write!(attrs, " sortBy=\"{por}\"{}", dxf_da_cor(*alvo, cor, dxf));
                    }
                }
                let _ = write!(conds, "<sortCondition{attrs} ref=\"{ini}:{fim}\"/>");
            }
            let col = if por_colunas { " columnSort=\"1\"" } else { "" };
            let _ = write!(dentro, "<sortState{col} ref=\"{ref_area}\">{conds}</sortState>");
        }
    }
    let ref_total = format!("{}:{}", ref_celula(r0, c0), ref_celula(r1, c1));
    if dentro.is_empty() {
        format!("<autoFilter ref=\"{ref_total}\"/>")
    } else {
        format!("<autoFilter ref=\"{ref_total}\">{dentro}</autoFilter>")
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use serde_json::json;

    fn dxf_teste(dxfs: &mut Vec<CondStyle>) -> impl FnMut(&CondStyle) -> Option<usize> + '_ {
        move |e| {
            if let Some(i) = dxfs.iter().position(|x| x == e) { return Some(i); }
            dxfs.push(e.clone());
            Some(dxfs.len() - 1)
        }
    }

    #[test]
    fn sem_criterios_continua_igual_ao_de_antes() {
        let t = json!({"filtros": []});
        let mut d = Vec::new();
        assert_eq!(autofiltro_xml((0, 0, 9, 2), &t, &mut dxf_teste(&mut d)), "<autoFilter ref=\"A1:C10\"/>");
    }

    #[test]
    fn grava_cor_top10_dinamico_e_ordenacao() {
        let t = json!({
            "temCabecalho": true,
            "filtros": [
                {"col": 0, "criterio": {"op": "cor_fundo", "cor": "#ff0000"}},
                {"col": 1, "criterio": {"op": "cor_fonte", "cor": "#0000ff"}},
                {"col": 2, "criterio": {"op": "top10", "topo": false, "n": 3, "percentual": true}},
                {"col": 3, "criterio": {"op": "dinamico", "tipo": "lastQuarter"}},
                {"col": 4, "criterio": ["a", ""]},
                {"col": 5, "criterio": {"op": "contains", "val": "x"}},
            ],
            "ordenacao": {"por_colunas": false, "niveis": [
                {"indice": 1, "descendente": true, "por": "cor_fundo", "cor": "#ff0000"},
                {"indice": 2, "descendente": false, "por": "valor", "lista": ["seg", "ter"]},
            ]},
        });
        let mut d = Vec::new();
        let xml = autofiltro_xml((0, 0, 9, 5), &t, &mut dxf_teste(&mut d));
        assert!(xml.contains("<filterColumn colId=\"0\"><colorFilter dxfId=\"0\"/></filterColumn>"), "{xml}");
        assert!(xml.contains("<filterColumn colId=\"1\"><colorFilter dxfId=\"1\" cellColor=\"0\"/></filterColumn>"), "{xml}");
        assert!(xml.contains("<top10 top=\"0\" percent=\"1\" val=\"3\"/>"), "{xml}");
        assert!(xml.contains("<dynamicFilter type=\"lastQuarter\"/>"), "{xml}");
        assert!(xml.contains("<filters blank=\"1\"><filter val=\"a\"/></filters>"), "{xml}");
        assert!(!xml.contains("colId=\"5\""), "regra de texto não tem forma ainda");
        assert!(xml.contains("<sortState ref=\"A2:F10\">"), "{xml}");
        assert!(xml.contains("<sortCondition descending=\"1\" sortBy=\"cellColor\" dxfId=\"0\" ref=\"B2:B10\"/>"), "{xml}");
        assert!(xml.contains("<sortCondition customList=\"seg,ter\" ref=\"C2:C10\"/>"), "{xml}");
        assert_eq!(d.len(), 2);
    }

    #[test]
    fn ordenacao_da_esquerda_para_a_direita() {
        let t = json!({"temCabecalho": false, "filtros": [],
            "ordenacao": {"por_colunas": true, "niveis": [{"indice": 1, "descendente": false, "por": "valor"}]}});
        let mut d = Vec::new();
        let xml = autofiltro_xml((0, 0, 9, 3), &t, &mut dxf_teste(&mut d));
        assert!(xml.contains("<sortState columnSort=\"1\" ref=\"A1:D10\"><sortCondition ref=\"A2:D2\"/></sortState>"), "{xml}");
    }
}
