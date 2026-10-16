//! Borda de dados da navegação de macros (api-navegacao.ts).
//! O armazenamento esparso fornece os máximos; nunca caminha por 10M linhas.
use super::*;

impl Sheet {
    /// Índice 1-based da última linha com conteúdo nesta coluna.
    /// Zero para coluna vazia; estilos sem texto não são conteúdo.
    pub fn ultima_linha_na_coluna(&self, coluna: u32) -> u32 {
        if coluna >= MAX_COLS { return 0; }
        let mut ultimo = self.data.ultima_linha_coluna(coluna)
            .into_iter().chain(self.formulas.ultima_linha_coluna(coluna));
        let mut maior = ultimo.next();
        for r in ultimo { maior = Some(maior.map_or(r, |m| m.max(r))); }
        for (&chave, celula) in &self.cells {
            if key_col(chave) == coluna && !celula.raw.is_empty() {
                let r=key_row(chave);
                maior=Some(maior.map_or(r, |m| m.max(r)));
            }
        }
        maior.map_or(0, |r| r + 1)
    }

    /// Índice 1-based da última coluna com conteúdo na linha.
    /// O custo depende das colunas *existentes*, não da grade virtual.
    pub fn ultima_coluna_na_linha(&self, linha: u32) -> u32 {
        if linha >= MAX_ROWS { return 0; }
        let mut maior: Option<u32> = None;
        for (&chave, celula) in &self.cells {
            if key_row(chave) == linha && !celula.raw.is_empty() {
                let c=key_col(chave);
                maior=Some(maior.map_or(c,|m|m.max(c)));
            }
        }
        for c in self.data.colunas() {
            if self.data.cru_em(linha,c).is_some() {
                maior=Some(maior.map_or(c,|m|m.max(c)));
            }
        }
        self.formulas.for_each_in_range(linha,linha,0,MAX_COLS-1,|_,c|{
            maior=Some(maior.map_or(c,|m|m.max(c)));
        });
        maior.map_or(0,|c|c+1)
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    #[test]
    fn coluna_e_linha_temas_esparsos_e_formulas() {
        let mut s=Sheet::new("Dados");
        s.set_cell(0,0,"10");
        s.set_cell(5,2,"=A1*2");
        s.set_cell(3,2,"texto");
        s.set_cell(3,4,"8");
        assert_eq!(s.ultima_linha_na_coluna(0),1);
        assert_eq!(s.ultima_linha_na_coluna(2),6);
        assert_eq!(s.ultima_linha_na_coluna(4),4);
        assert_eq!(s.ultima_linha_na_coluna(1),0);
        assert_eq!(s.ultima_coluna_na_linha(3),5);
        assert_eq!(s.ultima_coluna_na_linha(5),3);
        assert_eq!(s.ultima_coluna_na_linha(2),0);
    }
    #[test]
    fn celula_somente_de_estilo_nao_conta_na_navegacao() {
        let mut s=Sheet::new("Dados");
        let mut style=crate::celulas::estilo_parcial::EstiloParcial::new();
        style.insert("bold".into(),crate::celulas::estilo_parcial::Campo::Logico(true));
        s.set_style(&Rect::cell(9,9),&style);
        assert_eq!(s.ultima_linha_na_coluna(9),0);
        assert_eq!(s.ultima_coluna_na_linha(9),0);
        assert_eq!(s.ultima_linha_na_coluna(MAX_COLS),0);
        assert_eq!(s.ultima_coluna_na_linha(MAX_ROWS),0);
    }
    #[test]
    fn dados_no_limite_de_milhoes_nao_exigem_varrer_toda_a_coluna() {
        let mut s=Sheet::new("Grande");
        s.set_cell(9_999_999,3,"final");
        assert_eq!(s.ultima_linha_na_coluna(3),10_000_000);
        assert_eq!(s.ultima_coluna_na_linha(9_999_999),4);
    }
}
