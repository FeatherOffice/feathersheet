//! Validação de dados do `.xlsx`: `<dataValidations>` e o bloco do Excel
//! novo em `<extLst>` (`x14:dataValidation`, onde ficam as listas que leem
//! outra aba). Vira o `Vec<Validacao>` do motor (JSON `Sheet.va`).
//!
//! Lista: `"SIM,NÃO"` vira lista fixa; intervalo da própria aba vira
//! intervalo; o resto (outra aba, nome, `INDIRECT`) vira lista por fórmula,
//! resolvida pelo motor (`planilha::lista_validacao`). Número inteiro e
//! decimal com os oito operadores; data com os que o motor tem; hora com os
//! oito operadores. Personalizada volta como regra do motor quando a fórmula é
//! a que `xlsx_write` grava (caixa, e-mail, URL, contém, começa com). Tamanho
//! do texto e outras personalizadas ficam de fora (anotado no diário).
//! Também lê `errorStyle`, `errorTitle`, `error`, `showInputMessage`,
//! `promptTitle` e `prompt`.
use super::formula::formula_from_excel;
use super::xml::XmlNode;
use feathersheet_core::celulas::refs::{parse_range_ref, Rect};
use feathersheet_core::operacoes::validation::{
    formatar_hora, DataOp, MensagensValidacao, NumeroOp, QuandoValidacao, Regra, Validacao,
};

/// As regras da aba, e quantas ficaram de fora por não terem equivalente.
pub(super) fn ler(root: &XmlNode) -> (Vec<Validacao>, usize) {
    let mut saida = Vec::new();
    let mut fora = 0;
    let classicas = root.child("dataValidations").into_iter().flat_map(|d| d.children_named("dataValidation"));
    // `<extLst><ext><x14:dataValidations>`: o leitor tira o prefixo.
    let novas = root.child("extLst").into_iter()
        .flat_map(|e| e.children_named("ext"))
        .flat_map(|e| e.children_named("dataValidations"))
        .flat_map(|d| d.children_named("dataValidation"));
    for dv in classicas.chain(novas) {
        match converter(dv) {
            Some(v) => saida.push(v),
            None => fora += 1,
        }
    }
    (saida, fora)
}

/// `formula1`/`formula2`: texto direto (clássica) ou dentro de `<xm:f>` (x14).
fn formula(dv: &XmlNode, nome: &str) -> Option<String> {
    let no = dv.child(nome)?;
    let texto = no.child("f").map_or(no.text.as_str(), |f| f.text.as_str()).trim();
    (!texto.is_empty()).then(|| texto.to_string())
}

fn intervalos(dv: &XmlNode) -> Vec<Rect> {
    let sqref = dv.attr("sqref").map(str::to_string)
        .or_else(|| dv.child("sqref").map(|s| s.text.clone()))
        .unwrap_or_default();
    sqref.split_whitespace().filter_map(|r| parse_range_ref(&r.replace('$', ""))).collect()
}

fn numero(texto: &str) -> Option<f64> {
    texto.trim().parse::<f64>().ok().filter(|n| n.is_finite())
}

/// Número de série do Excel → `DD/MM/AAAA`, o formato que o motor compara.
fn data(texto: &str) -> Option<String> {
    let serie = numero(texto)?;
    let (y, m, d) = super::sheet::dias_para_data(serie.floor() as i64 - 25_569);
    Some(format!("{d:02}/{m:02}/{y:04}"))
}

fn lista(f1: &str) -> Regra {
    if let Some(dentro) = f1.strip_prefix('"').and_then(|t| t.strip_suffix('"')) {
        let valores = dentro.split(',').map(|v| v.trim().to_string()).filter(|v| !v.is_empty()).collect();
        return Regra::Lista { valores };
    }
    let convertida = formula_from_excel(&format!("={f1}"));
    match parse_range_ref(&f1.replace('$', "")) {
        Some(origem) if origem.sheet.is_none() => Regra::Intervalo { origem },
        _ => Regra::Formula { formula: convertida },
    }
}

fn converter(dv: &XmlNode) -> Option<Validacao> {
    let ranges = intervalos(dv);
    if ranges.is_empty() { return None; }
    let f1 = formula(dv, "formula1");
    let f2 = formula(dv, "formula2");
    let operador = dv.attr("operator").unwrap_or("between");
    let regra = match dv.attr("type").unwrap_or("none") {
        "list" => lista(f1.as_deref()?),
        tipo @ ("whole" | "decimal") => {
            let a = numero(f1.as_deref()?)?;
            let b = f2.as_deref().and_then(numero);
            let op = match operator_numero(operador) { Some(op) => op, None => return None };
            let b = if matches!(op, NumeroOp::Entre | NumeroOp::Fora) { Some(b?) } else { None };
            Regra::Numero { op, a, b, inteiro: tipo == "whole" }
        }
        "date" => {
            let a = data(f1.as_deref()?)?;
            let b = f2.as_deref().and_then(data);
            match operador {
                "between" => Regra::Data { op: DataOp::Entre, a, b: Some(b?) },
                "equal" => Regra::Data { op: DataOp::Entre, a: a.clone(), b: Some(a) },
                "greaterThan" | "greaterThanOrEqual" => Regra::Data { op: DataOp::Depois, a, b: None },
                "lessThan" | "lessThanOrEqual" => Regra::Data { op: DataOp::Antes, a, b: None },
                _ => return None,
            }
        }
        "time" => {
            let segundos = |t: &str| numero(t).map(|f| ((f - f.floor()) * 86_400.0).round() as i64 % 86_400);
            let a = formatar_hora(segundos(f1.as_deref()?)?);
            let b = f2.as_deref().and_then(segundos).map(formatar_hora);
            let op = operator_numero(operador)?;
            let b = if matches!(op, NumeroOp::Entre | NumeroOp::Fora) { Some(b?) } else { None };
            Regra::Hora { op, a, b }
        }
        "custom" => {
            let alvo = ranges[0].normalized();
            let celula = feathersheet_core::celulas::refs::cell_name(alvo.r0, alvo.c0);
            crate::xlsx_validacao_custom::regra_de_custom(f1.as_deref()?, &celula)?
        }
        _ => return None,
    };
    // Sem mensagem de erro, o Excel deixa passar: aqui vira só um aviso.
    let mostra_erro = matches!(dv.attr("showErrorMessage"), Some("1") | Some("true"));
    let quando = match (mostra_erro, dv.attr("errorStyle").unwrap_or("stop")) {
        (true, "stop") => QuandoValidacao::Recusar,
        (true, "information") => QuandoValidacao::Informar,
        _ => QuandoValidacao::Avisar,
    };
    let texto = |nome: &str| dv.attr(nome).map(str::to_string).filter(|t| !t.trim().is_empty());
    let mensagem = texto("error");
    let mostra_entrada = matches!(dv.attr("showInputMessage"), Some("1") | Some("true"));
    let mensagens = MensagensValidacao {
        entrada_titulo: if mostra_entrada { texto("promptTitle") } else { None },
        entrada_texto: if mostra_entrada { texto("prompt") } else { None },
        erro_titulo: texto("errorTitle"),
    };
    Some(Validacao { ranges, regra, quando, mensagem, mensagens })
}

fn operator_numero(op: &str) -> Option<NumeroOp> {
    Some(match op {
        "between" => NumeroOp::Entre,
        "notBetween" => NumeroOp::Fora,
        "equal" => NumeroOp::Igual,
        "notEqual" => NumeroOp::Diferente,
        "greaterThan" => NumeroOp::Maior,
        "lessThan" => NumeroOp::Menor,
        "greaterThanOrEqual" => NumeroOp::MaiorIgual,
        "lessThanOrEqual" => NumeroOp::MenorIgual,
        _ => return None,
    })
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn classicas_e_x14_viram_regras_do_motor() {
        let xml = r#"<worksheet><sheetData/><dataValidations count="4">
<dataValidation type="list" allowBlank="1" showErrorMessage="1" sqref="B19"><formula1>"SIM,NÃO"</formula1></dataValidation>
<dataValidation type="list" allowBlank="1" showErrorMessage="1" sqref="E28 H28"><formula1>INDIRECT("Inversores")</formula1></dataValidation>
<dataValidation type="list" sqref="C2:C9"><formula1>$A$2:$A$9</formula1></dataValidation>
<dataValidation type="whole" operator="greaterThanOrEqual" showErrorMessage="1" error="Só inteiro" sqref="D2"><formula1>1</formula1></dataValidation>
<dataValidation type="textLength" sqref="F2"><formula1>5</formula1></dataValidation>
</dataValidations><extLst><ext uri="x"><x14:dataValidations><x14:dataValidation type="list" showErrorMessage="1"><x14:formula1><xm:f>'Quadro-resumo '!$B$5:$I$5</xm:f></x14:formula1><xm:sqref>D6:D99</xm:sqref></x14:dataValidation></x14:dataValidations></ext></extLst></worksheet>"#;
        let root = super::super::xml::parse_xml(xml).unwrap();
        let (v, fora) = ler(&root);
        assert_eq!((v.len(), fora), (5, 1), "o tamanho do texto fica de fora");
        assert_eq!(v[0].regra, Regra::Lista { valores: vec!["SIM".into(), "NÃO".into()] });
        assert_eq!(v[0].quando, QuandoValidacao::Recusar);
        assert_eq!(v[1].ranges.len(), 2);
        assert!(matches!(&v[1].regra, Regra::Formula { formula } if formula.contains("Inversores")), "{:?}", v[1].regra);
        assert!(matches!(v[2].regra, Regra::Intervalo { .. }));
        assert_eq!(v[2].quando, QuandoValidacao::Avisar, "sem showErrorMessage é só aviso");
        assert!(matches!(v[3].regra, Regra::Numero { op: NumeroOp::MaiorIgual, inteiro: true, .. }));
        assert_eq!(v[3].mensagem.as_deref(), Some("Só inteiro"));
        assert!(matches!(&v[4].regra, Regra::Formula { formula } if formula.contains("Quadro-resumo")), "{:?}", v[4].regra);
        assert_eq!(v[4].ranges[0], Rect::new(5, 3, 98, 3));
    }

    #[test]
    fn hora_mensagens_e_estilos_do_excel() {
        let xml = r#"<worksheet><sheetData/><dataValidations count="3">
<dataValidation type="time" operator="between" allowBlank="1" showInputMessage="1" showErrorMessage="1" errorStyle="information" errorTitle="Horário" error="Fora do expediente" promptTitle="Expediente" prompt="Das 8h as 18h" sqref="A1:A9"><formula1>0.333333333333333</formula1><formula2>0.75</formula2></dataValidation>
<dataValidation type="time" operator="greaterThan" showErrorMessage="1" errorStyle="warning" promptTitle="Some" prompt="Escondida" sqref="B1"><formula1>0.5</formula1></dataValidation>
<dataValidation type="custom" showErrorMessage="1" sqref="C3:C9"><formula1>ISNUMBER(SEARCH("ab",C3))</formula1></dataValidation>
</dataValidations></worksheet>"#;
        let root = super::super::xml::parse_xml(xml).unwrap();
        let (v, fora) = ler(&root);
        assert_eq!((v.len(), fora), (3, 0));
        assert_eq!(v[0].regra, Regra::Hora { op: NumeroOp::Entre, a: "08:00".into(), b: Some("18:00".into()) });
        assert_eq!(v[0].quando, QuandoValidacao::Informar);
        assert_eq!(v[0].mensagem.as_deref(), Some("Fora do expediente"));
        assert_eq!(v[0].mensagens.erro_titulo.as_deref(), Some("Horário"));
        assert_eq!(v[0].mensagem_de_entrada(), Some(("Expediente".into(), "Das 8h as 18h".into())));
        assert_eq!(v[1].regra, Regra::Hora { op: NumeroOp::Maior, a: "12:00".into(), b: None });
        assert_eq!(v[1].quando, QuandoValidacao::Avisar);
        assert_eq!(v[1].mensagem_de_entrada(), None, "sem showInputMessage a mensagem não aparece");
        assert!(matches!(&v[2].regra, Regra::Texto { valor: Some(t), .. } if t == "ab"));
    }
}
