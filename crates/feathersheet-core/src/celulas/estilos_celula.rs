//! Estilo direto das células, guardado por **faixa** e não por célula.
//!
//! Até 16/10/2026 `Sheet::styles` era um mapa `chave da célula → índice do
//! estilo`: na Planilha Pesada (1 milhão de linhas × 13 colunas, 6 milhões de
//! células formatadas) eram 6 milhões de entradas, ~150 MB, e cada abertura
//! ou gravação do arquivo passava por todas elas, uma a uma. Mas uma coluna
//! inteira em moeda é **um** fato, não um milhão: aqui cada coluna guarda uma
//! lista ordenada de faixas `(linha inicial, linha final, estilo)` e a
//! Planilha Pesada vira 13 faixas.
//!
//! A interface por fora é a de um mapa (`insert`, `remove`, `get`,
//! `contains_key`, `keys`, `iter`, `len`) para os ~50 usos do motor não
//! mudarem de forma. Faixas vizinhas com o mesmo estilo se fundem ao inserir;
//! remover ou trocar o estilo de uma célula no meio parte a faixa em duas.
//! O pior caso (linhas alternadas, cada uma com estilo próprio) ocupa uma
//! faixa por célula — 12 bytes, ainda menos que a entrada do mapa antigo.
//!
//! Plano de 400 MB, Fase 2 (`docs/planejamento/PLANO-MEMORIA-400MB.md`).

use std::collections::HashMap;

use crate::celulas::refs::{key, key_col, key_row, CellKey};

/// Linhas `r0..=r1` de uma coluna com o estilo `idx`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Faixa {
    pub r0: u32,
    pub r1: u32,
    pub idx: u32,
}

impl Faixa {
    #[inline]
    fn celulas(&self) -> usize {
        (self.r1 - self.r0) as usize + 1
    }
}

/// O estilo direto de cada célula da aba, por faixas de coluna.
#[derive(Default, Clone)]
pub struct EstilosCelula {
    /// Por coluna, as faixas em ordem crescente de linha, sem sobreposição
    /// e sem duas vizinhas com o mesmo estilo (sempre fundidas).
    colunas: HashMap<u32, Vec<Faixa>>,
    /// Quantas células têm estilo (soma das faixas).
    total: usize,
}

/// A posição da faixa que contém `row`, ou onde uma nova entraria.
#[inline]
fn procurar(faixas: &[Faixa], row: u32) -> Result<usize, usize> {
    faixas.binary_search_by(|f| {
        if f.r1 < row {
            std::cmp::Ordering::Less
        } else if f.r0 > row {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        }
    })
}

/// Funde a faixa em `p` com as vizinhas quando têm o mesmo estilo e são
/// contíguas. Devolve a posição final dela.
fn fundir(faixas: &mut Vec<Faixa>, mut p: usize) -> usize {
    if p + 1 < faixas.len() {
        let (a, b) = (faixas[p], faixas[p + 1]);
        if a.idx == b.idx && a.r1.checked_add(1) == Some(b.r0) {
            faixas[p].r1 = b.r1;
            faixas.remove(p + 1);
        }
    }
    if p > 0 {
        let (a, b) = (faixas[p - 1], faixas[p]);
        if a.idx == b.idx && a.r1.checked_add(1) == Some(b.r0) {
            faixas[p - 1].r1 = b.r1;
            faixas.remove(p);
            p -= 1;
        }
    }
    p
}

impl EstilosCelula {
    pub fn new() -> Self {
        Self::default()
    }

    /// Quantas células têm estilo direto.
    #[inline]
    pub fn len(&self) -> usize {
        self.total
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.total == 0
    }

    /// Quantas faixas existem ao todo (o tamanho real guardado).
    pub fn faixas_len(&self) -> usize {
        self.colunas.values().map(Vec::len).sum()
    }

    pub fn clear(&mut self) {
        self.colunas.clear();
        self.total = 0;
    }

    /// O estilo da célula, se tiver.
    #[inline]
    pub fn get(&self, k: CellKey) -> Option<usize> {
        self.get_em(key_row(k), key_col(k))
    }

    #[inline]
    pub fn get_em(&self, row: u32, col: u32) -> Option<usize> {
        let faixas = self.colunas.get(&col)?;
        procurar(faixas, row).ok().map(|p| faixas[p].idx as usize)
    }

    #[inline]
    pub fn contains_key(&self, k: &CellKey) -> bool {
        self.get(*k).is_some()
    }

    /// Dá o estilo `idx` à célula. Devolve o estilo anterior, como um mapa.
    pub fn insert(&mut self, k: CellKey, idx: usize) -> Option<usize> {
        self.inserir_faixa(key_col(k), key_row(k), key_row(k), idx)
    }

    /// Dá o estilo `idx` às linhas `r0..=r1` da coluna, de uma vez. Devolve o
    /// estilo anterior quando a faixa é uma célula só (para [`Self::insert`]).
    pub fn inserir_faixa(&mut self, col: u32, r0: u32, r1: u32, idx: usize) -> Option<usize> {
        if r1 < r0 {
            return None;
        }
        let idx = u32::try_from(idx).ok()?;
        let faixas = self.colunas.entry(col).or_default();
        let nova = Faixa { r0, r1, idx };

        // Caminho rápido: coluna vazia ou faixa depois da última.
        if faixas.last().map_or(true, |u| u.r1 < r0) {
            faixas.push(nova);
            self.total += nova.celulas();
            fundir(faixas, faixas.len() - 1);
            return None;
        }

        // Caminho rápido: uma célula dentro de uma faixa.
        if r0 == r1 {
            match procurar(faixas, r0) {
                Ok(p) => {
                    let atual = faixas[p];
                    if atual.idx == idx {
                        return Some(idx as usize);
                    }
                    // Parte a faixa em até três: antes, a célula, depois.
                    let mut novas = Vec::with_capacity(3);
                    if atual.r0 < r0 {
                        novas.push(Faixa { r0: atual.r0, r1: r0 - 1, idx: atual.idx });
                    }
                    let meio = novas.len();
                    novas.push(nova);
                    if atual.r1 > r1 {
                        novas.push(Faixa { r0: r1 + 1, r1: atual.r1, idx: atual.idx });
                    }
                    faixas.splice(p..=p, novas);
                    fundir(faixas, p + meio);
                    return Some(atual.idx as usize);
                }
                Err(p) => {
                    faixas.insert(p, nova);
                    self.total += 1;
                    fundir(faixas, p);
                    return None;
                }
            }
        }

        // Caso geral: a faixa nova cobre o que havia no intervalo.
        let removidas = Self::remover_intervalo(faixas, r0, r1);
        self.total -= removidas;
        let p = match procurar(faixas, r0) {
            Ok(p) | Err(p) => p,
        };
        faixas.insert(p, nova);
        self.total += nova.celulas();
        fundir(faixas, p);
        None
    }

    /// Tira as linhas `r0..=r1` das faixas da coluna, partindo as pontas.
    /// Devolve quantas células perderam estilo.
    fn remover_intervalo(faixas: &mut Vec<Faixa>, r0: u32, r1: u32) -> usize {
        let inicio = match procurar(faixas, r0) {
            Ok(p) | Err(p) => p,
        };
        let mut removidas = 0usize;
        let mut sobras: Vec<Faixa> = Vec::new();
        let mut fim = inicio;
        while fim < faixas.len() && faixas[fim].r0 <= r1 {
            let f = faixas[fim];
            let dentro_r0 = f.r0.max(r0);
            let dentro_r1 = f.r1.min(r1);
            removidas += (dentro_r1 - dentro_r0) as usize + 1;
            if f.r0 < r0 {
                sobras.push(Faixa { r0: f.r0, r1: r0 - 1, idx: f.idx });
            }
            if f.r1 > r1 {
                sobras.push(Faixa { r0: r1 + 1, r1: f.r1, idx: f.idx });
            }
            fim += 1;
        }
        faixas.splice(inicio..fim, sobras);
        removidas
    }

    /// Tira o estilo da célula. Devolve o que havia.
    pub fn remove(&mut self, k: &CellKey) -> Option<usize> {
        let (row, col) = (key_row(*k), key_col(*k));
        let faixas = self.colunas.get_mut(&col)?;
        let p = procurar(faixas, row).ok()?;
        let f = faixas[p];
        self.total -= 1;
        if f.r0 == f.r1 {
            faixas.remove(p);
        } else if f.r0 == row {
            faixas[p].r0 += 1;
        } else if f.r1 == row {
            faixas[p].r1 -= 1;
        } else {
            faixas[p].r1 = row - 1;
            faixas.insert(p + 1, Faixa { r0: row + 1, r1: f.r1, idx: f.idx });
        }
        if faixas.is_empty() {
            self.colunas.remove(&col);
        }
        Some(f.idx as usize)
    }

    /// Tira o estilo das linhas `r0..=r1` da coluna. Devolve quantas células
    /// perderam estilo.
    pub fn remover_faixa(&mut self, col: u32, r0: u32, r1: u32) -> usize {
        let Some(faixas) = self.colunas.get_mut(&col) else { return 0 };
        let removidas = Self::remover_intervalo(faixas, r0, r1);
        if faixas.is_empty() {
            self.colunas.remove(&col);
        }
        self.total -= removidas;
        removidas
    }

    /// As colunas que têm alguma célula com estilo, em ordem.
    pub fn colunas(&self) -> Vec<u32> {
        let mut c: Vec<u32> = self.colunas.keys().copied().collect();
        c.sort_unstable();
        c
    }

    /// As faixas de uma coluna, em ordem de linha.
    pub fn faixas_da_coluna(&self, col: u32) -> &[Faixa] {
        self.colunas.get(&col).map(Vec::as_slice).unwrap_or(&[])
    }

    /// Todas as faixas, por coluna em ordem: `(coluna, faixa)`.
    pub fn faixas(&self) -> impl Iterator<Item = (u32, Faixa)> + '_ {
        let mut cols: Vec<u32> = self.colunas.keys().copied().collect();
        cols.sort_unstable();
        cols.into_iter().flat_map(move |c| self.colunas[&c].iter().map(move |f| (c, *f)))
    }

    /// A chave de cada célula com estilo (expande as faixas). Ordem: por
    /// coluna, depois por linha.
    pub fn keys(&self) -> impl Iterator<Item = CellKey> + '_ {
        self.faixas().flat_map(|(c, f)| (f.r0..=f.r1).map(move |r| key(r, c)))
    }

    /// Cada célula com estilo e o estilo dela.
    pub fn iter(&self) -> impl Iterator<Item = (CellKey, usize)> + '_ {
        self.faixas().flat_map(|(c, f)| (f.r0..=f.r1).map(move |r| (key(r, c), f.idx as usize)))
    }

    /// As chaves com estilo dentro do retângulo, sem passear pela aba inteira:
    /// só as faixas das colunas pedidas, cortadas em `r0..=r1`.
    pub fn chaves_no_retangulo(&self, r0: u32, c0: u32, r1: u32, c1: u32) -> Vec<CellKey> {
        let mut out = Vec::new();
        for (col, faixas) in &self.colunas {
            if *col < c0 || *col > c1 {
                continue;
            }
            let inicio = match procurar(faixas, r0) {
                Ok(p) | Err(p) => p,
            };
            for f in &faixas[inicio..] {
                if f.r0 > r1 {
                    break;
                }
                for r in f.r0.max(r0)..=f.r1.min(r1) {
                    out.push(key(r, *col));
                }
            }
        }
        out
    }

    /// Alguma célula com estilo dentro do retângulo satisfaz `cond`?
    pub fn alguma_no_retangulo(&self, r0: u32, c0: u32, r1: u32, c1: u32,
        mut cond: impl FnMut(u32, u32, usize) -> bool) -> bool {
        for (col, faixas) in &self.colunas {
            if *col < c0 || *col > c1 {
                continue;
            }
            let inicio = match procurar(faixas, r0) {
                Ok(p) | Err(p) => p,
            };
            for f in &faixas[inicio..] {
                if f.r0 > r1 {
                    break;
                }
                for r in f.r0.max(r0)..=f.r1.min(r1) {
                    if cond(r, *col, f.idx as usize) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// A maior linha e a maior coluna com estilo, se houver.
    pub fn limites(&self) -> Option<(u32, u32)> {
        let mut out: Option<(u32, u32)> = None;
        for (col, faixas) in &self.colunas {
            let Some(ultima) = faixas.last() else { continue };
            let (r, c) = out.unwrap_or((0, 0));
            out = Some((r.max(ultima.r1), c.max(*col)));
        }
        out
    }

    /// Inserir `count` linhas em `at`: o que está em `at` ou abaixo desce.
    /// Uma faixa que atravessa `at` fica com um buraco (as linhas novas não
    /// têm estilo), como o mapa antigo fazia célula a célula.
    pub fn inserir_linhas(&mut self, at: u32, count: u32) {
        if count == 0 {
            return;
        }
        for faixas in self.colunas.values_mut() {
            let mut i = 0;
            while i < faixas.len() {
                let f = faixas[i];
                if f.r1 < at {
                    i += 1;
                    continue;
                }
                if f.r0 >= at {
                    faixas[i].r0 = f.r0.saturating_add(count);
                    faixas[i].r1 = f.r1.saturating_add(count);
                    i += 1;
                    continue;
                }
                // Atravessa `at`: parte em duas, a de baixo desce.
                faixas[i].r1 = at - 1;
                faixas.insert(i + 1, Faixa { r0: at.saturating_add(count), r1: f.r1.saturating_add(count), idx: f.idx });
                i += 2;
            }
        }
    }

    /// Excluir as linhas `at..at+count`: o que está nelas some, o que está
    /// abaixo sobe.
    pub fn excluir_linhas(&mut self, at: u32, count: u32) {
        if count == 0 {
            return;
        }
        let end = at.saturating_add(count);
        let mut vazias = Vec::new();
        for (col, faixas) in self.colunas.iter_mut() {
            let removidas = Self::remover_intervalo(faixas, at, end - 1);
            self.total -= removidas;
            let mut p_fundir = None;
            for (i, f) in faixas.iter_mut().enumerate() {
                if f.r0 >= end {
                    f.r0 -= count;
                    f.r1 -= count;
                    if p_fundir.is_none() {
                        p_fundir = Some(i);
                    }
                }
            }
            if let Some(p) = p_fundir {
                fundir(faixas, p);
            }
            if faixas.is_empty() {
                vazias.push(*col);
            }
        }
        for c in vazias {
            self.colunas.remove(&c);
        }
    }

    /// Inserir `count` colunas em `at`: as colunas de `at` em diante andam.
    pub fn inserir_colunas(&mut self, at: u32, count: u32) {
        if count == 0 {
            return;
        }
        let colunas = std::mem::take(&mut self.colunas);
        self.colunas = colunas
            .into_iter()
            .map(|(c, f)| (if c >= at { c.saturating_add(count) } else { c }, f))
            .collect();
    }

    /// Excluir as colunas `at..at+count`.
    pub fn excluir_colunas(&mut self, at: u32, count: u32) {
        if count == 0 {
            return;
        }
        let end = at.saturating_add(count);
        let colunas = std::mem::take(&mut self.colunas);
        for (c, faixas) in colunas {
            if c >= at && c < end {
                self.total -= faixas.iter().map(Faixa::celulas).sum::<usize>();
                continue;
            }
            let destino = if c >= end { c - count } else { c };
            self.colunas.insert(destino, faixas);
        }
    }
}

impl std::fmt::Debug for EstilosCelula {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EstilosCelula")
            .field("celulas", &self.total)
            .field("faixas", &self.faixas_len())
            .finish()
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn k(r: u32, c: u32) -> CellKey {
        key(r, c)
    }

    #[test]
    fn coluna_inteira_e_uma_faixa_so() {
        let mut e = EstilosCelula::new();
        for r in 1..=1_000 {
            assert_eq!(e.insert(k(r, 3), 7), None);
        }
        assert_eq!(e.len(), 1_000);
        assert_eq!(e.faixas_len(), 1);
        assert_eq!(e.get(k(500, 3)), Some(7));
        assert_eq!(e.get(k(0, 3)), None);
        assert_eq!(e.get(k(1_001, 3)), None);
        assert_eq!(e.get(k(500, 4)), None);
    }

    #[test]
    fn trocar_o_estilo_no_meio_parte_a_faixa() {
        let mut e = EstilosCelula::new();
        for r in 0..10 {
            e.insert(k(r, 0), 1);
        }
        assert_eq!(e.insert(k(5, 0), 2), Some(1));
        assert_eq!(e.faixas_len(), 3);
        assert_eq!(e.len(), 10);
        assert_eq!(e.get(k(4, 0)), Some(1));
        assert_eq!(e.get(k(5, 0)), Some(2));
        assert_eq!(e.get(k(6, 0)), Some(1));
        // Voltar ao estilo 1 funde as três de novo.
        assert_eq!(e.insert(k(5, 0), 1), Some(2));
        assert_eq!(e.faixas_len(), 1);
        assert_eq!(e.len(), 10);
    }

    #[test]
    fn remover_nas_pontas_e_no_meio() {
        let mut e = EstilosCelula::new();
        for r in 0..10 {
            e.insert(k(r, 0), 1);
        }
        assert_eq!(e.remove(&k(0, 0)), Some(1));
        assert_eq!(e.remove(&k(9, 0)), Some(1));
        assert_eq!(e.remove(&k(5, 0)), Some(1));
        assert_eq!(e.remove(&k(5, 0)), None);
        assert_eq!(e.len(), 7);
        assert_eq!(e.faixas_len(), 2);
        let chaves: Vec<u32> = e.keys().map(key_row).collect();
        assert_eq!(chaves, vec![1, 2, 3, 4, 6, 7, 8]);
        for r in 1..9 {
            if r != 5 {
                e.remove(&k(r, 0));
            }
        }
        assert!(e.is_empty());
        assert_eq!(e.faixas_len(), 0);
    }

    #[test]
    fn inserir_fora_de_ordem_funde_igual() {
        let mut e = EstilosCelula::new();
        e.insert(k(5, 0), 1);
        e.insert(k(3, 0), 1);
        e.insert(k(4, 0), 1);
        e.insert(k(7, 0), 1);
        e.insert(k(6, 0), 1);
        assert_eq!(e.faixas_len(), 1);
        assert_eq!(e.len(), 5);
        assert_eq!(e.faixas_da_coluna(0), &[Faixa { r0: 3, r1: 7, idx: 1 }]);
    }

    #[test]
    fn linhas_alternadas_viram_uma_faixa_por_celula() {
        let mut e = EstilosCelula::new();
        for r in 0..100 {
            e.insert(k(r, 2), (r % 2) as usize);
        }
        assert_eq!(e.faixas_len(), 100);
        assert_eq!(e.len(), 100);
        assert_eq!(e.get(k(51, 2)), Some(1));
        assert_eq!(e.get(k(50, 2)), Some(0));
    }

    #[test]
    fn faixa_inteira_cobre_o_que_havia() {
        let mut e = EstilosCelula::new();
        for r in 0..20 {
            e.insert(k(r, 0), (r % 3) as usize);
        }
        e.inserir_faixa(0, 5, 14, 9);
        assert_eq!(e.len(), 20);
        assert_eq!(e.get(k(4, 0)), Some(1));
        assert_eq!(e.get(k(5, 0)), Some(9));
        assert_eq!(e.get(k(14, 0)), Some(9));
        assert_eq!(e.get(k(15, 0)), Some(0));
        assert_eq!(e.remover_faixa(0, 0, 4), 5);
        assert_eq!(e.len(), 15);
        assert_eq!(e.get(k(4, 0)), None);
        assert_eq!(e.remover_faixa(0, 100, 200), 0);
    }

    #[test]
    fn inserir_e_excluir_linhas() {
        let mut e = EstilosCelula::new();
        for r in 0..10 {
            e.insert(k(r, 1), 1);
        }
        e.insert(k(20, 1), 2);
        e.inserir_linhas(5, 3);
        assert_eq!(e.len(), 11);
        assert_eq!(e.get(k(4, 1)), Some(1));
        assert_eq!(e.get(k(5, 1)), None);
        assert_eq!(e.get(k(7, 1)), None);
        assert_eq!(e.get(k(8, 1)), Some(1));
        assert_eq!(e.get(k(12, 1)), Some(1));
        assert_eq!(e.get(k(13, 1)), None);
        assert_eq!(e.get(k(23, 1)), Some(2));
        e.excluir_linhas(5, 3);
        assert_eq!(e.len(), 11);
        assert_eq!(e.faixas_len(), 2);
        assert_eq!(e.get(k(9, 1)), Some(1));
        assert_eq!(e.get(k(20, 1)), Some(2));
        e.excluir_linhas(0, 4);
        assert_eq!(e.len(), 7);
        assert_eq!(e.get(k(0, 1)), Some(1));
        assert_eq!(e.get(k(5, 1)), Some(1));
        assert_eq!(e.get(k(6, 1)), None);
        assert_eq!(e.get(k(16, 1)), Some(2));
    }

    #[test]
    fn inserir_e_excluir_colunas() {
        let mut e = EstilosCelula::new();
        e.insert(k(0, 0), 1);
        e.insert(k(0, 2), 2);
        e.insert(k(0, 5), 3);
        e.inserir_colunas(2, 2);
        assert_eq!(e.get(k(0, 0)), Some(1));
        assert_eq!(e.get(k(0, 2)), None);
        assert_eq!(e.get(k(0, 4)), Some(2));
        assert_eq!(e.get(k(0, 7)), Some(3));
        e.excluir_colunas(3, 2);
        assert_eq!(e.len(), 2);
        assert_eq!(e.get(k(0, 0)), Some(1));
        assert_eq!(e.get(k(0, 5)), Some(3));
        assert_eq!(e.colunas(), vec![0, 5]);
    }

    #[test]
    fn retangulo_e_limites() {
        let mut e = EstilosCelula::new();
        for r in 0..100 {
            e.insert(k(r, 0), 1);
            e.insert(k(r, 3), 2);
        }
        let mut dentro = e.chaves_no_retangulo(10, 0, 12, 1);
        dentro.sort_unstable();
        assert_eq!(dentro, vec![k(10, 0), k(11, 0), k(12, 0)]);
        assert_eq!(e.limites(), Some((99, 3)));
        assert!(e.alguma_no_retangulo(0, 0, 99, 3, |_, c, i| c == 3 && i == 2));
        assert!(!e.alguma_no_retangulo(0, 0, 99, 2, |_, c, _| c == 3));
        assert_eq!(e.iter().count(), 200);
    }

    #[test]
    fn bate_com_um_mapa_em_operacoes_aleatorias() {
        // Gerador simples e determinístico (LCG) — sem dependência.
        let mut semente: u64 = 0x9e37_79b9_7f4a_7c15;
        let mut rnd = move || {
            semente = semente.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (semente >> 33) as u32
        };
        let mut e = EstilosCelula::new();
        let mut mapa: HashMap<CellKey, usize> = HashMap::new();
        for _ in 0..20_000 {
            let r = rnd() % 64;
            let c = rnd() % 3;
            let op = rnd() % 10;
            let chave = k(r, c);
            if op < 6 {
                let idx = (rnd() % 3) as usize;
                assert_eq!(e.insert(chave, idx), mapa.insert(chave, idx));
            } else if op < 9 {
                assert_eq!(e.remove(&chave), mapa.remove(&chave));
            } else {
                let r1 = (r + rnd() % 8).min(63);
                let idx = (rnd() % 3) as usize;
                e.inserir_faixa(c, r, r1, idx);
                for rr in r..=r1 {
                    mapa.insert(k(rr, c), idx);
                }
            }
            assert_eq!(e.len(), mapa.len());
        }
        let mut a: Vec<(CellKey, usize)> = e.iter().collect();
        let mut b: Vec<(CellKey, usize)> = mapa.into_iter().collect();
        a.sort_unstable();
        b.sort_unstable();
        assert_eq!(a, b);
        // Nenhuma faixa vizinha ficou por fundir.
        for c in e.colunas() {
            let f = e.faixas_da_coluna(c);
            for par in f.windows(2) {
                assert!(par[0].r1 < par[1].r0);
                assert!(!(par[0].idx == par[1].idx && par[0].r1 + 1 == par[1].r0), "faixas por fundir: {par:?}");
            }
        }
    }
}
