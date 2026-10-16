//! A tabela (o Ctrl+T do Excel): o que o `core/table.ts` e o
//! `modelo/tabela.ts` ainda calculavam no TypeScript.
//!
//! - [`calcular_total`] / [`total_da_coluna`] — a linha de totais, **pulando
//!   linha escondida** (o `calcularTotal`). É por isso que ela não é uma
//!   fórmula na célula: `=SOMA(F2:F100)` somaria o que o filtro escondeu.
//! - [`crescer_tabela`] — a tabela cresce quando se escreve logo abaixo ou
//!   logo à direita dela (o `crescerTabela`).
//! - [`nome_disponivel`] — `Tabela1`, `Tabela2`… (o `nomeDisponivel`).
//!
//! O resto da tabela já estava no Rust antes deste módulo: a região em volta
//! da célula é o [`Sheet::regiao_da_tabela`] (o `regiaoAtual` do `table.ts`),
//! o "parece cabeçalho" é o `ops::parece_cabecalho`, e o filtro (o
//! `makePredicate` e as listas de caixinhas) é o `operacoes/filtro.rs`.
//!
//! Os metadados da tabela (área, filtros, estilo, totais) continuam morando
//! na `Aba` do TypeScript e chegam aqui como argumento do comando — o que
//! este módulo recebe é só o que a conta precisa, sem JSON (o `sheet-core`
//! não tem dependência). O estilo (`EstiloDeTabela`, as paletas) é desenho e
//! fica no TypeScript.

use crate::celulas::refs::Rect;
use crate::celulas::values::Value;
use crate::planilha::sheet::Sheet;

/// A conta da linha de totais sobre os valores de uma coluna (já sem as
/// linhas escondidas). Tradução do `calcularTotal` do `core/table.ts`, linha
/// a linha. `None` quando não há o que contar: soma de nada é nada, não zero.
///
/// Como lá, e diferente do rodapé (`ops::selection_stats_com`): a
/// **contagem conta célula com erro** (é "não vazia"), e o texto vazio (`""`,
/// o de uma fórmula que devolve vazio) **não** conta. Até 27/09/2026 a ponte
/// usava a conta do rodapé para a linha de totais e divergia nesses dois
/// pontos; o teste `test/ponte/tabela-rust.test.ts` trava a tradução.
pub fn calcular_total(valores: impl IntoIterator<Item = Value>, funcao: &str) -> Option<f64> {
    if funcao == "nenhum" {
        return None;
    }
    let mut n = 0u64;
    let mut n_numeros = 0u64;
    let mut soma = 0.0;
    let mut soma_quadrados = 0.0;
    let mut maior = f64::NEG_INFINITY;
    let mut menor = f64::INFINITY;
    for v in valores {
        // `v === null || v === undefined || v === ''`.
        if matches!(&v, Value::Empty) || matches!(&v, Value::Text(t) if t.is_empty()) {
            continue;
        }
        n += 1;
        if let Value::Number(x) = v {
            if x.is_finite() {
                n_numeros += 1;
                soma += x;
                soma_quadrados += x * x;
                if x > maior {
                    maior = x;
                }
                if x < menor {
                    menor = x;
                }
            }
        }
    }
    let k = n_numeros as f64;
    match funcao {
        "contagem" => Some(n as f64),
        "contagem-numeros" => Some(k),
        "soma" if n_numeros > 0 => Some(soma),
        "media" if n_numeros > 0 => Some(soma / k),
        "maximo" if n_numeros > 0 => Some(maior),
        "minimo" if n_numeros > 0 => Some(menor),
        // Desvio da amostra (n-1), o mesmo do rodapé e do gráfico.
        "variancia" if n_numeros >= 2 => {
            let media = soma / k;
            Some(((soma_quadrados - k * media * media) / (k - 1.0)).max(0.0))
        }
        "desvio" if n_numeros >= 2 => {
            let media = soma / k;
            let variancia = (soma_quadrados - k * media * media) / (k - 1.0);
            Some(if variancia > 0.0 { variancia.sqrt() } else { 0.0 })
        }
        _ => None,
    }
}

/// A linha de totais de uma coluna da tabela: `area` é a coluna (a primeira
/// coluna do retângulo), sem o cabeçalho. Lê só até a última linha usada —
/// abaixo dela tudo é vazio, e vazio não entra na conta.
///
/// `visivel` diz se a linha está à vista (altura 0 é o que o filtro esconde;
/// na ponte as alturas chegam como `FaixasOcultas`), e `valor` lê pela pasta,
/// para a fórmula que lê outra aba funcionar.
pub fn total_da_coluna(
    sheet: &Sheet,
    area: Rect,
    funcao: &str,
    mut visivel: impl FnMut(u32) -> bool,
    valor: impl Fn(u32, u32) -> Value,
) -> Option<f64> {
    let area = area.normalized();
    let col = area.c0;
    let usado = sheet.used_rect();
    let vazia = sheet.is_empty() || col < usado.c0 || col > usado.c1;
    let (r0, r1) = (area.r0.max(usado.r0), area.r1.min(usado.r1));
    if vazia || r0 > r1 {
        return calcular_total(std::iter::empty(), funcao);
    }
    calcular_total((r0..=r1).filter(move |&r| visivel(r)).map(|r| valor(r, col)), funcao)
}

/// A área de uma tabela, com o que decide se ela pode crescer.
#[derive(Debug, Clone, PartialEq)]
pub struct AreaDaTabela {
    pub area: Rect,
    /// A linha logo abaixo é a de totais: ali não se digita.
    pub mostrar_totais: bool,
    /// Só AutoFiltro (Ctrl+Shift+L): não tem corpo próprio para crescer.
    pub so_filtro: bool,
}

/// Faz a tabela crescer quando se escreve logo abaixo (ou logo à direita)
/// dela — o `crescerTabela` do `modelo/tabela.ts`. Devolve o índice da
/// primeira tabela que cresce e a área nova, ou `None`.
///
/// Não cresce para cima nem para a esquerda de propósito: ali a linha nova
/// seria o cabeçalho. Na linha de totais também não.
pub fn crescer_tabela(tabelas: &[AreaDaTabela], row: u32, col: u32) -> Option<(usize, Rect)> {
    for (i, t) in tabelas.iter().enumerate() {
        if t.so_filtro {
            continue;
        }
        let a = &t.area;
        let abaixo = a.r1.checked_add(1) == Some(row);
        if t.mostrar_totais && abaixo {
            continue;
        }
        if abaixo && col >= a.c0 && col <= a.c1 {
            return Some((i, Rect::new(a.r0, a.c0, row, a.c1)));
        }
        if a.c1.checked_add(1) == Some(col) && row >= a.r0 && row <= a.r1 {
            return Some((i, Rect::new(a.r0, a.c0, a.r1, col)));
        }
    }
    None
}

/// O primeiro nome livre da série `Tabela1`, `Tabela2`… — sem diferença de
/// maiúscula (o `nomeDisponivel` compara em `toLowerCase`).
pub fn nome_disponivel(usados: &[String], prefixo: &str) -> String {
    let ja_tem: std::collections::HashSet<String> = usados.iter().map(|n| n.to_lowercase()).collect();
    (1u64..)
        .map(|i| format!("{prefixo}{i}"))
        .find(|nome| !ja_tem.contains(&nome.to_lowercase()))
        .expect("a série não acaba")
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::celulas::values::{CellError, ErrorCode};

    fn coluna(valores: &[&str]) -> Sheet {
        let mut s = Sheet::new("P");
        for (r, v) in valores.iter().enumerate() {
            s.set_cell(r as u32, 0, v);
        }
        s
    }

    fn total(s: &Sheet, r0: u32, r1: u32, escondida: Option<u32>, f: &str) -> Option<f64> {
        total_da_coluna(s, Rect::new(r0, 0, r1, 0), f, |r| Some(r) != escondida, |r, c| s.get_value(r, c))
    }

    // Os testes do `calcularTotal` (`test/io/tabela-estilo.test.ts`), portados.
    #[test]
    fn cada_conta_da_linha_de_totais() {
        let s = coluna(&["Valor", "2", "4", "texto", "6", "8"]);
        let t = |f: &str| total(&s, 1, 5, None, f);
        assert_eq!(t("soma"), Some(20.0));
        assert_eq!(t("media"), Some(5.0));
        assert_eq!(t("maximo"), Some(8.0));
        assert_eq!(t("minimo"), Some(2.0));
        assert_eq!(t("contagem"), Some(5.0), "a contagem conta o texto também");
        assert_eq!(t("contagem-numeros"), Some(4.0), "a de números, não");
        assert!((t("desvio").unwrap() - 2.581988).abs() < 1e-5);
        assert_eq!(t("nenhum"), None);
        assert_eq!(t("qualquer"), None);
    }

    #[test]
    fn o_total_ignora_a_linha_escondida() {
        let s = coluna(&["Valor", "10", "20", "30"]);
        assert_eq!(total(&s, 1, 3, None, "soma"), Some(60.0));
        assert_eq!(total(&s, 1, 3, Some(2), "soma"), Some(40.0));
        assert_eq!(total(&s, 1, 3, Some(2), "contagem"), Some(2.0));
    }

    #[test]
    fn coluna_sem_numero_nao_inventa_zero() {
        let s = coluna(&["Nome", "Ana"]);
        assert_eq!(total(&s, 1, 1, None, "soma"), None, "soma de nada é nada, não zero");
        assert_eq!(total(&s, 1, 1, None, "contagem"), Some(1.0));
        assert_eq!(total(&s, 1, 1, None, "desvio"), None);
        assert_eq!(total(&s, 50, 90, None, "contagem"), Some(0.0), "abaixo da área usada");
    }

    /// Onde o `calcularTotal` difere do rodapé, e a tradução segue ele: o
    /// erro conta como "não vazio", o texto vazio não.
    #[test]
    fn erro_conta_e_texto_vazio_nao() {
        let erro = Value::Error(CellError::new(ErrorCode::Div0));
        let valores = vec![Value::Number(3.0), erro, Value::Text(String::new()), Value::Bool(true), Value::Empty];
        assert_eq!(calcular_total(valores.clone(), "contagem"), Some(3.0));
        assert_eq!(calcular_total(valores, "soma"), Some(3.0));
        let mut s = coluna(&["=1/0", "=\"\"", "4"]);
        assert_eq!(total(&s, 0, 2, None, "contagem"), Some(2.0));
        s.set_cell(3, 0, "VERDADEIRO");
        assert_eq!(total(&s, 0, 3, None, "contagem"), Some(3.0));
        assert_eq!(total(&s, 0, 3, None, "contagem-numeros"), Some(1.0));
    }

    fn t(r0: u32, c0: u32, r1: u32, c1: u32) -> AreaDaTabela {
        AreaDaTabela { area: Rect::new(r0, c0, r1, c1), mostrar_totais: false, so_filtro: false }
    }

    // Os casos do `crescerTabela` (`test/io/tabela-estilo.test.ts`).
    #[test]
    fn cresce_para_baixo_e_para_a_direita() {
        let lista = [t(0, 0, 3, 2)];
        assert_eq!(crescer_tabela(&lista, 4, 1), Some((0, Rect::new(0, 0, 4, 2))));
        assert_eq!(crescer_tabela(&lista, 2, 3), Some((0, Rect::new(0, 0, 3, 3))));
        assert_eq!(crescer_tabela(&lista, 5, 1), None, "duas linhas abaixo não");
        assert_eq!(crescer_tabela(&lista, 4, 3), None, "a diagonal não");
        assert_eq!(crescer_tabela(&lista, 4, 5), None);
    }

    #[test]
    fn nao_cresce_para_cima_nem_na_linha_de_totais_nem_autofiltro() {
        let mut com_totais = t(2, 2, 5, 4);
        com_totais.mostrar_totais = true;
        assert_eq!(crescer_tabela(std::slice::from_ref(&com_totais), 6, 3), None, "linha de totais");
        assert_eq!(crescer_tabela(std::slice::from_ref(&com_totais), 3, 5), Some((0, Rect::new(2, 2, 5, 5))));
        assert_eq!(crescer_tabela(&[t(2, 2, 5, 4)], 1, 3), None, "para cima");
        assert_eq!(crescer_tabela(&[t(2, 2, 5, 4)], 3, 1), None, "para a esquerda");
        let mut filtro = t(0, 0, 3, 2);
        filtro.so_filtro = true;
        assert_eq!(crescer_tabela(&[filtro, t(0, 0, 3, 2)], 4, 0), Some((1, Rect::new(0, 0, 4, 2))), "a primeira que cresce");
    }

    #[test]
    fn nome_pula_os_usados_sem_olhar_maiuscula() {
        let usados = ["tabela1".to_string(), "Tabela3".to_string()];
        assert_eq!(nome_disponivel(&usados, "Tabela"), "Tabela2");
        assert_eq!(nome_disponivel(&[], "Tabela"), "Tabela1");
        assert_eq!(nome_disponivel(&["TABELA1".into(), "Tabela2".into()], "Tabela"), "Tabela3");
    }
}
