//! O que não é célula e anda com a estrutura: ao inserir ou excluir linha ou
//! coluna, os retângulos das tabelas, dinâmicas, proteções, mesclas e regras,
//! e as larguras e alturas personalizadas, vão para o lugar novo.
//!
//! Tradução do `reflowSizes`/`reflowRules`/`reflowTables`/`reflowProtecoes`
//! do `ops.ts` e do `ajustarRect` do `table.ts`. As contas são por eixo, em
//! `i64`: excluir antes do começo daria número negativo em `u32`.

/// Um intervalo `inicio..=fim` de um eixo depois de inserir (`quantas > 0`)
/// ou excluir (`quantas < 0`) em `at`. `None` quando some inteiro.
///
/// É o `ajustarRect`: inserir antes empurra, inserir dentro estica, excluir
/// dentro encolhe, e excluir o intervalo todo apaga.
pub fn ajustar_intervalo(inicio: i64, fim: i64, at: i64, quantas: i64) -> Option<(i64, i64)> {
    ajustar_intervalo_com_borda(inicio, fim, at, quantas, false)
}

/// Como `ajustar_intervalo`, mas para tabelas e dinâmicas: inserir logo na
/// linha de baixo (ou coluna da direita) estica o intervalo para incluir a nova linha.
pub fn ajustar_intervalo_tabela(inicio: i64, fim: i64, at: i64, quantas: i64) -> Option<(i64, i64)> {
    ajustar_intervalo_com_borda(inicio, fim, at, quantas, true)
}

pub fn ajustar_intervalo_com_borda(
    inicio: i64,
    fim: i64,
    at: i64,
    quantas: i64,
    esticar_borda: bool,
) -> Option<(i64, i64)> {
    let (mut novo_inicio, mut novo_fim) = (inicio, fim);
    if quantas > 0 {
        if at <= inicio {
            novo_inicio += quantas;
            novo_fim += quantas;
        } else if at <= if esticar_borda { fim + 1 } else { fim } {
            novo_fim += quantas;
        }
    } else {
        let removidas = -quantas;
        let ultima_removida = at + removidas - 1;
        if ultima_removida < inicio {
            novo_inicio -= removidas;
            novo_fim -= removidas;
        } else if at <= inicio && ultima_removida >= fim {
            return None;
        } else if at <= inicio {
            novo_inicio = at;
            novo_fim = fim - (ultima_removida - inicio + 1);
        } else if at <= fim {
            novo_fim = fim - removidas.min(fim - at + 1);
        }
    }
    (novo_fim >= novo_inicio).then_some((novo_inicio, novo_fim))
}

/// Acompanha inserções/exclusões de linhas ou colunas em regiões mescladas.
/// Porte de `core/mesclagem.ts::reflowMesclas`: chama a mesma
/// `ajustar_intervalo` das tabelas, mas SEM esticar a borda adjacente.
/// Mesclas que viram uma única célula (ou desaparecem) deixam de existir.
/// O resultado preserva a ordem e nunca transfere conteúdo entre células.
pub fn refluir_mesclas(
    mesclas: &[crate::celulas::refs::Rect],
    coluna: bool,
    at: u32,
    delta: i64,
) -> Vec<crate::celulas::refs::Rect> {
    use crate::celulas::refs::{MAX_COLS, MAX_ROWS};
    let limite = if coluna { MAX_COLS } else { MAX_ROWS } as i64;
    mesclas.iter().filter_map(|original| {
        let m = original.normalized();
        let (a, b) = if coluna { (m.c0, m.c1) } else { (m.r0, m.r1) };
        let (inicio, fim) = ajustar_intervalo(a as i64, b as i64, at as i64, delta)?;
        // Não persistir coordenadas inválidas/fora da grade.
        if inicio < 0 || fim >= limite || inicio > fim { return None; }
        let mut nova = m;
        if coluna {
            nova.c0 = inicio as u32;
            nova.c1 = fim as u32;
        } else {
            nova.r0 = inicio as u32;
            nova.r1 = fim as u32;
        }
        (nova.r1 > nova.r0 || nova.c1 > nova.c0).then_some(nova)
    }).collect()
}

/// O intervalo de uma regra de formatação condicional, com a conta própria
/// do `reflowRules` (que o `ops.ts` mantém à parte do `ajustarRect`, com os
/// seus testes). `None` quando o intervalo some.
pub fn ajustar_intervalo_de_regra(inicio: i64, fim: i64, at: i64, quantas: i64, removendo: bool) -> Option<(i64, i64)> {
    let comeco = |i: i64| {
        if removendo {
            if i < at {
                i
            } else if i < at + quantas {
                at
            } else {
                i - quantas
            }
        } else if i >= at {
            i + quantas
        } else {
            i
        }
    };
    let final_ = |i: i64| {
        if removendo {
            if i < at {
                i
            } else if i < at + quantas {
                at - 1
            } else {
                i - quantas
            }
        } else if i >= at {
            i + quantas
        } else {
            i
        }
    };
    let (a, b) = (comeco(inicio), final_(fim));
    (b >= a).then_some((a, b))
}

/// As larguras ou alturas personalizadas, andando com as linhas/colunas. A
/// que estava numa linha excluída sai.
pub fn ajustar_tamanhos(tamanhos: &[(u32, u32)], at: u32, quantas: u32, removendo: bool) -> Vec<(u32, u32)> {
    let mut saida: Vec<(u32, u32)> = tamanhos
        .iter()
        .filter_map(|&(i, t)| {
            if removendo {
                if i >= at && i < at + quantas {
                    return None;
                }
                Some((if i >= at + quantas { i - quantas } else { i }, t))
            } else {
                Some((if i >= at { i + quantas } else { i }, t))
            }
        })
        .collect();
    saida.sort_unstable();
    saida
}

#[cfg(test)]
mod testes {
    use super::*;

    /// Os casos do `ajustarRect acompanha inserir e excluir` (table.test.ts).
    #[test]
    fn ajustar_como_o_ajustar_rect() {
        assert_eq!(ajustar_intervalo(5, 9, 2, 3), Some((8, 12)), "inserir antes empurra");
        assert_eq!(ajustar_intervalo(5, 9, 7, 2), Some((5, 11)), "inserir dentro estica");
        assert_eq!(ajustar_intervalo(5, 9, 20, 5), Some((5, 9)), "inserir depois não mexe");
        assert_eq!(ajustar_intervalo(5, 9, 0, -2), Some((3, 7)), "excluir antes puxa");
        assert_eq!(ajustar_intervalo(5, 9, 6, -2), Some((5, 7)), "excluir dentro encolhe");
        assert_eq!(ajustar_intervalo(5, 9, 5, -5), None, "excluir tudo apaga");
        assert_eq!(ajustar_intervalo(0, 3, 0, -1), Some((0, 2)), "coluna: o começo fica e o fim encolhe");
        // ⚠ Herdado do `ajustarRect` (regra 12: traduzir, não melhorar).
        // Excluir 3..=6 de 5..=9 deixa as linhas 7, 8 e 9, que viram 3..=5;
        // a conta antiga desconta só as duas excluídas de dentro e dá
        // 3..=7. Anotado para consertar com o usuário.
        assert_eq!(ajustar_intervalo(5, 9, 3, -4), Some((3, 7)), "excluir o começo: comportamento antigo");
    }

    #[test]
    fn regra_anda_e_encolhe() {
        assert_eq!(ajustar_intervalo_de_regra(5, 9, 2, 3, false), Some((8, 12)));
        assert_eq!(ajustar_intervalo_de_regra(5, 9, 7, 2, false), Some((5, 11)));
        assert_eq!(ajustar_intervalo_de_regra(5, 9, 6, 2, true), Some((5, 7)));
        assert_eq!(ajustar_intervalo_de_regra(5, 9, 5, 5, true), None);
        assert_eq!(ajustar_intervalo_de_regra(5, 9, 0, 2, true), Some((3, 7)));
    }

    #[test]
    fn tamanhos_andam_e_os_excluidos_saem() {
        assert_eq!(ajustar_tamanhos(&[(1, 30), (5, 40)], 3, 2, false), vec![(1, 30), (7, 40)]);
        assert_eq!(ajustar_tamanhos(&[(1, 30), (3, 50), (5, 40)], 3, 1, true), vec![(1, 30), (4, 40)]);
    }

    #[test]
    fn reflow_mesclas_ao_inserir_excluir_linhas_e_colunas() {
        use crate::celulas::refs::Rect;
        let mesclas = vec![Rect::new(5, 2, 7, 4), Rect::new(11, 1, 11, 3)];
        // Inserir antes empurra tudo, sem mexer no tamanho.
        assert_eq!(refluir_mesclas(&mesclas, false, 2, 3),
            vec![Rect::new(8, 2, 10, 4), Rect::new(14, 1, 14, 3)]);
        // Inserir por dentro estica apenas a área relevante.
        assert_eq!(refluir_mesclas(&mesclas, false, 6, 2),
            vec![Rect::new(5, 2, 9, 4), Rect::new(13, 1, 13, 3)]);
        // Excluir só a linha da mescla horizontal remove a região.
        assert_eq!(refluir_mesclas(&mesclas[1..], false, 11, -1), Vec::<Rect>::new());
        // Reduzir B2:C2 para uma única célula não deixa mescla fantasma.
        assert!(refluir_mesclas(&[Rect::new(1, 1, 1, 2)], true, 2, -1).is_empty());
        // Inserir logo DEPOIS da mescla não a expande.
        assert_eq!(refluir_mesclas(&mesclas[..1], true, 5, 1), mesclas[..1]);
    }

    #[test]
    fn tabela_estica_ao_inserir_logo_abaixo() {
        assert_eq!(ajustar_intervalo_tabela(5, 9, 10, 1), Some((5, 10)), "inserir logo abaixo estica a tabela");
        assert_eq!(ajustar_intervalo(5, 9, 10, 1), Some((5, 9)), "ajustar_intervalo comum não estica borda");
        assert_eq!(ajustar_intervalo_tabela(5, 9, 11, 1), Some((5, 9)), "inserir longe não mexe");
    }
}
