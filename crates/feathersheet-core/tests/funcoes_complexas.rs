//! Números complexos (IM*, COMPLEXO) pela planilha inteira (`Workbook`).
//!
//! Os valores esperados seguem a documentação do Excel e os exemplos do pedido.

use feathersheet_core::celulas::values::{ErrorCode, Value};
use feathersheet_core::planilha::workbook::Workbook;

/// Calcula a fórmula em G1 da aba 0 (com A1:A3 = "1+i", vazia, "2").
fn calcular(formula: &str) -> Value {
    let mut w = Workbook::new();
    w.escrever_celula(0, 0, 0, "1+i".to_string());
    w.escrever_celula(0, 2, 0, "2".to_string());
    w.escrever_celula(0, 0, 6, formula.to_string());
    w.valor(0, 0, 6)
}

fn texto(formula: &str) -> String {
    match calcular(formula) {
        Value::Text(t) => t,
        outro => panic!("{formula}: esperava texto, veio {outro:?}"),
    }
}

fn numero(formula: &str) -> f64 {
    match calcular(formula) {
        Value::Number(n) => n,
        outro => panic!("{formula}: esperava número, veio {outro:?}"),
    }
}

fn erro(formula: &str) -> ErrorCode {
    match calcular(formula) {
        Value::Error(e) => e.code,
        outro => panic!("{formula}: esperava erro, veio {outro:?}"),
    }
}

fn perto(obtido: f64, esperado: f64) {
    assert!(
        (obtido - esperado).abs() <= 1e-9 * esperado.abs().max(1.0),
        "obtido {obtido}, esperado {esperado}"
    );
}

#[test]
fn soma_produto_subtracao_e_quociente() {
    assert_eq!(texto("=IMSUM(\"3+4i\";\"1-2i\")"), "4+2i");
    assert_eq!(texto("=IMSOMA(\"3+4i\";\"1-2i\")"), "4+2i");
    assert_eq!(texto("=IMPRODUCT(\"1+2i\";\"3-i\")"), "5+5i");
    assert_eq!(texto("=IMPROD(\"3+4i\";\"5-3i\")"), "27+11i");
    assert_eq!(texto("=IMPRODUCT(\"1+2i\";30)"), "30+60i");
    assert_eq!(texto("=IMDIV(\"-238+240i\";\"10+24i\")"), "5+12i");
    assert_eq!(texto("=IMSUB(\"13+4i\";\"5+3i\")"), "8+i");
    assert_eq!(texto("=IMSUBTR(\"13+4i\";\"5+3i\")"), "8+i");
    assert_eq!(texto("=IMSUM(\"1.5e1+2E-1j\")"), "15+0,2j");
}

#[test]
fn soma_de_intervalo_ignora_vazio() {
    // A1 = "1+i", A2 vazia, A3 = "2": soma 3+i.
    assert_eq!(texto("=IMSUM(A1:A3)"), "3+i");
}

#[test]
fn modulo_argumento_partes() {
    perto(numero("=IMABS(\"3+4i\")"), 5.0);
    perto(numero("=IMREAL(\"3+4i\")"), 3.0);
    perto(numero("=IMAGINÁRIO(\"3+4i\")"), 4.0);
    perto(numero("=IMAGINARY(\"4\")"), 0.0);
    perto(numero("=IMREAL(7)"), 7.0);
    perto(numero("=IMARG(\"3+4i\")"), 0.927_295_218_001_612_2);
    perto(numero("=IMAGINARY(\"-2.5i\")"), -2.5);
    perto(numero("=IMAGINARY(\"-2,5i\")"), -2.5);
    assert_eq!(erro("=IMARGUMENT(\"0\")"), ErrorCode::Div0);
    assert_eq!(texto("=IMCONJ(\"3+4i\")"), "3-4i");
    assert_eq!(texto("=IMCONJUGATE(\"3+4i\")"), "3-4i");
}

#[test]
fn raiz_e_potencias() {
    // Igual ao Excel: a raiz passa pela forma polar e o cos(pi/2) deixa o resíduo 1,2E-16.
    assert_eq!(texto("=IMSQRT(\"-4\")"), "1,22464679914735E-16+2i");
    assert_eq!(texto("=IMRAIZ(\"-4\")"), "1,22464679914735E-16+2i");
    assert_eq!(texto("=IMPOWER(\"2+3i\";3)"), "-46+9,00000000000001i"); // como na documentação do Excel
    assert_eq!(texto("=IMPOT(\"i\";2)"), "-1+1,22464679914735E-16i"); // forma polar, como o Excel
    assert_eq!(texto("=IMPOWER(\"i\";0)"), "1");
    perto(numero("=IMREAL(IMPOWER(\"2+3i\";0,5))"), 1.674_149_228_035_540_2);
    perto(numero("=IMAGINARY(IMPOWER(\"2+3i\";0,5))"), 0.895_977_476_129_838);
}

#[test]
fn exponencial_e_logaritmos() {
    perto(numero("=IMREAL(IMEXP(\"1+i\"))"), 1.468_693_939_915_885_2);
    perto(numero("=IMAGINARY(IMEXP(\"1+i\"))"), 2.287_355_287_178_842);
    perto(numero("=IMREAL(IMLN(\"3+4i\"))"), 1.609_437_912_434_100_3);
    perto(numero("=IMAGINARY(IMLN(\"3+4i\"))"), 0.927_295_218_001_612_2);
    perto(numero("=IMAGINARY(IMLOG10(\"3+4i\"))"), 0.402_719_196_273_373_2);
    perto(numero("=IMAGINARY(IMLOG2(\"3+4i\"))"), 1.337_804_212_450_976_8);
    assert_eq!(erro("=IMLN(\"0\")"), ErrorCode::Num);
    assert_eq!(erro("=IMLOG10(\"0\")"), ErrorCode::Num);
}

#[test]
fn trigonometria_complexa() {
    perto(numero("=IMREAL(IMSIN(\"3+4i\"))"), 3.853_738_037_919_377);
    perto(numero("=IMAGINARY(IMSIN(\"3+4i\"))"), -27.016_813_258_003_93);
    perto(numero("=IMREAL(IMCOS(\"1+i\"))"), 0.833_730_025_131_149);
    perto(numero("=IMAGINARY(IMCOS(\"1+i\"))"), -0.988_897_705_762_865);
    perto(numero("=IMREAL(IMSINH(\"4+3i\"))"), -27.016_813_258_003_93);
    perto(numero("=IMREAL(IMCOSH(\"4+3i\"))"), -27.034_945_603_074_224);
    perto(numero("=IMREAL(IMTAN(\"4+3i\"))"), 0.004_908_258_067_496_06);
    perto(numero("=IMAGINARY(IMTAN(\"4+3i\"))"), 1.000_709_536_067_233);
    perto(numero("=IMREAL(IMCOT(\"4+3i\"))"), 0.004_901_182_394_304_4);
    perto(numero("=IMREAL(IMSEC(\"4+3i\"))"), -0.065_294_027_857_947_1);
    perto(numero("=IMREAL(IMCSC(\"4+3i\"))"), -0.075_489_832_915_863_7);
    assert_eq!(texto("=IMSECH(\"0\")"), "1");
    assert_eq!(erro("=IMCSCH(\"0\")"), ErrorCode::Num);
    assert_eq!(erro("=IMCSC(\"0\")"), ErrorCode::Num);
}

#[test]
fn complexo_montado_pela_funcao() {
    assert_eq!(texto("=COMPLEX(3;4)"), "3+4i");
    assert_eq!(texto("=COMPLEXO(3;4)"), "3+4i");
    assert_eq!(texto("=COMPLEX(3;4;\"j\")"), "3+4j");
    assert_eq!(texto("=COMPLEX(0;1)"), "i");
    assert_eq!(texto("=COMPLEX(0;-1)"), "-i");
    assert_eq!(texto("=COMPLEX(3;-1)"), "3-i");
    assert_eq!(texto("=COMPLEX(0;4)"), "4i");
    assert_eq!(texto("=COMPLEX(0;0)"), "0");
    assert_eq!(erro("=COMPLEX(3;4;\"k\")"), ErrorCode::Value);
}

#[test]
fn texto_invalido_e_erro_de_numero() {
    assert_eq!(erro("=IMABS(\"abc\")"), ErrorCode::Num);
    assert_eq!(erro("=IMABS(\"3+4\")"), ErrorCode::Num);
    assert_eq!(erro("=IMDIV(\"1+i\";\"0\")"), ErrorCode::Num);
}

#[test]
fn sufixos_misturados_sao_valor() {
    assert_eq!(erro("=IMSUM(\"1+i\";\"1+j\")"), ErrorCode::Value);
    assert_eq!(erro("=IMPRODUCT(\"1+2i\";\"3+4j\")"), ErrorCode::Value);
    assert_eq!(erro("=IMSUB(\"1+i\";\"1+j\")"), ErrorCode::Value);
}

#[test]
fn logico_nao_e_complexo() {
    assert_eq!(erro("=IMABS(VERDADEIRO())"), ErrorCode::Value);
}

#[test]
fn sufixo_j_e_mantido_na_saida() {
    assert_eq!(texto("=IMSUM(\"3+4j\";\"1-2j\")"), "4+2j");
    assert_eq!(texto("=IMCONJ(\"3-4j\")"), "3+4j");
}
