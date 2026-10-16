//! As respostas das tarefas assíncronas (`IMPORTARDADOS`, função de macro…)
//! em JSON: no arquivo (campo `pd` da aba no FSH2) e na ponte
//! (`tarefa_resolver` com `valor`).
//!
//! No TypeScript isto era o `pending` do `SheetData` (`AsyncValues.toJSON`),
//! que só ia no `toJSON` — o `.fshz` colunar o perdia, e reabrir o arquivo
//! mostrava `#CALCULANDO!` até a busca voltar. A forma é a mesma do
//! `AsyncCacheData`: `{ chave: { v } | { m }, at }`. O `at` aqui é a ordem de
//! chegada (não um relógio): é só para o teto de 500 descartar a mais antiga.
//!
//! Um valor é número, texto, lógico, `null` (vazio) ou `{ "erro": "#N/D",
//! "detalhe": "…" }` — o objeto do `valor_de_celula_json` da ponte, com o
//! motivo junto.

use serde_json::{json, Map, Value as Json};
use feathersheet_core::celulas::values::{CellError, ErrorCode, Value};
use feathersheet_core::planilha::assincrono::Resposta;
use feathersheet_core::planilha::sheet::Sheet;

pub fn valor_para_json(v: &Value) -> Json {
    match v {
        Value::Empty => Json::Null,
        Value::Number(n) if n.is_finite() => json!(n),
        Value::Number(_) => json!({ "erro": ErrorCode::Num.as_str() }),
        Value::Text(t) => json!(t),
        Value::Bool(b) => json!(b),
        Value::Error(e) => {
            let mut o = json!({ "erro": e.code.as_str() });
            if let Some(d) = &e.detail {
                o["detalhe"] = json!(d);
            }
            o
        }
    }
}

pub fn valor_de_json(j: &Json) -> Value {
    match j {
        Json::Null => Value::Empty,
        Json::Bool(b) => Value::Bool(*b),
        Json::Number(n) => n.as_f64().map(Value::Number).unwrap_or(Value::Empty),
        Json::String(t) => Value::Text(t.clone()),
        Json::Object(o) => {
            let code = o.get("erro").and_then(Json::as_str).and_then(ErrorCode::from_code).unwrap_or(ErrorCode::Value);
            match o.get("detalhe").and_then(Json::as_str) {
                Some(d) => Value::Error(CellError::com_motivo(code, d)),
                None => Value::Error(CellError::new(code)),
            }
        }
        // Uma lista solta no lugar de um valor: o texto dela, como o
        // `JSON.stringify` do `fn-web.ts` fazia com o aninhado demais.
        outro => Value::Text(outro.to_string()),
    }
}

/// A resposta como vai no arquivo: `{ v }` ou `{ m }`, e a ordem de chegada.
pub fn resposta_para_json(r: &Resposta, at: u64) -> Json {
    match r {
        Resposta::Valor(v) => json!({ "v": valor_para_json(v), "at": at }),
        Resposta::Matriz(linhas) => json!({
            "m": linhas.iter().map(|l| l.iter().map(valor_para_json).collect::<Vec<_>>()).collect::<Vec<_>>(),
            "at": at,
        }),
    }
}

/// O contrário: `{ m: [[…]] }` é tabela; `{ v }` (ou sem `v`, como o
/// `entry.v ?? null` do TypeScript) é um valor.
pub fn resposta_de_json(j: &Json) -> Option<Resposta> {
    let o = j.as_object()?;
    if let Some(m) = o.get("m").and_then(Json::as_array) {
        return Some(Resposta::Matriz(
            m.iter().map(|l| l.as_array().map(|l| l.iter().map(valor_de_json).collect()).unwrap_or_default()).collect(),
        ));
    }
    Some(Resposta::Valor(o.get("v").map(valor_de_json).unwrap_or(Value::Empty)))
}

/// As respostas guardadas da aba, para o arquivo — `None` quando não há
/// nenhuma (o campo nem aparece, e o arquivo fica igual ao de antes).
pub fn pendentes_da_aba(s: &Sheet) -> Option<Json> {
    let a = s.assincronos.borrow();
    let guardadas = a.respondidas();
    if guardadas.is_empty() {
        return None;
    }
    let mut o = Map::new();
    for (k, r, at) in guardadas {
        o.insert(k.clone(), resposta_para_json(r, at));
    }
    Some(Json::Object(o))
}

/// Carrega o `pd` do arquivo na aba, na ordem de chegada (`at`). Campo
/// ausente ou estranho: nenhuma resposta, e a tarefa pergunta de novo.
pub fn carregar_na_aba(s: &Sheet, pd: Option<&Json>) {
    let mut lidas: Vec<(u64, String, Resposta)> = pd
        .and_then(Json::as_object)
        .into_iter()
        .flatten()
        .filter_map(|(k, j)| Some((j.get("at").and_then(Json::as_f64).unwrap_or(0.0).max(0.0) as u64, k.clone(), resposta_de_json(j)?)))
        .collect();
    lidas.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    s.assincronos.borrow_mut().carregar(lidas.into_iter().map(|(_, k, r)| (k, r)));
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn ida_e_volta_com_erro_e_tabela() {
        let erro = Value::Error(CellError::com_motivo(ErrorCode::Na, "sem linha"));
        assert_eq!(valor_de_json(&valor_para_json(&erro)), erro);
        let t = Resposta::Matriz(vec![vec![Value::Text("a".into()), Value::Number(1.5)], vec![Value::Bool(true), Value::Empty]]);
        assert_eq!(resposta_de_json(&resposta_para_json(&t, 3)), Some(t));
        // O `pending` do TypeScript (`{ v, at }` com `at` em milissegundos).
        assert_eq!(resposta_de_json(&json!({ "v": 99, "at": 1_700_000_000_000u64 })), Some(Resposta::Valor(Value::Number(99.0))));
    }

    #[test]
    fn a_aba_grava_e_le_na_ordem_de_chegada() {
        let s = Sheet::new("A");
        s.assincronos.borrow_mut().resolver("b", Resposta::Valor(Value::Number(2.0)));
        s.assincronos.borrow_mut().resolver("a", Resposta::Valor(Value::Number(1.0)));
        s.assincronos.borrow_mut().resolver("ruim", Resposta::falha("caiu"));
        let pd = pendentes_da_aba(&s).unwrap();
        assert!(pd.get("ruim").is_none(), "erro não vai ao arquivo");
        let outra = Sheet::new("B");
        carregar_na_aba(&outra, Some(&pd));
        let ordem: Vec<String> = outra.assincronos.borrow().respondidas().into_iter().map(|(k, _, _)| k.clone()).collect();
        assert_eq!(ordem, vec!["b", "a"]);
        assert!(pendentes_da_aba(&Sheet::new("C")).is_none());
    }
}
