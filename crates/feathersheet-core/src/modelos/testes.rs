use super::*;
use crate::celulas::values::Value;

fn sem_erros(w: &Workbook) -> Result<usize, String> {
    let mut formulas = 0;
    for indice in 0..w.len() {
        let aba = w.aba(indice).expect("aba existe");
        let ur = aba.sheet.used_rect();
        for r in 0..=ur.r1 {
            for c in 0..=ur.c1 {
                if aba.sheet.get_raw(r, c).starts_with('=') {
                    formulas += 1;
                }
                if let Value::Error(e) = w.valor(indice, r, c) {
                    return Err(format!("aba {indice} célula {r},{c}: {e:?}"));
                }
            }
        }
    }
    Ok(formulas)
}

#[test]
fn lista_comeca_pelo_em_branco_e_tem_onze_modelos() {
    let itens = lista();
    assert_eq!(itens.len(), 11);
    assert_eq!(itens[0].id, "em-branco");
    let mut ids: Vec<&str> = itens.iter().map(|m| m.id).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 11, "ids repetidos");
}

#[test]
fn cada_modelo_cria_tem_aba_e_calcula_sem_erro() {
    for modelo in lista() {
        let w = criar(modelo.id).unwrap_or_else(|| panic!("{} não criou", modelo.id));
        assert!(w.len() >= 1, "{} sem aba", modelo.id);
        let formulas = sem_erros(&w).unwrap_or_else(|e| panic!("{}: {e}", modelo.id));
        if modelo.id != "em-branco" {
            assert!(formulas > 0, "{} sem nenhuma fórmula", modelo.id);
        }
    }
}

#[test]
fn id_inexistente_devolve_none() {
    assert!(criar("inexistente").is_none());
}

#[test]
fn datas_digitadas_viram_numero() {
    let w = criar("lista-tarefas").unwrap();
    assert!(matches!(w.valor(0, R_DADOS, 2), Value::Number(_)), "o prazo deveria ser data");
    let w = criar("vendas").unwrap();
    assert!(matches!(w.valor(0, R_DADOS, 0), Value::Number(_)), "a data da venda deveria ser data");
}

#[test]
fn calendario_usa_o_epoca_certo() {
    // 1 de outubro de 2026 é quinta-feira: quatro dias vazios antes (dom=0).
    let mut w = Workbook::new();
    w.escrever_celula(0, 0, 0, "=MOD(DATA(2026;10;1)-1;7)".into());
    assert_eq!(w.valor(0, 0, 0), Value::Number(4.0));
}

#[test]
fn regras_e_mesclas_dos_modelos_sao_validas() {
    use crate::operacoes::rules::ConditionalRule;
    for modelo in lista() {
        let w = criar(modelo.id).unwrap();
        let s = &w.aba(0).unwrap().sheet;
        if let Some(r) = &s.r {
            let regras: Vec<ConditionalRule> = serde_json::from_str(r)
                .unwrap_or_else(|e| panic!("{}: regra inválida: {e}\n{r}", modelo.id));
            assert!(!regras.is_empty());
        }
        if let Some(m) = &s.mc {
            // O formato que a grade, o PDF e o arquivo leem: objetos `Rect`.
            let mesclas: Vec<Rect> = serde_json::from_str(m)
                .unwrap_or_else(|e| panic!("{}: mescla inválida: {e}", modelo.id));
            assert!(mesclas.iter().all(|m| m.r0 <= m.r1 && m.c0 <= m.c1));
        }
        if modelo.id != "em-branco" {
            assert!(s.col_widths.values().all(|&w| w >= 20), "{}: largura em pixels", modelo.id);
        }
    }
}

#[test]
fn indicadores_calculam_o_valor_esperado() {
    let w = criar("controle-gastos").unwrap();
    // Total gasto = soma dos 13 lançamentos.
    assert_eq!(w.valor(0, R_INDICADORES + 1, 0), Value::Number(2177.0));
    let w = criar("vendas").unwrap();
    assert!(matches!(w.valor(0, R_INDICADORES + 1, 8), Value::Text(_)), "melhor vendedor é um nome");
    let w = criar("calendario-mensal").unwrap();
    assert!(matches!(w.valor(0, 0, 0), Value::Text(ref t) if t.contains(" de ")), "título com mês e ano");
}

#[test]
fn queda_de_tensao_confere_com_o_calculo_a_mao() {
    // Cobre a 20 °C (ρ = 1/56), limite de queda 4%, desequilíbrio 10%.
    // (sistema, fases, V, P, cosφ, L, S) dos dez circuitos de exemplo.
    let ex: [(&str, &str, f64, f64, f64, f64, f64); 10] = [
        ("M", "R", 127.0, 2400.0, 0.92, 25.0, 4.0), ("M", "S", 127.0, 2400.0, 0.92, 28.0, 4.0),
        ("M", "T", 127.0, 3000.0, 0.95, 30.0, 6.0), ("B", "RS", 220.0, 5400.0, 1.0, 25.0, 6.0),
        ("B", "ST", 220.0, 2760.0, 0.82, 38.0, 2.5), ("T", "RST", 220.0, 18000.0, 0.88, 42.0, 16.0),
        ("T", "RST", 220.0, 11000.0, 0.85, 65.0, 16.0), ("T", "RST", 220.0, 3600.0, 0.8, 90.0, 2.5),
        ("M", "S", 127.0, 1500.0, 0.75, 150.0, 6.0), ("B", "TR", 220.0, 6000.0, 0.95, 30.0, 6.0),
    ];
    let w = criar("queda-tensao").unwrap();
    assert_eq!(w.len(), 5);
    let perto = |w: &Workbook, aba: usize, r: u32, c: u32, esperado: f64| match w.valor(aba, r, c) {
        Value::Number(n) => assert!((n - esperado).abs() < 1e-6 * esperado.abs().max(1.0), "aba {aba} ({r},{c}) = {n}, esperado {esperado}"),
        outro => panic!("aba {aba} ({r},{c}): {outro:?}"),
    };
    let secoes = [1.5, 2.5, 4.0, 6.0, 10.0, 16.0, 25.0, 35.0, 50.0, 70.0, 95.0, 120.0, 150.0, 185.0, 240.0, 300.0];
    let rho = 1.0 / 56.0;
    let (mut amp, mut watt, mut reprovados, mut maior) = ([0.0f64; 3], [0.0f64; 3], 0.0, 0.0f64);
    for (i, (sis, fases, v, p, fp, l, s)) in ex.iter().enumerate() {
        let r = R_DADOS + i as u32;
        let (k, n) = if *sis == "T" { (3f64.sqrt(), 3f64.sqrt()) } else { (2.0, 1.0) };
        let corrente = p / (n * v * fp);
        let dv = k * rho * l * corrente * fp / s;
        let pct = dv / v;
        let minima_calc = k * rho * l * corrente * fp / (0.04 * v);
        let minima = secoes.iter().copied().find(|x| *x >= minima_calc).unwrap();
        perto(&w, 1, r, 7, corrente);
        perto(&w, 1, r, 10, k);
        perto(&w, 1, r, 11, dv);
        perto(&w, 1, r, 12, pct);
        perto(&w, 1, r, 14, minima);
        let esperado = if pct <= 0.04 { "OK" } else { "Acima do limite" };
        assert_eq!(w.valor(1, r, 13), Value::Text(esperado.into()), "circuito {}", i + 1);
        if pct > 0.04 { reprovados += 1.0; }
        maior = maior.max(pct);
        for (j, f) in ["R", "S", "T"].iter().enumerate() {
            if fases.contains(f) || *fases == "RST" {
                amp[j] += corrente;
                watt[j] += p / fases.len() as f64;
            }
        }
    }
    assert_eq!(w.valor(1, R_DADOS + 1, 13), Value::Text("OK".into()));
    assert_eq!(w.valor(1, R_DADOS + 7, 13), Value::Text("Acima do limite".into()));
    assert_eq!(w.valor(1, R_DADOS + 8, 13), Value::Text("Acima do limite".into()));
    perto(&w, 1, R_INDICADORES + 1, 0, 10.0);
    perto(&w, 1, R_INDICADORES + 1, 2, maior);
    perto(&w, 1, R_INDICADORES + 1, 5, reprovados);
    // Linhas vazias da tabela ficam em branco.
    assert_eq!(w.valor(1, R_DADOS + 10, 7), Value::Text(String::new()));
    // Balanceamento: corrente conta inteira em cada fase ligada; potência é dividida.
    for j in 0..3 {
        perto(&w, 2, R_DADOS + j as u32, 1, amp[j]);
        perto(&w, 2, R_DADOS + j as u32, 2, watt[j]);
    }
    let media = amp.iter().sum::<f64>() / 3.0;
    let maxa = amp.iter().cloned().fold(0.0, f64::max);
    perto(&w, 2, 16, 1, (maxa - media) / media);
    assert_eq!(w.valor(2, 17, 1), Value::Text("OK".into()));
    // Fases incompatíveis com o sistema pedem revisão.
    let mut w = w;
    w.escrever_celula(1, R_DADOS, 3, "RS".into());
    assert_eq!(w.valor(1, R_DADOS, 13), Value::Text("Revisar fases".into()));
}
