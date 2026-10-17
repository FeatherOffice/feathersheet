//! Os valores das pastas externas que o `.fshz` de destino guarda — o campo
//! `ex` dos metadados do livro (contrato em `FORMATO.md` da suíte).
//!
//! Uma fórmula `=[Vendas.fshz]Dados!A1` precisa de um valor mesmo com
//! `Vendas.fshz` fechado: vale o último que chegou, guardado aqui junto com
//! de onde veio e quando. Só vão as células das áreas que as fórmulas leem
//! (`Workbook::externos_para_gravar`), não a outra pasta inteira.
//!
//! Forma (MessagePack no FSH3, mapas nomeados como o resto dos metadados):
//!
//! ```text
//! ex: [ { n: "Vendas.fshz",              // o arquivo como a fórmula escreve
//!         p: "/home/x/Vendas.fshz",      // último caminho absoluto (opcional)
//!         t: 1760630000,                 // quando os valores chegaram, s desde 1970 UTC (opcional)
//!         a: [ { n: "Dados", c: [[linha, coluna, valor], …] } ] } ]
//! ```
//!
//! `valor`: número, texto, verdadeiro/falso ou `{e: "#REF!"}` para erro.

use feathersheet_core::celulas::values::{CellError, ErrorCode, Value};
use feathersheet_core::planilha::workbook::vinculos::LivroGuardado;
use serde_json::{json, Value as Json};

/// Um valor de célula na forma do arquivo. Número que o JSON não representa
/// (infinito) vira `#NUM!`; vazio vira `null` (não costuma ir: só células
/// não vazias são guardadas).
pub fn valor_para_json(v: &Value) -> Json {
    match v {
        Value::Empty => Json::Null,
        Value::Number(n) => serde_json::Number::from_f64(*n)
            .map(Json::Number)
            .unwrap_or_else(|| json!({ "e": ErrorCode::Num.as_str() })),
        Value::Text(t) => Json::String(t.clone()),
        Value::Bool(b) => Json::Bool(*b),
        Value::Error(e) => json!({ "e": e.code.as_str() }),
    }
}

/// O inverso de [`valor_para_json`]. Código de erro desconhecido vira `#ERROR!`.
pub fn valor_de_json(j: &Json) -> Value {
    match j {
        Json::Null => Value::Empty,
        Json::Bool(b) => Value::Bool(*b),
        Json::Number(n) => n.as_f64().map_or(Value::Empty, Value::Number),
        Json::String(s) => Value::Text(s.clone()),
        Json::Object(o) => Value::Error(CellError::new(
            o.get("e").and_then(Json::as_str).and_then(ErrorCode::from_code).unwrap_or(ErrorCode::Error),
        )),
        Json::Array(_) => Value::Empty,
    }
}

/// As células `[[linha, coluna, valor], …]` na forma do arquivo.
pub fn celulas_para_json(celulas: &[(u32, u32, Value)]) -> Json {
    Json::Array(celulas.iter().map(|(r, c, v)| json!([r, c, valor_para_json(v)])).collect())
}

/// O inverso de [`celulas_para_json`]; entradas malformadas são puladas.
pub fn celulas_de_json(j: &Json) -> Vec<(u32, u32, Value)> {
    j.as_array()
        .into_iter()
        .flatten()
        .filter_map(|c| {
            let c = c.as_array()?;
            let r = u32::try_from(c.first()?.as_u64()?).ok()?;
            let col = u32::try_from(c.get(1)?.as_u64()?).ok()?;
            Some((r, col, valor_de_json(c.get(2).unwrap_or(&Json::Null))))
        })
        .collect()
}

/// O campo `ex` inteiro. `None` quando não há pasta externa a guardar (o
/// campo nem vai para o arquivo).
pub fn para_json(livros: &[LivroGuardado]) -> Option<Json> {
    if livros.is_empty() {
        return None;
    }
    Some(Json::Array(
        livros
            .iter()
            .map(|l| {
                let mut o = serde_json::Map::new();
                o.insert("n".into(), Json::String(l.nome.clone()));
                if let Some(p) = &l.caminho {
                    o.insert("p".into(), Json::String(p.clone()));
                }
                if let Some(t) = l.atualizado_em {
                    o.insert("t".into(), json!(t));
                }
                o.insert(
                    "a".into(),
                    Json::Array(l.abas.iter().map(|(n, c)| json!({ "n": n, "c": celulas_para_json(c) })).collect()),
                );
                Json::Object(o)
            })
            .collect(),
    ))
}

/// O inverso de [`para_json`]; livro sem nome é pulado.
pub fn de_json(j: &Json) -> Vec<LivroGuardado> {
    j.as_array()
        .into_iter()
        .flatten()
        .filter_map(|l| {
            let nome = l.get("n")?.as_str()?.to_string();
            if nome.is_empty() {
                return None;
            }
            let abas = l
                .get("a")
                .and_then(Json::as_array)
                .into_iter()
                .flatten()
                .filter_map(|a| Some((a.get("n")?.as_str()?.to_string(), celulas_de_json(a.get("c")?))))
                .collect();
            Some(LivroGuardado {
                nome,
                caminho: l.get("p").and_then(Json::as_str).map(str::to_string),
                atualizado_em: l.get("t").and_then(Json::as_i64),
                abas,
            })
        })
        .collect()
}

#[cfg(test)]
mod testes {
    use super::*;
    use feathersheet_core::planilha::vinculo::aba_de_valores;
    use feathersheet_core::planilha::workbook::Workbook;

    #[test]
    fn valores_vao_e_voltam() {
        for v in [
            Value::Number(1.5),
            Value::Text("Olá".into()),
            Value::Bool(true),
            Value::Error(CellError::new(ErrorCode::Div0)),
        ] {
            assert_eq!(valor_de_json(&valor_para_json(&v)), v);
        }
        assert_eq!(valor_para_json(&Value::Number(f64::INFINITY)), json!({ "e": "#NUM!" }));
        assert_eq!(valor_de_json(&json!({ "e": "#INVENTADO" })), Value::Error(CellError::new(ErrorCode::Error)));
    }

    /// De ponta a ponta: a pasta de destino grava os valores de fora no
    /// `.fshz` e, reaberta sem a origem, as fórmulas mostram o último valor.
    #[test]
    fn valores_externos_vao_e_voltam_no_fshz() {
        let mut wb = Workbook::new();
        wb.escrever_celula(0, 0, 0, "=[Vendas.fshz]Dados!A1*2".into());
        wb.escrever_celula(0, 1, 0, "=SOMA('/home/x/[Vendas.fshz]Dados'!B1:B2)".into());
        let dados = aba_de_valores("Dados", &[
            (0, 0, Value::Number(21.0)),
            (0, 1, Value::Text("x".into())),
            (5, 5, Value::Number(1.0)),
        ]);
        wb.instalar_externo_com("Vendas.fshz", vec![dados], Some("/home/x/Vendas.fshz".into()), Some(1_760_630_000));
        let outro = aba_de_valores("Dados", &[(0, 1, Value::Number(4.0)), (1, 1, Value::Number(6.0))]);
        wb.instalar_externo_com("/home/x/Vendas.fshz", vec![outro], None, Some(7));
        assert_eq!(wb.valor(0, 0, 0), Value::Number(42.0));
        assert_eq!(wb.valor(0, 1, 0), Value::Number(10.0));

        let bytes = crate::da_workbook::pasta_gravar_bytes(&wb, 0).expect("grava");
        let lida = crate::fshz_read::ler_fshz(&bytes).expect("lê");
        assert!(lida.ex.is_some());
        let reaberta = crate::para_workbook::para_workbook(&lida);
        assert_eq!(reaberta.valor(0, 0, 0), Value::Number(42.0));
        assert_eq!(reaberta.valor(0, 1, 0), Value::Number(10.0));
        let livro = reaberta.livro_externo("vendas.fshz").expect("guardado");
        assert_eq!(livro.caminho.as_deref(), Some("/home/x/Vendas.fshz"));
        assert_eq!(livro.atualizado_em, Some(1_760_630_000));
        // Só as células citadas foram para o arquivo (F6 ninguém lê).
        assert_eq!(livro.aba("Dados").map(|a| a.get_value(5, 5)), Some(Value::Empty));
        assert_eq!(reaberta.livro_externo("/home/x/Vendas.fshz").and_then(|l| l.atualizado_em), Some(7));
    }

    #[test]
    fn pasta_sem_vinculo_nao_grava_o_campo() {
        let wb = Workbook::new();
        let bytes = crate::da_workbook::pasta_gravar_bytes(&wb, 0).expect("grava");
        assert!(crate::fshz_read::ler_fshz(&bytes).expect("lê").ex.is_none());
    }
}
