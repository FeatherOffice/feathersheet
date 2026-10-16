//! A matriz dinâmica volta derramada depois de salvar e reabrir o `.fshz`:
//! o arquivo guarda só a fórmula da âncora (e o primeiro valor), e a área
//! derramada é montada de novo ao abrir.
use feathersheet_core::celulas::values::Value;
use feathersheet_core::planilha::workbook::Workbook;
use feathersheet_io::da_workbook::pasta_gravar_bytes;
use feathersheet_io::fshz_read::ler_fshz;
use feathersheet_io::para_workbook::para_workbook;

#[test]
fn matriz_volta_derramada_do_arquivo() {
    let mut w = Workbook::new();
    w.escrever_celula(0, 0, 0, "=SEQUÊNCIA(3)".into());
    w.escrever_celula(0, 0, 2, "=SOMA(A1:A3)".into());
    assert_eq!(w.valor(0, 2, 0), Value::Number(3.0));

    let bytes = pasta_gravar_bytes(&w, 0).expect("gravar");
    let w2 = para_workbook(&ler_fshz(&bytes).expect("ler"));
    let s2 = &w2.aba(0).unwrap().sheet;
    assert_eq!(s2.get_raw(0, 0), "=SEQUÊNCIA(3)");
    assert_eq!(s2.get_raw(1, 0), "", "a vizinha não vira conteúdo próprio no arquivo");
    assert_eq!(w2.valor(0, 1, 0), Value::Number(2.0));
    assert_eq!(w2.valor(0, 2, 0), Value::Number(3.0));
    assert_eq!(w2.valor(0, 0, 2), Value::Number(6.0));
}
