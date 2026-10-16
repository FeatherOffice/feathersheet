//! `SOMASES`/`CONT.SES` lendo a faixa de uma vez: o resultado é o mesmo de
//! antes, e mudar a aba de origem (valor, fórmula, desfazer) nunca deixa um
//! valor velho guardado.
use feathersheet_core::celulas::values::Value;
use feathersheet_core::planilha::workbook::Workbook;

const N: u32 = 5_000;

/// Aba 0 "Resumo" com as contas; aba 1 "Dados": A = região, B = valor,
/// C = fórmula (B*2).
fn pasta() -> (Workbook, usize) {
    let mut w = Workbook::new();
    let dados = w.criar_aba("Dados");
    for r in 0..N {
        let regiao = if r % 3 == 0 { "Sul" } else if r % 3 == 1 { "Norte" } else { "Leste" };
        w.escrever_celula(dados, r, 0, regiao.into());
        w.escrever_celula(dados, r, 1, format!("{}", r + 1));
        w.escrever_celula(dados, r, 2, format!("=B{}*2", r + 1));
    }
    let fim = N;
    w.escrever_celula(0, 0, 0, format!("=SOMASES(Dados!B1:B{fim};Dados!A1:A{fim};\"sul\")"));
    w.escrever_celula(0, 1, 0, format!("=CONT.SES(Dados!A1:A{fim};\"Norte\")"));
    w.escrever_celula(0, 2, 0, format!("=SOMASE(Dados!A1:A{fim};\"Leste\";Dados!C1:C{fim})"));
    w.escrever_celula(0, 3, 0, format!("=MÉDIASES(Dados!B1:B{fim};Dados!A1:A{fim};\"Sul\")"));
    (w, dados)
}

fn soma_sul(b: impl Fn(u32) -> f64) -> f64 {
    (0..N).filter(|r| r % 3 == 0).map(b).sum()
}

fn num(v: Value) -> f64 {
    match v {
        Value::Number(n) => n,
        outro => panic!("esperava número, veio {outro:?}"),
    }
}

#[test]
fn resultados_iguais_aos_de_celula_por_celula() {
    let (w, _) = pasta();
    assert_eq!(num(w.valor(0, 0, 0)), soma_sul(|r| f64::from(r + 1)));
    assert_eq!(num(w.valor(0, 1, 0)), f64::from((0..N).filter(|r| r % 3 == 1).count() as u32));
    let leste: f64 = (0..N).filter(|r| r % 3 == 2).map(|r| f64::from(r + 1) * 2.0).sum();
    assert_eq!(num(w.valor(0, 2, 0)), leste);
    let qtd = (0..N).filter(|r| r % 3 == 0).count() as f64;
    assert_eq!(num(w.valor(0, 3, 0)), soma_sul(|r| f64::from(r + 1)) / qtd);
}

#[test]
fn mudar_a_origem_recalcula_sem_valor_guardado_velho() {
    let (mut w, dados) = pasta();
    let antes = num(w.valor(0, 0, 0));
    // Valor de uma linha do Sul.
    w.escrever_celula(dados, 0, 1, "1001".into());
    assert_eq!(num(w.valor(0, 0, 0)), antes + 1000.0);
    // A região muda: a linha sai do Sul.
    w.escrever_celula(dados, 0, 0, "Oeste".into());
    assert_eq!(num(w.valor(0, 0, 0)), antes - 1.0);
    // Fórmula da origem (coluna C) depende de B: SOMASE sobre C acompanha.
    let leste = num(w.valor(0, 2, 0));
    w.escrever_celula(dados, 2, 1, "103".into()); // B3 era 3; C3 = B3*2
    assert_eq!(num(w.valor(0, 2, 0)), leste + 200.0);
    // Desfazer (como o Ctrl+Z da janela) volta ao valor anterior.
    let mut desfez = false;
    w.editar(dados, |s| desfez = s.undo());
    assert!(desfez);
    assert_eq!(num(w.valor(0, 2, 0)), leste);
}
