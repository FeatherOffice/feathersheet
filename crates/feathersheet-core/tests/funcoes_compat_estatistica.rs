//! Nomes antigos de testes estatísticos (CHITEST, TTEST) e PERCENTOF, pela
//! planilha inteira (`Workbook`), como o usuário as digita.
//!
//! CHITEST e TTEST são só apelidos: o resultado tem de ser idêntico ao da
//! função nova, célula por célula. PERCENTOF tem o valor conferido à mão.

use feathersheet_core::celulas::values::{ErrorCode, Value};
use feathersheet_core::planilha::workbook::Workbook;

/// Código do erro, se o resultado for erro.
fn codigo(v: Value) -> Option<ErrorCode> {
    match v {
        Value::Error(e) => Some(e.code),
        _ => None,
    }
}

/// Aba 0: A1:A4 = 10, 20, 30, 40; B1:B4 = 12, 18, 33, 37; C1:C2 = 5, 15;
/// D1:D2 = 1, 3; E1:E2 = 0, 0.
fn pasta() -> Workbook {
    let mut w = Workbook::new();
    for (i, v) in [10, 20, 30, 40].iter().enumerate() {
        w.escrever_celula(0, i as u32, 0, v.to_string());
    }
    for (i, v) in [12, 18, 33, 37].iter().enumerate() {
        w.escrever_celula(0, i as u32, 1, v.to_string());
    }
    for (i, v) in [5, 15].iter().enumerate() {
        w.escrever_celula(0, i as u32, 2, v.to_string());
    }
    for (i, v) in [1, 3].iter().enumerate() {
        w.escrever_celula(0, i as u32, 3, v.to_string());
    }
    for i in 0..2 {
        w.escrever_celula(0, i, 4, "0".to_string());
    }
    w
}

/// Escreve a fórmula em G1 (sobre a pasta de teste) e devolve o valor.
fn calcular(formula: &str) -> Value {
    let mut w = pasta();
    w.escrever_celula(0, 0, 6, formula.to_string());
    w.valor(0, 0, 6)
}

fn numero(v: Value) -> f64 {
    match v {
        Value::Number(n) => n,
        outro => panic!("esperava número, veio {outro:?}"),
    }
}

#[test]
fn chitest_e_o_mesmo_que_chisq_test() {
    let antigo = calcular("=CHITEST(A1:A4;B1:B4)");
    let novo = calcular("=CHISQ.TEST(A1:A4;B1:B4)");
    assert_eq!(antigo, novo);
    let p = numero(antigo);
    assert!(p > 0.0 && p < 1.0, "p fora de (0; 1): {p}");
}

#[test]
fn ttest_e_o_mesmo_que_t_test_nos_quatro_tipos() {
    // caudas 1 ou 2; tipo 1 (pareado), 2 (variância igual), 3 (variância diferente).
    for caudas in [1, 2] {
        for tipo in [1, 2, 3] {
            let antigo = calcular(&format!("=TTEST(A1:A4;B1:B4;{caudas};{tipo})"));
            let novo = calcular(&format!("=T.TEST(A1:A4;B1:B4;{caudas};{tipo})"));
            assert_eq!(antigo, novo, "caudas {caudas}, tipo {tipo}");
        }
    }
}

#[test]
fn ttest_com_argumentos_invalidos_da_o_mesmo_erro() {
    // tipo 1 pede pares de mesmo tamanho: A1:A4 com C1:C2 falha igual nos dois nomes.
    assert_eq!(codigo(calcular("=TTEST(A1:A4;C1:C2;2;1)")), Some(ErrorCode::Num));
    assert_eq!(codigo(calcular("=T.TEST(A1:A4;C1:C2;2;1)")), Some(ErrorCode::Num));
}

#[test]
fn percentof_soma_do_subconjunto_sobre_a_soma_do_total() {
    // Subconjunto 12+18 = 30; total 12+18+33+37 = 100; 30/100 = 0,3.
    let v = numero(calcular("=PERCENTOF(B1:B2;B1:B4)"));
    assert!((v - 0.3).abs() < 1e-12, "obtido {v}");
    // Números soltos também valem: (1+3) / (10+20+30+40) = 0,04.
    let v = numero(calcular("=PERCENTOF(D1:D2;A1:A4)"));
    assert!((v - 0.04).abs() < 1e-12, "obtido {v}");
    // Subconjunto igual ao total dá 1.
    let v = numero(calcular("=PERCENTOF(A1:A4;A1:A4)"));
    assert!((v - 1.0).abs() < 1e-12, "obtido {v}");
}

#[test]
fn percentof_total_zero_e_div0() {
    assert_eq!(codigo(calcular("=PERCENTOF(A1:A4;E1:E2)")), Some(ErrorCode::Div0));
}

#[test]
fn percentof_com_nome_em_portugues_nao_existe_ainda() {
    // Decisão: só o nome em inglês entra; PERCENTOF em pt-BR não foi confirmado.
    assert_eq!(codigo(calcular("=PORCENTAGEMDE(A1:A4;A1:A4)")), Some(ErrorCode::Name));
}
