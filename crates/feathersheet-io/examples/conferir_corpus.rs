//! Confere o motor Rust contra o motor TypeScript, fórmula a fórmula.
//!
//! O `scripts/gerar-corpus.ts` roda ~700 fórmulas no motor TypeScript (a
//! REFERÊNCIA, pela regra 12 do CLAUDE.md) e grava as respostas em
//! `test/corpus/formulas.json`. Este exemplo roda as mesmas fórmulas no
//! `sheet-core`, na mesma planilha e na mesma célula (A1), e lista onde os
//! dois discordam.
//!
//! Mora no `sheet-io` só porque precisa ler JSON: o `sheet-core` não tem
//! dependência nenhuma, de propósito (regra 11).
//!
//! ```bash
//! npx tsx scripts/gerar-corpus.ts          # regenera a referência
//! cargo run --release --example conferir_corpus -- ../../test/corpus/formulas.json
//! ```

use serde_json::{json, Value as Json};
use feathersheet_core::formula::evaluator::{EvalContext, Evaluator};
use feathersheet_core::formula::parser::parse_formula;
use feathersheet_core::celulas::refs::{col_index, Rect};
use feathersheet_core::planilha::sheet::Sheet;
use feathersheet_core::celulas::values::{CellError, Computed, ErrorCode, Value};

/// A planilha vista pelo avaliador, só pela API pública da `Sheet`.
struct Ctx<'a> {
    sheet: &'a Sheet,
}

impl EvalContext for Ctx<'_> {
    fn get_value(&self, row: u32, col: u32, sheet: Option<&str>) -> Value {
        if sheet.is_some_and(|s| !s.eq_ignore_ascii_case(&self.sheet.name)) {
            return Value::Error(CellError::new(ErrorCode::Ref));
        }
        self.sheet.get_value(row, col)
    }

    fn for_each_in_range(&self, rect: &Rect, visit: &mut dyn FnMut(&Value, u32, u32)) {
        let usado = self.sheet.used_rect();
        for r in rect.r0..=rect.r1.min(usado.r1) {
            for c in rect.c0..=rect.c1.min(usado.c1) {
                let v = self.sheet.get_value(r, c);
                if !matches!(v, Value::Empty) {
                    visit(&v, r, c);
                }
            }
        }
    }

    fn range_size(&self, rect: &Rect) -> u64 {
        (rect.r1 - rect.r0 + 1) as u64 * (rect.c1 - rect.c0 + 1) as u64
    }

    fn resolve_name(&self, _name: &str) -> Option<Rect> {
        None
    }
}

fn valor_json(v: &Value) -> Json {
    match v {
        Value::Empty => json!({ "tipo": "vazio" }),
        Value::Number(n) => json!({ "tipo": "numero", "n": n }),
        Value::Text(s) => json!({ "tipo": "texto", "s": s }),
        Value::Bool(b) => json!({ "tipo": "logico", "b": b }),
        Value::Error(e) => json!({ "tipo": "erro", "codigo": e.code.as_str() }),
    }
}

fn computado_json(c: &Computed) -> Json {
    match c {
        Computed::Scalar(v) => valor_json(v),
        Computed::Matrix(m) => json!({
            "tipo": "matriz",
            "linhas": m.rows().iter().map(|l| l.iter().map(valor_json).collect::<Vec<_>>()).collect::<Vec<_>>(),
        }),
        Computed::Lambda(_) => json!({ "tipo": "erro", "codigo": "#VALUE!" }),
    }
}

/// Igual, com folga de arredondamento para número (1e-9 relativo).
fn iguais(a: &Json, b: &Json) -> bool {
    match (a, b) {
        (Json::Number(x), Json::Number(y)) => {
            let (x, y) = (x.as_f64().unwrap(), y.as_f64().unwrap());
            x == y || (x - y).abs() <= 1e-9 * x.abs().max(y.abs()).max(1.0)
        }
        (Json::Object(x), Json::Object(y)) => x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| iguais(v, w))),
        (Json::Array(x), Json::Array(y)) => x.len() == y.len() && x.iter().zip(y).all(|(v, w)| iguais(v, w)),
        _ => a == b,
    }
}

fn main() {
    let caminho = std::env::args().nth(1).unwrap_or_else(|| "../../test/corpus/formulas.json".into());
    let corpus: Json = serde_json::from_str(&std::fs::read_to_string(&caminho).expect("não li o corpus")).unwrap();

    // `HOJE()`/`AGORA()` seguem o fuso de quem gerou o corpus.
    if let Some(fuso) = corpus["fuso_minutos"].as_i64() {
        feathersheet_core::funcoes::fn_date::definir_fuso_minutos(fuso as i32);
    }

    let mut sheet = Sheet::new("Planilha1");
    for (endereco, bruto) in corpus["planilha"].as_object().unwrap() {
        let letras: String = endereco.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
        let linha: u32 = endereco[letras.len()..].parse().unwrap();
        sheet.set_cell_raw(linha - 1, col_index(&letras).unwrap(), bruto.as_str().unwrap().into(), None);
    }
    let ctx = Ctx { sheet: &sheet };

    let casos = corpus["casos"].as_array().unwrap();
    let mut certos = 0;
    let mut aleatorias = 0;
    let mut erradas: Vec<(String, Json, Json)> = Vec::new();
    for caso in casos {
        let formula = caso["formula"].as_str().unwrap();
        let esperado = if caso["compila"].as_bool().unwrap() { caso["valor"].clone() } else { json!("não compila") };
        let obtido = match parse_formula(formula) {
            Err(_) => json!("não compila"),
            Ok(no) => {
                let resultado = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    Evaluator::na_celula(&ctx, 0, 0).eval_array(&no)
                }));
                match resultado {
                    Ok(c) => computado_json(&c),
                    Err(_) => json!("PÂNICO"),
                }
            }
        };
        let nome = formula.split('(').next().unwrap_or("").to_uppercase();
        if ["RAND", "ALEATÓRIO", "ALEATORIO", "RANDBETWEEN", "ALEATÓRIOENTRE"].contains(&nome.as_str()) {
            // Sorteio: cada motor tira o seu número. Não há o que comparar.
            aleatorias += 1;
            continue;
        }
        // `AGORA()` anda entre gerar o corpus e rodar isto: um minuto de folga.
        let relogio = ["NOW", "AGORA"].contains(&nome.as_str())
            && (esperado["n"].as_f64().unwrap_or(0.0) - obtido["n"].as_f64().unwrap_or(f64::MAX)).abs() < 1.0 / 1440.0;
        if relogio || iguais(&esperado, &obtido) {
            certos += 1;
        } else {
            erradas.push((formula.to_string(), esperado, obtido));
        }
    }

    for (f, e, o) in &erradas {
        println!("✗ ={f}\n    TypeScript: {e}\n    Rust:       {o}");
    }
    println!(
        "\n{certos} de {} iguais ao TypeScript · {} diferentes · {aleatorias} sorteios não comparados",
        casos.len() - aleatorias,
        erradas.len()
    );
}
