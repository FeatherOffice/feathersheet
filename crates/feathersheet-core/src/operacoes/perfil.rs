//! Perfil da coluna (item 15 da fila, 24/09/2026; "planilha analítica" do
//! `docs/ROADMAP.md`, item 3): a qualidade da coluna inteira num clique —
//! vazias, distintas, repetidas, histograma dos números e os pontos fora da
//! curva. Uma ida só pela ponte: o TypeScript recebe o resumo pronto e desenha.

use std::collections::HashMap;

use crate::celulas::refs::Rect;
use crate::celulas::values::{to_text, Value};

/// Quantas faixas o histograma tem, no máximo.
pub const FAIXAS: usize = 12;
/// Quantos valores frequentes e quantos outliers voltam de exemplo.
pub const EXEMPLOS: usize = 5;

#[derive(Debug, Clone, PartialEq)]
pub struct Faixa {
    pub de: f64,
    pub ate: f64,
    pub qtd: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Quartis {
    pub q1: f64,
    pub mediana: f64,
    pub q3: f64,
    /// Abaixo disto é outlier: Q1 − 1,5 × (Q3 − Q1).
    pub limite_baixo: f64,
    /// Acima disto é outlier: Q3 + 1,5 × (Q3 − Q1).
    pub limite_alto: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PerfilColuna {
    /// Linhas olhadas (até a última usada da aba).
    pub linhas: usize,
    pub vazias: usize,
    pub numeros: usize,
    pub textos: usize,
    /// Verdadeiro/falso.
    pub logicos: usize,
    pub erros: usize,
    pub distintos: usize,
    /// Células cujo valor já tinha aparecido antes na coluna.
    pub repetidos: usize,
    pub min: f64,
    pub max: f64,
    pub media: f64,
    /// `None` com menos de quatro números: quartil de três pontos não diz nada.
    pub quartis: Option<Quartis>,
    pub histograma: Vec<Faixa>,
    pub outliers: usize,
    /// (linha, valor) dos primeiros outliers, na ordem da coluna.
    pub exemplos_outliers: Vec<(u32, f64)>,
    /// Os valores que mais se repetem, do mais frequente ao menos.
    pub frequentes: Vec<(String, usize)>,
}

/// Quantil por interpolação linear (o `QUARTIL.INC` do Excel), com `v` ordenado.
fn quantil(v: &[f64], p: f64) -> f64 {
    let pos = p * (v.len() - 1) as f64;
    let i = pos.floor() as usize;
    let resto = pos - i as f64;
    match v.get(i + 1) {
        Some(prox) => v[i] + resto * (prox - v[i]),
        None => v[i],
    }
}

/// O perfil da primeira coluna de `area`, parando na última linha usada
/// (`max_row`): selecionar a coluna inteira não anda um milhão de vazias.
pub fn perfil_coluna(area: Rect, max_row: u32, valor: impl Fn(u32, u32) -> Value) -> PerfilColuna {
    let rect = area.normalized();
    let col = rect.c0;
    let max_r = rect.r1.min(max_row);

    let (mut vazias, mut textos, mut logicos, mut erros) = (0usize, 0usize, 0usize, 0usize);
    // (linha, número): a linha é para dar o endereço dos outliers.
    let mut numeros: Vec<(u32, f64)> = Vec::new();
    // Número por bits, para não alocar texto a cada célula numérica.
    let mut cont_num: HashMap<u64, usize> = HashMap::new();
    let mut cont_txt: HashMap<String, usize> = HashMap::new();
    let mut linhas = 0usize;

    if rect.r0 <= max_r {
        for r in rect.r0..=max_r {
            linhas += 1;
            let v = valor(r, col);
            match &v {
                Value::Empty => vazias += 1,
                Value::Text(t) if t.is_empty() => vazias += 1,
                Value::Number(n) => {
                    // -0 e 0 são o mesmo valor para quem lê a planilha.
                    let n = if *n == 0.0 { 0.0 } else { *n };
                    numeros.push((r, n));
                    *cont_num.entry(n.to_bits()).or_insert(0) += 1;
                }
                _ => {
                    match &v {
                        Value::Text(_) => textos += 1,
                        Value::Bool(_) => logicos += 1,
                        _ => erros += 1,
                    }
                    *cont_txt.entry(to_text(&v)).or_insert(0) += 1;
                }
            }
        }
    }

    let preenchidas = linhas - vazias;
    let distintos = cont_num.len() + cont_txt.len();

    let mut frequentes: Vec<(String, usize)> = cont_num
        .iter()
        .filter(|(_, &q)| q > 1)
        .map(|(&bits, &q)| (to_text(&Value::Number(f64::from_bits(bits))), q))
        .chain(cont_txt.iter().filter(|(_, &q)| q > 1).map(|(t, &q)| (t.clone(), q)))
        .collect();
    // Empate pelo texto, para a resposta não depender da ordem do HashMap.
    frequentes.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    frequentes.truncate(EXEMPLOS);

    let mut ordenados: Vec<f64> = numeros.iter().map(|&(_, n)| n).collect();
    ordenados.sort_by(f64::total_cmp);
    let (min, max) = match (ordenados.first(), ordenados.last()) {
        (Some(&a), Some(&b)) => (a, b),
        _ => (0.0, 0.0),
    };
    let media = if ordenados.is_empty() { 0.0 } else { ordenados.iter().sum::<f64>() / ordenados.len() as f64 };

    let quartis = (ordenados.len() >= 4).then(|| {
        let q1 = quantil(&ordenados, 0.25);
        let q3 = quantil(&ordenados, 0.75);
        let iqr = q3 - q1;
        Quartis { q1, mediana: quantil(&ordenados, 0.5), q3, limite_baixo: q1 - 1.5 * iqr, limite_alto: q3 + 1.5 * iqr }
    });

    let (mut outliers, mut exemplos_outliers) = (0usize, Vec::new());
    if let Some(q) = &quartis {
        for &(r, n) in &numeros {
            if n < q.limite_baixo || n > q.limite_alto {
                outliers += 1;
                if exemplos_outliers.len() < EXEMPLOS {
                    exemplos_outliers.push((r, n));
                }
            }
        }
    }

    PerfilColuna {
        linhas,
        vazias,
        numeros: numeros.len(),
        textos,
        logicos,
        erros,
        distintos,
        repetidos: preenchidas - distintos,
        min,
        max,
        media,
        quartis,
        histograma: histograma(&ordenados),
        outliers,
        exemplos_outliers,
        frequentes,
    }
}

/// Faixas de mesma largura entre o mínimo e o máximo. Com poucos valores
/// diferentes (notas de 1 a 5, por exemplo), uma faixa por valor.
fn histograma(ordenados: &[f64]) -> Vec<Faixa> {
    let (Some(&min), Some(&max)) = (ordenados.first(), ordenados.last()) else {
        return Vec::new();
    };
    if min == max {
        return vec![Faixa { de: min, ate: max, qtd: ordenados.len() }];
    }
    let mut unicos: Vec<f64> = ordenados.to_vec();
    unicos.dedup();
    if unicos.len() <= FAIXAS && unicos.iter().all(|n| n.fract() == 0.0) {
        return unicos
            .iter()
            .map(|&u| Faixa { de: u, ate: u, qtd: ordenados.partition_point(|&n| n <= u) - ordenados.partition_point(|&n| n < u) })
            .collect();
    }
    let largura = (max - min) / FAIXAS as f64;
    let mut faixas: Vec<Faixa> = (0..FAIXAS)
        .map(|i| Faixa { de: min + largura * i as f64, ate: min + largura * (i + 1) as f64, qtd: 0 })
        .collect();
    faixas[FAIXAS - 1].ate = max;
    for &n in ordenados {
        let i = (((n - min) / largura) as usize).min(FAIXAS - 1);
        faixas[i].qtd += 1;
    }
    faixas
}

#[cfg(test)]
mod testes {
    use super::*;

    fn perfil(valores: &[Value]) -> PerfilColuna {
        let area = Rect::new(0, 0, 1_048_575, 0);
        perfil_coluna(area, valores.len() as u32 - 1, |r, _| valores[r as usize].clone())
    }

    fn n(x: f64) -> Value {
        Value::Number(x)
    }

    #[test]
    fn conta_vazias_tipos_distintos_e_repetidos() {
        let p = perfil(&[
            Value::Text("nome".into()),
            n(1.0),
            n(1.0),
            Value::Empty,
            Value::Text(String::new()),
            Value::Text("a".into()),
            Value::Text("a".into()),
            Value::Bool(true),
        ]);
        assert_eq!(p.linhas, 8);
        assert_eq!(p.vazias, 2);
        assert_eq!((p.numeros, p.textos, p.logicos, p.erros), (2, 3, 1, 0));
        // nome, 1, a, VERDADEIRO
        assert_eq!(p.distintos, 4);
        assert_eq!(p.repetidos, 2);
        assert_eq!(p.frequentes, vec![("1".to_string(), 2), ("a".to_string(), 2)]);
    }

    #[test]
    fn quartis_e_outlier_pelo_intervalo_interquartil() {
        // 1..=9 e um 100: Q1 = 3,25, Q3 = 7,75, limite alto = 14,5.
        let mut v: Vec<Value> = (1..=9).map(|i| n(i as f64)).collect();
        v.push(n(100.0));
        let p = perfil(&v);
        let q = p.quartis.expect("dez números têm quartis");
        assert_eq!(q.q1, 3.25);
        assert_eq!(q.mediana, 5.5);
        assert_eq!(q.q3, 7.75);
        assert_eq!(q.limite_alto, 14.5);
        assert_eq!(p.outliers, 1);
        assert_eq!(p.exemplos_outliers, vec![(9, 100.0)]);
        assert_eq!((p.min, p.max), (1.0, 100.0));
    }

    #[test]
    fn poucos_numeros_nao_tem_quartil_nem_outlier() {
        let p = perfil(&[n(1.0), n(2.0), n(1000.0)]);
        assert!(p.quartis.is_none());
        assert_eq!(p.outliers, 0);
    }

    #[test]
    fn histograma_de_inteiros_poucos_tem_uma_faixa_por_valor() {
        let p = perfil(&[n(1.0), n(3.0), n(3.0), n(5.0)]);
        let qtd: Vec<(f64, usize)> = p.histograma.iter().map(|f| (f.de, f.qtd)).collect();
        assert_eq!(qtd, vec![(1.0, 1), (3.0, 2), (5.0, 1)]);
    }

    #[test]
    fn histograma_continuo_soma_todos_os_numeros() {
        let v: Vec<Value> = (0..1000).map(|i| n(i as f64 * 0.37)).collect();
        let p = perfil(&v);
        assert_eq!(p.histograma.len(), FAIXAS);
        assert_eq!(p.histograma.iter().map(|f| f.qtd).sum::<usize>(), 1000);
        assert_eq!(p.histograma.last().unwrap().ate, p.max);
    }

    #[test]
    fn para_na_ultima_linha_usada() {
        let area = Rect::new(0, 0, 1_048_575, 0);
        let p = perfil_coluna(area, 2, |_, _| n(1.0));
        assert_eq!(p.linhas, 3);
    }
}
