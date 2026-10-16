//! Executa de verdade o JavaScript convertido no motor embutido das macros, para pegar
//! métodos que o motor não tem (ex.: `String.prototype.substr` derrubava `vba.mid`).

use feathersheet_core::{celulas::values::Value, planilha::workbook::Workbook};
use feathersheet_vba::converter_texto;

fn roda(vba: &str, nome: &str) -> Workbook {
    let c = converter_texto(&[("Módulo1", vba)], &[]);
    let js = c.macros.iter().find(|m| m.nome == nome).expect("macro convertida").codigo.clone();
    let mut wb = Workbook::new();
    let r = feathersheet_macros::executar(&mut wb, 0, &js);
    assert!(r.erro.is_none(), "{:?}\n{js}", r.erro);
    wb
}

#[test]
fn mid_do_vba_funciona_no_motor_embutido() {
    let wb = roda(
        "Sub T()\n  Range(\"A1\").Value = Mid(\"abcdef\", 2, 3)\n  Range(\"A2\").Value = Mid(\"abc\", 5)\n  Range(\"A3\").Value = Mid(\"abc\", 2)\n  Range(\"A4\").Value = Mid(\"abc\", 2, 0)\nEnd Sub\n",
        "T",
    );
    assert_eq!(wb.valor(0, 0, 0), Value::Text("bcd".into()));
    assert_eq!(wb.valor(0, 1, 0), Value::Empty);
    assert_eq!(wb.valor(0, 2, 0), Value::Text("bc".into()));
    assert_eq!(wb.valor(0, 3, 0), Value::Empty);
}

#[test]
fn outras_funcoes_de_texto_e_format_rodam_no_motor() {
    let wb = roda(
        "Sub T()\n  Range(\"A1\").Value = Format(5, \"000\") & \"x\"\n  Range(\"A2\").Value = Format(1234.5, \"#,##0.00\")\n  Range(\"A3\").Value = Left(\"abcdef\", 2) & Right(\"abcdef\", 2)\n  Range(\"A4\").Value = Replace(Trim(\"  a-b \"), \"-\", \"+\")\n  Range(\"A5\").Value = InStr(\"abcdef\", \"cd\")\n  Range(\"A6\").Value = String(3, \"x\")\nEnd Sub\n",
        "T",
    );
    assert_eq!(wb.valor(0, 0, 0), Value::Text("005x".into()));
    assert_eq!(wb.valor(0, 2, 0), Value::Text("abef".into()));
    assert_eq!(wb.valor(0, 3, 0), Value::Text("a+b".into()));
    assert_eq!(wb.valor(0, 4, 0), Value::Number(3.0));
    assert_eq!(wb.valor(0, 5, 0), Value::Text("xxx".into()));
}
