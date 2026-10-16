use super::*;

pub(super) fn grade<'a>(linhas: &'a [&'a [Value]]) -> impl Fn(u32, u32) -> Value + 'a {
    move |r, c| linhas.get(r as usize).and_then(|l| l.get(c as usize)).cloned().unwrap_or(Value::Empty)
}

#[test]
pub(super) fn texto_em_cima_e_numero_embaixo_e_cabecalho() {
    let t = |s: &str| Value::Text(s.to_string());
    let dados: [&[Value]; 2] = [&[t("Nome"), t("Valor")], &[t("a"), Value::Number(1.0)]];
    assert!(parece_cabecalho(Rect::new(0, 0, 1, 1), grade(&dados)));
    // Tudo texto: não dá para dizer.
    let so_texto: [&[Value]; 2] = [&[t("Nome"), t("Valor")], &[t("a"), t("b")]];
    assert!(!parece_cabecalho(Rect::new(0, 0, 1, 1), grade(&so_texto)));
    // Número em cima: não é cabeçalho.
    let numero: [&[Value]; 2] = [&[Value::Number(1.0), t("Valor")], &[Value::Number(2.0), Value::Number(3.0)]];
    assert!(!parece_cabecalho(Rect::new(0, 0, 1, 1), grade(&numero)));
    // Uma linha só.
    assert!(!parece_cabecalho(Rect::new(0, 0, 0, 1), grade(&dados)));
}
