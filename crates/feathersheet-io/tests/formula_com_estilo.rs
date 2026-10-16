//! Fórmula com estilo (moeda, %, data) mora na coluna empacotada e o estilo
//! sobrevive a salvar e reabrir o `.fshz`.
use feathersheet_core::celulas::estilo_parcial::{Campo, EstiloParcial};
use feathersheet_core::celulas::refs::Rect;
use feathersheet_core::celulas::values::Value;
use feathersheet_core::planilha::workbook::Workbook;
use feathersheet_io::da_workbook::pasta_gravar_bytes;
use feathersheet_io::fshz_read::ler_fshz;
use feathersheet_io::para_workbook::para_workbook;

#[test]
fn formula_com_estilo_volta_do_arquivo() {
    let mut w = Workbook::new();
    for r in 0..1_000u32 {
        w.escrever_celula(0, r, 0, format!("{}", r + 1));
        w.escrever_celula(0, r, 1, format!("=A{}*2", r + 1));
    }
    let mut moeda = EstiloParcial::new();
    moeda.insert("format".into(), Campo::Texto("currency".into()));
    w.aba_mut(0).unwrap().sheet.set_style(&Rect::new(0, 1, 999, 1), &moeda);
    {
        let s = &w.aba(0).unwrap().sheet;
        assert_eq!(s.formulas.size(), 1_000, "as fórmulas com estilo continuam empacotadas");
        assert!(s.cells.is_empty(), "nenhuma fórmula caiu no caminho por célula");
    }
    assert_eq!(w.valor(0, 999, 1), Value::Number(2_000.0));

    let bytes = pasta_gravar_bytes(&w, 0).expect("gravar");
    let lida = ler_fshz(&bytes).expect("ler");
    let w2 = para_workbook(&lida);
    let s2 = &w2.aba(0).unwrap().sheet;
    assert_eq!(s2.get_raw(999, 1), "=A1000*2");
    assert_eq!(w2.valor(0, 999, 1), Value::Number(2_000.0));
    assert!(s2.estilo_mesclado(999, 1).is_some_and(|e| e.get("format") == Some(&Campo::Texto("currency".into()))),
        "o estilo da fórmula voltou do arquivo");
    assert_eq!(s2.formulas.size(), 1_000, "e ela continua empacotada depois de reabrir");
}
