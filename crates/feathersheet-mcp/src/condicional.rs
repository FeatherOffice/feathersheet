//! `sheet_condicional`: listar, criar, remover e limpar regras de
//! formatação condicional — as mesmas `ConditionalRule` da janela da faixa
//! (`operacoes::rules`), gravadas no `r` da aba com Ctrl+Z. Funciona com e
//! sem o aplicativo: a regra mora no motor, e a grade só a desenha.
use feathersheet_core::operacoes::rules::{
    CellCondition, CellOp, ColorScaleCondition, CondStyle, Condition, ConditionalRule, DataBarCondition, Faixa,
    ScalePoint, ScalePointType,
};
use serde_json::Value as Json;

use crate::ferramentas::Saida;
use crate::hospede::Hospede;
use crate::texto::*;

/// O nome em português do tipo de regra → o operador do motor.
const TIPOS: &[(&str, CellOp)] = &[
    ("maior", CellOp::GreaterThan), ("maior_igual", CellOp::GreaterOrEqual), ("menor", CellOp::LessThan),
    ("menor_igual", CellOp::LessOrEqual), ("igual", CellOp::Equal), ("diferente", CellOp::NotEqual),
    ("entre", CellOp::Between), ("fora", CellOp::NotBetween), ("vazia", CellOp::IsEmpty),
    ("nao_vazia", CellOp::IsNotEmpty), ("contem", CellOp::TextContains), ("nao_contem", CellOp::TextNotContains),
    ("comeca", CellOp::TextStartsWith), ("termina", CellOp::TextEndsWith), ("antes", CellOp::DateBefore),
    ("depois", CellOp::DateAfter), ("na_data", CellOp::DateOn), ("duplicados", CellOp::Duplicate),
    ("unicos", CellOp::Unique), ("acima_media", CellOp::AboveAverage), ("abaixo_media", CellOp::BelowAverage),
    ("formula", CellOp::Formula),
];

/// Quantos valores cada operador pede.
fn aridade(op: CellOp) -> usize {
    match op {
        CellOp::Between | CellOp::NotBetween => 2,
        CellOp::IsEmpty | CellOp::IsNotEmpty | CellOp::Duplicate | CellOp::Unique
            | CellOp::AboveAverage | CellOp::BelowAverage => 0,
        _ => 1,
    }
}

fn hex(v: &Json, campo: &str) -> Result<String, String> {
    let t = v.as_str().map(str::trim).ok_or_else(|| format!("`{campo}`: use \"#RRGGBB\"."))?;
    let h = t.trim_start_matches('#');
    if !(h.len() == 3 || h.len() == 6) || !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("`{campo}`: \"{t}\" não é cor; use \"#RRGGBB\" (ex.: \"#f4cccc\")."));
    }
    Ok(format!("#{}", h.to_ascii_lowercase()))
}

fn estilo(args: &Json) -> Result<CondStyle, String> {
    let e = &args["estilo"];
    let cor = |k: &str| e.get(k).filter(|v| !v.is_null()).map(|v| hex(v, k)).transpose();
    let logico = |k: &str| e.get(k).and_then(Json::as_bool);
    let mut s = CondStyle {
        bg: cor("fundo")?, color: cor("cor")?, bold: logico("negrito"), italic: logico("italico"),
        underline: logico("sublinhado"), strike: logico("tachado"),
    };
    if s == CondStyle::default() {
        // Sem estilo: o vermelho claro do Excel.
        s.bg = Some("#ffc7ce".into());
        s.color = Some("#9c0006".into());
    }
    Ok(s)
}

fn faixa_de(r: &feathersheet_core::celulas::refs::Rect) -> Faixa {
    Faixa { r0: r.r0, c0: r.c0, r1: r.r1, c1: r.c1 }
}

fn regras(h: &dyn Hospede, aba: usize) -> Result<Vec<ConditionalRule>, String> {
    let sheet = &h.pasta().aba(aba).ok_or("Aba inexistente")?.sheet;
    match sheet.r.as_deref() {
        None => Ok(Vec::new()),
        Some(t) => serde_json::from_str(t).map_err(|_| "As regras condicionais desta aba estão ilegíveis; nada foi alterado.".to_string()),
    }
}

fn gravar(h: &mut dyn Hospede, aba: usize, lista: &[ConditionalRule], area: &feathersheet_core::celulas::refs::Rect) -> Result<(), String> {
    let texto = (!lista.is_empty()).then(|| serde_json::to_string(lista).expect("regras em memória"));
    h.pasta_mut().editar(aba, |sheet| { sheet.alterar_condicionais(texto, area); });
    h.marcar_alterado();
    Ok(())
}

fn descrever(r: &ConditionalRule) -> String {
    let onde: Vec<String> = r.ranges.iter()
        .map(|f| nome_area(&feathersheet_core::celulas::refs::Rect::new(f.r0, f.c0, f.r1, f.c1))).collect();
    let o_que = match &r.condition {
        Condition::Cell(c) => {
            let nome = TIPOS.iter().find(|(_, op)| *op == c.op).map_or("?", |(n, _)| n);
            let cores = [c.style.bg.as_deref().map(|b| format!("fundo {b}")), c.style.color.as_deref().map(|b| format!("cor {b}"))]
                .into_iter().flatten().collect::<Vec<_>>().join(", ");
            format!("{nome} {} → {cores}", c.values.join(" e "))
        }
        Condition::ColorScale(e) => format!("escala {} → {}{}", e.min.color,
            e.mid.as_ref().map(|m| format!("{} → ", m.color)).unwrap_or_default(), e.max.color),
        Condition::DataBar(b) => format!("barra {}", b.color),
    };
    format!("{} em {}: {o_que}", r.id, onde.join(", "))
}

pub fn condicional(h: &mut dyn Hospede, args: &Json) -> Saida {
    let acao = arg_texto(args, "acao").unwrap_or("listar");
    match acao {
        "listar" => {
            let aba = aba_do_arg(h, args)?;
            let lista = regras(h, aba)?;
            let nome = &h.pasta().nomes()[aba];
            Ok(conteudo(if lista.is_empty() {
                format!("Nenhuma regra condicional em {nome}.")
            } else {
                format!("{} regra(s) em {nome}, na ordem em que valem:\n{}", lista.len(),
                    lista.iter().map(descrever).collect::<Vec<_>>().join("\n"))
            }))
        }
        "criar" => {
            let (aba, area) = intervalo(h, args, "intervalo")?;
            let tipo = arg_texto(args, "tipo").ok_or("Falta `tipo` (ex.: maior, entre, contem, duplicados, formula, escala, barra).")?;
            let condicao = match tipo {
                "escala" => {
                    let cores = args["cores"].as_array().filter(|c| c.len() == 2 || c.len() == 3)
                        .ok_or("`cores`: duas ou três cores, da menor para a maior (ex.: [\"#f8696b\", \"#ffeb84\", \"#63be7b\"]).")?;
                    let ponta = |i: usize, t: ScalePointType| -> Result<ScalePoint, String> {
                        Ok(ScalePoint { point_type: t, value: (t == ScalePointType::Percentile).then_some(50.0), color: hex(&cores[i], "cores")? })
                    };
                    Condition::ColorScale(ColorScaleCondition {
                        min: ponta(0, ScalePointType::Min)?,
                        mid: if cores.len() == 3 { Some(ponta(1, ScalePointType::Percentile)?) } else { None },
                        max: ponta(cores.len() - 1, ScalePointType::Max)?,
                    })
                }
                "barra" => Condition::DataBar(DataBarCondition {
                    color: hex(args.get("cor").unwrap_or(&Json::String("#638ec6".into())), "cor")?,
                    negative_color: None,
                    hide_value: args["so_barra"].as_bool().unwrap_or(false),
                    min: None, max: None,
                }),
                _ => {
                    let op = TIPOS.iter().find(|(n, _)| *n == tipo).map(|(_, op)| *op)
                        .ok_or_else(|| format!("Tipo \"{tipo}\" desconhecido. Tipos: {}, escala, barra.",
                            TIPOS.iter().map(|(n, _)| *n).collect::<Vec<_>>().join(", ")))?;
                    let values: Vec<String> = args["valores"].as_array().map(|l| l.iter().map(conteudo_da_ia).collect()).unwrap_or_default();
                    if values.len() < aridade(op) {
                        return Err(format!("`{tipo}` pede {} valor(es) em `valores`{}.", aridade(op),
                            if op == CellOp::Formula { " (a fórmula da primeira célula, ex.: \"=$C2>100\")" } else { "" }));
                    }
                    Condition::Cell(CellCondition { op, values: values.into_iter().take(aridade(op)).collect(), style: estilo(args)? })
                }
            };
            let mut lista = regras(h, aba)?;
            let id = (1..).map(|n| format!("mcp_{n}")).find(|c| !lista.iter().any(|r| &r.id == c)).expect("sempre há id livre");
            let regra = ConditionalRule { id, ranges: vec![faixa_de(&area)], condition: condicao };
            let texto = descrever(&regra);
            lista.push(regra);
            gravar(h, aba, &lista, &area)?;
            Ok(conteudo(format!("Regra criada: {texto}")))
        }
        "remover" => {
            let aba = aba_do_arg(h, args)?;
            let id = arg_texto(args, "id").ok_or("Falta `id` (veja com acao listar).")?;
            let mut lista = regras(h, aba)?;
            let antes = lista.len();
            let area = lista.iter().find(|r| r.id == id).and_then(|r| r.ranges.first())
                .map(|f| feathersheet_core::celulas::refs::Rect::new(f.r0, f.c0, f.r1, f.c1))
                .ok_or_else(|| format!("Nenhuma regra com id \"{id}\"."))?;
            lista.retain(|r| r.id != id);
            gravar(h, aba, &lista, &area)?;
            Ok(conteudo(format!("Regra {id} removida ({} restante(s)).", antes - 1)))
        }
        "limpar" => {
            // Com intervalo, só as regras que passam por ele; sem, a aba toda.
            let (aba, area) = match arg_texto(args, "intervalo") {
                Some(_) => { let (a, r) = intervalo(h, args, "intervalo")?; (a, Some(r)) }
                None => (aba_do_arg(h, args)?, None),
            };
            let mut lista = regras(h, aba)?;
            let antes = lista.len();
            lista.retain(|r| area.as_ref().is_some_and(|a| !r.ranges.iter()
                .any(|f| f.r0 <= a.r1 && a.r0 <= f.r1 && f.c0 <= a.c1 && a.c0 <= f.c1)));
            let toda = feathersheet_core::celulas::refs::Rect::new(0, 0, 0, 0);
            gravar(h, aba, &lista, area.as_ref().unwrap_or(&toda))?;
            Ok(conteudo(format!("{} regra(s) removida(s).", antes - lista.len())))
        }
        outra => Err(format!("Ação \"{outra}\" desconhecida: listar, criar, remover ou limpar.")),
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::ferramentas::chamar;
    use crate::hospede::Avulso;
    use serde_json::json;

    fn texto(r: Saida) -> String {
        r.unwrap().iter().filter_map(|c| c["text"].as_str()).collect::<Vec<_>>().join("\n")
    }

    #[test]
    fn criar_listar_pintar_e_desfazer() {
        let mut h = Avulso::default();
        for (r, v) in ["5", "150", "80"].iter().enumerate() {
            h.pasta.escrever_celula(0, r as u32, 0, (*v).into());
        }
        let t = texto(chamar(&mut h, "sheet_condicional", &json!({ "acao": "criar", "intervalo": "A1:A3",
            "tipo": "maior", "valores": [100], "estilo": { "fundo": "#f4cccc", "negrito": true } })));
        assert!(t.contains("mcp_1") && t.contains("maior 100"), "{t}");
        texto(chamar(&mut h, "sheet_condicional", &json!({ "acao": "criar", "intervalo": "B1:B3", "tipo": "escala",
            "cores": ["#ffffff", "#63be7b"] })));
        let t = texto(chamar(&mut h, "sheet_condicional", &json!({ "acao": "listar" })));
        assert!(t.contains("2 regra(s)") && t.contains("escala"), "{t}");
        // A regra gravada é a que a grade usa.
        let lista = regras(&h, 0).unwrap();
        let mut motor = feathersheet_core::operacoes::rules::ConditionalFormats::new();
        motor.set_rules(lista);
        assert!(matches!(&motor.rules[0].condition, Condition::Cell(c) if c.style.bold == Some(true)));
        texto(chamar(&mut h, "sheet_desfazer", &json!({})));
        assert_eq!(regras(&h, 0).unwrap().len(), 1, "um Ctrl+Z tira a última regra");
        let t = texto(chamar(&mut h, "sheet_condicional", &json!({ "acao": "remover", "id": "mcp_1" })));
        assert!(t.contains("removida"), "{t}");
        assert!(h.pasta.aba(0).unwrap().sheet.r.is_none());
    }

    #[test]
    fn tipo_errado_e_valor_faltando_explicam() {
        let mut h = Avulso::default();
        let e = chamar(&mut h, "sheet_condicional", &json!({ "acao": "criar", "intervalo": "A1:A3", "tipo": "maior" })).unwrap_err();
        assert!(e.contains("1 valor"), "{e}");
        let e = chamar(&mut h, "sheet_condicional", &json!({ "acao": "criar", "intervalo": "A1:A3", "tipo": "xyz" })).unwrap_err();
        assert!(e.contains("duplicados"), "{e}");
    }
}
