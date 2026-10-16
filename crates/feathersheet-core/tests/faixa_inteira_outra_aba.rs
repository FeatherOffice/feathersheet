//! Colunas e linhas inteiras de outra aba (`modulos!C:I`, `'2. Eixo'!5:5`),
//! como na v6.0a: `=SEERRO(PROCV(B29;modulos!C:I;2;FALSO);"")` dava #NOME?.
use feathersheet_core::{planilha::workbook::Workbook, Value};

fn pasta() -> Workbook {
    let mut w = Workbook::new();
    w.criar_aba("modulos");
    w.escrever_celula(1, 2, 2, "BYD 450".into());
    w.escrever_celula(1, 2, 3, "450".into());
    w.escrever_celula(1, 3, 2, "Jinko 550".into());
    w.escrever_celula(1, 3, 3, "550".into());
    w
}

#[test]
fn procv_em_colunas_inteiras_de_outra_aba() {
    let mut w = pasta();
    w.escrever_celula(0, 28, 1, "BYD 450".into());
    w.escrever_celula(0, 29, 1, "=SEERRO(PROCV(B29;modulos!C:I;2;FALSO);\"\")".into());
    assert_eq!(w.valor(0, 29, 1), Value::Number(450.));
    assert_eq!(w.aba(0).unwrap().sheet.get_raw(29, 1), "=SEERRO(PROCV(B29;modulos!C:I;2;FALSO);\"\")");
}

#[test]
fn trocar_a_celula_lida_recalcula_a_formula() {
    let mut w = pasta();
    w.escrever_celula(0, 28, 1, "BYD 450".into());
    w.escrever_celula(0, 29, 1, "=PROCV(B29;modulos!C:I;2;FALSO)".into());
    assert_eq!(w.valor(0, 29, 1), Value::Number(450.));
    // A célula escolhida na lista muda: a fórmula acompanha.
    w.escrever_celula(0, 28, 1, "Jinko 550".into());
    assert_eq!(w.valor(0, 29, 1), Value::Number(550.));
    // A tabela da outra aba muda: a fórmula também acompanha.
    w.escrever_celula(1, 3, 3, "560".into());
    assert_eq!(w.valor(0, 29, 1), Value::Number(560.));
}

#[test]
fn linhas_inteiras_e_aba_com_espaco() {
    let mut w = Workbook::new();
    w.criar_aba("2. Eixo");
    w.escrever_celula(1, 4, 2, "7".into());
    for f in ["=SOMA('2. Eixo'!C:C)", "=SOMA('2. Eixo'!$C:$D)", "=SOMA('2. Eixo'!5:5)"] {
        w.escrever_celula(0, 0, 0, f.into());
        assert_eq!(w.valor(0, 0, 0), Value::Number(7.), "{f}");
        assert_eq!(w.aba(0).unwrap().sheet.get_raw(0, 0), f);
    }
}
