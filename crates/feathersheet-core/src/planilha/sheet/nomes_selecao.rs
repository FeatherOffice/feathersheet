//! Fórmulas → Criar a partir da seleção: os títulos de uma borda da
//! seleção viram nomes para as linhas ou colunas de dados, como no Excel.
//!
//! Com Linha superior marcada, cada coluna ganha o nome do seu título e o
//! intervalo vai do título (exclusive) até a última linha de dados; as outras
//! bordas marcadas ficam de fora do intervalo.
use super::Sheet;
use crate::celulas::number::to_js_string;
use crate::celulas::refs::{col_index, Rect, MAX_ROWS};
use crate::celulas::values::Value;

/// Quais bordas da seleção têm os títulos.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Bordas {
    pub topo: bool,
    pub esquerda: bool,
    pub base: bool,
    pub direita: bool,
}

/// Nome aceito numa fórmula a partir do texto de um título: espaço e
/// símbolo viram `_`, começo com algarismo ganha `_`, e um texto que seria
/// endereço (`A1`) ganha `_` no fim. Vazio: `None`.
pub fn nome_de_titulo(titulo: &str) -> Option<String> {
    let mut nome: String = titulo.trim().chars()
        .map(|c| if c.is_alphanumeric() || c == '_' || c == '.' { c } else { '_' })
        .take(60)
        .collect();
    if nome.is_empty() || nome.chars().all(|c| c == '_') {
        return None;
    }
    if !nome.chars().next().is_some_and(|c| c.is_alphabetic() || c == '_') {
        nome.insert(0, '_');
    }
    if let Some(pos) = nome.find(|c: char| c.is_ascii_digit()) {
        let (coluna, linha) = nome.split_at(pos);
        if col_index(coluna).is_some() && linha.parse::<u32>().is_ok_and(|l| (1..=MAX_ROWS).contains(&l)) {
            nome.push('_');
        }
    }
    Some(nome)
}

impl Sheet {
    fn titulo(&self, r: u32, c: u32) -> Option<String> {
        match self.get_value(r, c) {
            Value::Text(t) => nome_de_titulo(&t),
            Value::Number(n) => nome_de_titulo(&to_js_string(n)),
            Value::Bool(b) => nome_de_titulo(if b { "VERDADEIRO" } else { "FALSO" }),
            _ => None,
        }
    }

    /// As bordas que parecem títulos, para a janela já abrir marcada:
    /// a linha de cima (ou a coluna da esquerda) só com texto, e o resto não.
    pub fn sugerir_bordas(&self, area: &Rect) -> Bordas {
        let a = area.normalized();
        let texto = |r: u32, c: u32| matches!(self.get_value(r, c), Value::Text(_));
        let vazio = |r: u32, c: u32| matches!(self.get_value(r, c), Value::Empty);
        let todos_texto = |celulas: &mut dyn Iterator<Item = (u32, u32)>| {
            let mut algum = false;
            for (r, c) in celulas {
                if vazio(r, c) { continue; }
                if !texto(r, c) { return false; }
                algum = true;
            }
            algum
        };
        let mut b = Bordas::default();
        if a.r1 > a.r0 {
            b.topo = todos_texto(&mut (a.c0..=a.c1).map(|c| (a.r0, c)));
        }
        if a.c1 > a.c0 {
            let r0 = if b.topo { a.r0 + 1 } else { a.r0 };
            b.esquerda = r0 <= a.r1 && todos_texto(&mut (r0..=a.r1).map(|r| (r, a.c0)));
        }
        if !b.topo && !b.esquerda {
            if a.r1 > a.r0 { b.topo = true } else if a.c1 > a.c0 { b.esquerda = true }
        }
        b
    }

    /// Os nomes que a seleção gera, na ordem: topo, esquerda, base, direita.
    /// Título vazio não gera nome; o mesmo nome repetido fica só na primeira vez.
    pub fn nomes_da_selecao(&self, area: &Rect, bordas: Bordas) -> Vec<(String, Rect)> {
        let a = area.normalized();
        // O corpo é a seleção sem as bordas de título marcadas.
        let r0 = a.r0 + bordas.topo as u32;
        let r1 = a.r1.saturating_sub(bordas.base as u32);
        let c0 = a.c0 + bordas.esquerda as u32;
        let c1 = a.c1.saturating_sub(bordas.direita as u32);
        let mut saida: Vec<(String, Rect)> = Vec::new();
        let mut por = |titulo: Option<String>, rect: Rect| {
            if let Some(nome) = titulo {
                if !saida.iter().any(|(n, _)| n.to_uppercase() == nome.to_uppercase()) {
                    saida.push((nome, rect));
                }
            }
        };
        if r0 > r1 || c0 > c1 {
            return saida;
        }
        if bordas.topo { for c in c0..=c1 { por(self.titulo(a.r0, c), Rect::new(r0, c, r1, c)) } }
        if bordas.esquerda { for r in r0..=r1 { por(self.titulo(r, a.c0), Rect::new(r, c0, r, c1)) } }
        if bordas.base { for c in c0..=c1 { por(self.titulo(a.r1, c), Rect::new(r0, c, r1, c)) } }
        if bordas.direita { for r in r0..=r1 { por(self.titulo(r, a.c1), Rect::new(r, c0, r, c1)) } }
        saida
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn tabela() -> Sheet {
        let mut s = Sheet::new("Plan1");
        for (c, t) in ["", "Preço", "Qtd total"].iter().enumerate() {
            s.set_cell_raw(0, c as u32, t.to_string(), None);
        }
        for (r, t) in ["Jan", "Fev"].iter().enumerate() {
            s.set_cell_raw(r as u32 + 1, 0, t.to_string(), None);
            s.set_cell_raw(r as u32 + 1, 1, "10".into(), None);
            s.set_cell_raw(r as u32 + 1, 2, "3".into(), None);
        }
        s
    }

    #[test]
    fn sugere_topo_e_esquerda_quando_sao_texto() {
        let s = tabela();
        let b = s.sugerir_bordas(&Rect::new(0, 0, 2, 2));
        assert_eq!(b, Bordas { topo: true, esquerda: true, base: false, direita: false });
    }

    #[test]
    fn topo_e_esquerda_geram_colunas_e_linhas_sem_os_titulos() {
        let s = tabela();
        let nomes = s.nomes_da_selecao(&Rect::new(0, 0, 2, 2),
            Bordas { topo: true, esquerda: true, ..Default::default() });
        assert_eq!(nomes, vec![
            ("Preço".to_string(), Rect::new(1, 1, 2, 1)),
            ("Qtd_total".to_string(), Rect::new(1, 2, 2, 2)),
            ("Jan".to_string(), Rect::new(1, 1, 1, 2)),
            ("Fev".to_string(), Rect::new(2, 1, 2, 2)),
        ]);
    }

    #[test]
    fn titulo_vira_nome_valido() {
        assert_eq!(nome_de_titulo("Total 2023").as_deref(), Some("Total_2023"));
        assert_eq!(nome_de_titulo("2023").as_deref(), Some("_2023"));
        assert_eq!(nome_de_titulo("A1").as_deref(), Some("A1_"));
        assert_eq!(nome_de_titulo("  ").as_deref(), None);
        assert_eq!(nome_de_titulo("R$/mês").as_deref(), Some("R__mês"));
    }
}
