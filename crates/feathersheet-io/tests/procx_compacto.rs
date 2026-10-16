//! Round-trip do PROCX compacto, incluindo valores invalidados após edição.
use feathersheet_core::{planilha::workbook::Workbook, Value};
use feathersheet_io::{
    da_workbook::pasta_gravar_bytes, fshz_read::ler_fshz, para_workbook::para_workbook,
};

#[test]
fn procx_compacto_volta_calculado_e_recalcula_apos_edicao() {
    let mut w = Workbook::new();
    let produtos = w.criar_aba("Produtos");
    w.escrever_celula(produtos, 0, 0, "A".into());
    w.escrever_celula(produtos, 0, 1, "10".into());
    for r in 0..100 {
        w.escrever_celula(0, r, 0, "A".into());
        w.escrever_celula(
            0,
            r,
            1,
            format!("=PROCX(A{};Produtos!$A$1:$A$1;Produtos!$B$1:$B$1)", r + 1),
        );
    }
    assert_eq!(w.valor(0, 99, 1), Value::Number(10.0));
    w.consolidar_calculos(usize::MAX);
    let bytes = pasta_gravar_bytes(&w, 0).unwrap();
    let mut reaberta = para_workbook(&ler_fshz(&bytes).unwrap());
    assert_eq!(reaberta.aba(0).unwrap().sheet.formulas.size(), 100);
    assert!(reaberta.aba(0).unwrap().sheet.cells.is_empty());
    assert_eq!(reaberta.valor(0, 99, 1), Value::Number(10.0));
    reaberta.escrever_celula(produtos, 0, 1, "20".into());
    // Gravar antes de recalcular não pode ressuscitar o resultado antigo.
    let bytes = pasta_gravar_bytes(&reaberta, 0).unwrap();
    let mut outra = para_workbook(&ler_fshz(&bytes).unwrap());
    assert_eq!(outra.valor(0, 99, 1), Value::Number(20.0));
    outra.consolidar_calculos(usize::MAX);
    assert_eq!(outra.valor(0, 99, 1), Value::Number(20.0));
    assert_eq!(outra.valor(0, 50, 1), Value::Number(20.0));
}
