//! Comentários (anotações) das células — aba Revisão do Excel.
//!
//! Cada célula tem no máximo um comentário: autor, texto e se o balão fica
//! sempre à mostra (Mostrar/ocultar comentário). No Excel novo isso se chama
//! **anotação** (a nota clássica); o comentário encadeado, com respostas, é
//! lido do `.xlsx` como uma anotação só, com as respostas no texto.
//!
//! É dado do documento: entra no desfazer (um passo por célula alterada,
//! [`Sheet::definir_comentario`] e [`Sheet::limpar_comentarios`] fecham uma
//! transação cada), anda junto ao inserir/excluir linhas e colunas
//! ([`Sheet::deslocar_comentarios`]) e ao ordenar
//! ([`Sheet::permutar_comentarios`]), e é gravado no `.fshz` (campo `cm`) e
//! no `.xlsx` (`xl/commentsN.xml` + `vmlDrawingN.vml`).

use std::collections::BTreeMap;

use super::Sheet;
use crate::celulas::refs::Rect;

/// O comentário de uma célula.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Comentario {
    /// Quem escreveu (no Excel, o nome do usuário do Office). Pode ser vazio.
    pub autor: String,
    pub texto: String,
    /// Balão sempre à mostra (Mostrar/ocultar comentário), não só ao passar
    /// o mouse.
    pub visivel: bool,
}

/// Os comentários da aba, em ordem de linha e depois de coluna — a ordem
/// do Próximo/Anterior.
pub type Comentarios = BTreeMap<(u32, u32), Comentario>;

impl Sheet {
    /// O comentário da célula, se houver.
    pub fn comentario(&self, linha: u32, coluna: u32) -> Option<&Comentario> {
        self.comentarios.get(&(linha, coluna))
    }

    /// Troca (ou apaga, com `None`) o comentário da célula num Ctrl+Z.
    /// Comentário de texto vazio também conta: o Excel deixa criar e só
    /// apaga por Excluir. Devolve se mudou algo.
    pub fn definir_comentario(&mut self, linha: u32, coluna: u32, novo: Option<Comentario>) -> bool {
        if self.comentarios.get(&(linha, coluna)) == novo.as_ref() { return false; }
        self.abrir_transacao();
        self.registrar_comentario(linha, coluna, novo);
        self.fechar_transacao();
        true
    }

    /// Apaga os comentários da área num Ctrl+Z (Excluir com várias células
    /// selecionadas, Limpar comentários). Devolve quantos saíram.
    pub fn limpar_comentarios(&mut self, area: &Rect) -> usize {
        let alvo = self.comentarios_na_area(area);
        if alvo.is_empty() { return 0; }
        self.abrir_transacao();
        for &(r, c) in &alvo { self.registrar_comentario(r, c, None); }
        self.fechar_transacao();
        alvo.len()
    }

    /// Liga ou desliga o "sempre à mostra" dos comentários da área num
    /// Ctrl+Z. Devolve quantos mudaram.
    pub fn mostrar_comentarios(&mut self, area: &Rect, visivel: bool) -> usize {
        let alvo: Vec<(u32, u32)> = self.comentarios_na_area(area).into_iter()
            .filter(|k| self.comentarios[k].visivel != visivel).collect();
        if alvo.is_empty() { return 0; }
        self.abrir_transacao();
        for &(r, c) in &alvo {
            let mut novo = self.comentarios[&(r, c)].clone();
            novo.visivel = visivel;
            self.registrar_comentario(r, c, Some(novo));
        }
        self.fechar_transacao();
        alvo.len()
    }

    /// As células com comentário dentro da área, em ordem.
    pub fn comentarios_na_area(&self, area: &Rect) -> Vec<(u32, u32)> {
        self.comentarios.range((area.r0, 0)..=(area.r1, u32::MAX))
            .map(|(&k, _)| k)
            .filter(|&(_, c)| c >= area.c0 && c <= area.c1)
            .collect()
    }

    /// O próximo comentário depois de `(linha, coluna)` em ordem de linha
    /// (ou o anterior, com `voltar`), dando a volta na aba como o Excel.
    /// `None` só quando a aba não tem nenhum.
    pub fn comentario_vizinho(&self, linha: u32, coluna: u32, voltar: bool) -> Option<(u32, u32)> {
        let aqui = (linha, coluna);
        if voltar {
            self.comentarios.range(..aqui).next_back()
                .or_else(|| self.comentarios.iter().next_back())
                .map(|(&k, _)| k)
        } else {
            use std::ops::Bound::{Excluded, Unbounded};
            self.comentarios.range((Excluded(aqui), Unbounded)).next()
                .or_else(|| self.comentarios.iter().next())
                .map(|(&k, _)| k)
        }
    }

    /// Troca o comentário dentro da transação aberta, com o passo de desfazer.
    pub(crate) fn registrar_comentario(&mut self, linha: u32, coluna: u32, depois: Option<Comentario>) {
        let antes = match &depois {
            Some(c) => self.comentarios.insert((linha, coluna), c.clone()),
            None => self.comentarios.remove(&(linha, coluna)),
        };
        if antes == depois { return; }
        self.empilhar_comentario(linha, coluna, antes, depois);
    }

    /// Põe o comentário sem histórico (desfazer/refazer e carga de arquivo).
    pub(crate) fn aplicar_comentario(&mut self, linha: u32, coluna: u32, valor: Option<Comentario>) {
        match valor {
            Some(c) => { self.comentarios.insert((linha, coluna), c); }
            None => { self.comentarios.remove(&(linha, coluna)); }
        }
    }

    /// Acompanha inserir (`removendo == false`) ou excluir `count`
    /// linhas/colunas em `at`; comentário de linha/coluna excluída some,
    /// como no Excel.
    pub(crate) fn deslocar_comentarios(&mut self, coluna: bool, at: u32, count: u32, removendo: bool) {
        if self.comentarios.is_empty() { return; }
        let mover = |p: u32| -> Option<u32> {
            if !removendo { return Some(if p >= at { p + count } else { p }); }
            if p >= at && p < at + count { return None; }
            Some(if p >= at + count { p - count } else { p })
        };
        self.comentarios = std::mem::take(&mut self.comentarios).into_iter()
            .filter_map(|((r, c), v)| {
                let k = if coluna { (r, mover(c)?) } else { (mover(r)?, c) };
                Some((k, v))
            })
            .collect();
    }

    /// Ordenar: a linha `destino_de[r - first]` recebe o comentário de `r`
    /// (só nas colunas `c0..=c1`, as que foram ordenadas).
    pub(crate) fn permutar_comentarios(&mut self, first: u32, c0: u32, c1: u32, destino_de: &[u32]) {
        let last = first + destino_de.len() as u32 - 1;
        let dentro: Vec<(u32, u32)> = self.comentarios.range((first, 0)..=(last, u32::MAX))
            .map(|(&k, _)| k).filter(|&(_, c)| c >= c0 && c <= c1).collect();
        let movidos: Vec<_> = dentro.into_iter()
            .filter_map(|k| self.comentarios.remove(&k).map(|v| (k, v))).collect();
        for ((r, c), v) in movidos {
            self.comentarios.insert((destino_de[(r - first) as usize], c), v);
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn nota(texto: &str) -> Comentario {
        Comentario { autor: "Ana".into(), texto: texto.into(), visivel: false }
    }

    #[test]
    fn inserir_editar_excluir_e_desfazer() {
        let mut s = Sheet::new("P");
        assert!(s.definir_comentario(1, 2, Some(nota("primeiro"))));
        assert!(!s.definir_comentario(1, 2, Some(nota("primeiro"))), "igual não empilha");
        assert!(s.definir_comentario(1, 2, Some(nota("editado"))));
        assert!(s.definir_comentario(1, 2, None));
        assert!(s.comentario(1, 2).is_none());
        assert!(s.undo());
        assert_eq!(s.comentario(1, 2).unwrap().texto, "editado");
        assert!(s.undo());
        assert_eq!(s.comentario(1, 2).unwrap().texto, "primeiro");
        assert!(s.undo());
        assert!(s.comentario(1, 2).is_none());
        assert!(s.redo() && s.redo());
        assert_eq!(s.comentario(1, 2).unwrap().texto, "editado");
    }

    #[test]
    fn limpar_e_mostrar_area_num_ctrl_z() {
        let mut s = Sheet::new("P");
        for (r, c) in [(0, 0), (0, 5), (3, 1), (9, 9)] {
            s.definir_comentario(r, c, Some(nota("x")));
        }
        assert_eq!(s.mostrar_comentarios(&Rect::new(0, 0, 3, 5), true), 3);
        assert!(s.comentario(3, 1).unwrap().visivel && !s.comentario(9, 9).unwrap().visivel);
        assert_eq!(s.limpar_comentarios(&Rect::new(0, 0, 3, 4)), 2);
        assert_eq!(s.comentarios.len(), 2);
        assert!(s.undo());
        assert_eq!(s.comentarios.len(), 4);
        assert!(s.undo());
        assert!(!s.comentario(0, 0).unwrap().visivel);
    }

    #[test]
    fn proximo_e_anterior_dao_a_volta() {
        let mut s = Sheet::new("P");
        assert_eq!(s.comentario_vizinho(0, 0, false), None);
        for (r, c) in [(2, 3), (0, 7), (2, 1)] {
            s.definir_comentario(r, c, Some(nota("x")));
        }
        assert_eq!(s.comentario_vizinho(0, 0, false), Some((0, 7)));
        assert_eq!(s.comentario_vizinho(0, 7, false), Some((2, 1)));
        assert_eq!(s.comentario_vizinho(2, 3, false), Some((0, 7)), "volta ao começo");
        assert_eq!(s.comentario_vizinho(0, 7, true), Some((2, 3)), "volta ao fim");
        assert_eq!(s.comentario_vizinho(2, 3, true), Some((2, 1)));
    }

    #[test]
    fn acompanha_inserir_excluir_e_ordenar() {
        let mut s = Sheet::new("P");
        s.definir_comentario(5, 2, Some(nota("a")));
        s.definir_comentario(1, 2, Some(nota("b")));
        s.deslocar(false, 3, 2, false);
        assert_eq!(s.comentario(7, 2).unwrap().texto, "a");
        assert_eq!(s.comentario(1, 2).unwrap().texto, "b");
        s.deslocar(true, 0, 1, true);
        assert_eq!(s.comentario(7, 1).unwrap().texto, "a");
        s.deslocar(false, 7, 1, true);
        assert!(s.comentario(7, 1).is_none() && s.comentarios.len() == 1);

        // Ordenar: A1:A3 = 3, 1, 2 com comentário em A1 ("três").
        let mut s = Sheet::new("P");
        for (r, v) in [(0, "3"), (1, "1"), (2, "2")] { s.set_cell_raw(r, 0, v.to_string(), None); }
        s.definir_comentario(0, 0, Some(nota("três")));
        s.permutar_linhas(0, 0, 0, &[1, 2, 0]);
        assert_eq!(s.get_raw(2, 0), "3");
        assert_eq!(s.comentario(2, 0).unwrap().texto, "três");
        assert!(s.undo());
        assert_eq!(s.comentario(0, 0).unwrap().texto, "três");
    }
}
