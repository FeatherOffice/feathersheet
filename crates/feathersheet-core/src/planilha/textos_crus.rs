//! O texto digitado de literal que não se reconstrói do valor ("007", "15.0",
//! "True"), guardado com pouca memória.
//!
//! Em planilhas grandes o mesmo texto aparece milhões de vezes ("True",
//! "1.0"). Um `HashMap<u64, String>` pagava uma alocação por célula. Aqui a
//! célula guarda só um `u32` (o id da forma) e as formas ficam num dicionário
//! sem repetição. Não há dependência nova: é só `std`.
//!
//! As formas não saem do dicionário quando a célula é apagada ou sobrescrita.
//! É aceitável: o dicionário cresce só com textos distintos já digitados.

use std::collections::HashMap;

/// Mapa de célula (`CellKey`) para o texto digitado, com as formas
/// deduplicadas. A API imita a de `HashMap<u64, String>` no que o `Sheet` usa.
#[derive(Default, Clone, Debug)]
pub struct TextosCrus {
    /// Célula → id da forma em `formas`.
    celulas: HashMap<u64, u32>,
    /// Cada forma distinta, uma vez só. O id é o índice aqui.
    formas: Vec<Box<str>>,
    /// Forma → id, para achar a forma já existente ao inserir.
    indice: HashMap<Box<str>, u32>,
}

impl TextosCrus {
    pub fn new() -> Self {
        Self::default()
    }

    /// Devolve (ou cria) o id da forma `texto`.
    fn id_da_forma(&mut self, texto: &str) -> u32 {
        if let Some(&id) = self.indice.get(texto) {
            return id;
        }
        let id = u32::try_from(self.formas.len()).expect("mais de 4 bilhões de textos distintos");
        let forma: Box<str> = texto.into();
        self.formas.push(forma.clone());
        self.indice.insert(forma, id);
        id
    }

    pub fn get(&self, k: &u64) -> Option<&str> {
        self.celulas.get(k).map(|&id| &*self.formas[id as usize])
    }

    pub fn contains_key(&self, k: &u64) -> bool {
        self.celulas.contains_key(k)
    }

    /// Guarda `texto` na célula `k`, trocando o que estava lá. Não aloca
    /// quando a forma já existe no dicionário.
    pub fn insert(&mut self, k: u64, texto: &str) {
        let id = self.id_da_forma(texto);
        self.celulas.insert(k, id);
    }

    /// Tira a célula `k`. Devolve se ela existia. A forma fica no dicionário.
    pub fn remove(&mut self, k: &u64) -> bool {
        self.celulas.remove(k).is_some()
    }

    pub fn len(&self) -> usize {
        self.celulas.len()
    }

    pub fn is_empty(&self) -> bool {
        self.celulas.is_empty()
    }

    /// Esvazia as células e o dicionário.
    pub fn clear(&mut self) {
        self.celulas.clear();
        self.formas.clear();
        self.indice.clear();
    }

    /// Pares (célula, texto), como `HashMap::iter`.
    pub fn iter(&self) -> impl Iterator<Item = (&u64, &str)> {
        self.celulas.iter().map(|(k, &id)| (k, &*self.formas[id as usize]))
    }

    /// Só os textos, como `HashMap::values`.
    pub fn values(&self) -> impl Iterator<Item = &str> {
        self.celulas.values().map(|&id| &*self.formas[id as usize])
    }

    /// Quantas formas distintas o dicionário guarda (para medir).
    pub fn formas_distintas(&self) -> usize {
        self.formas.len()
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn mil_celulas_com_o_mesmo_texto_guardam_uma_forma() {
        let mut t = TextosCrus::new();
        for k in 0..1000u64 {
            t.insert(k, "True");
        }
        assert_eq!(t.len(), 1000);
        assert_eq!(t.formas_distintas(), 1);
        assert_eq!(t.get(&999), Some("True"));
    }

    #[test]
    fn get_insert_remove_e_sobrescrever() {
        let mut t = TextosCrus::new();
        assert!(t.is_empty());
        assert_eq!(t.get(&1), None);

        t.insert(1, "007");
        assert!(t.contains_key(&1));
        assert_eq!(t.get(&1), Some("007"));

        t.insert(1, "15.0");
        assert_eq!(t.len(), 1, "sobrescrever não duplica a célula");
        assert_eq!(t.get(&1), Some("15.0"));

        assert!(t.remove(&1));
        assert!(!t.remove(&1));
        assert_eq!(t.get(&1), None);
        assert!(t.is_empty());
    }

    #[test]
    fn iter_e_values_devolvem_tudo() {
        let mut t = TextosCrus::new();
        t.insert(10, "a");
        t.insert(20, "b");
        t.insert(30, "a");

        let mut pares: Vec<(u64, String)> = t.iter().map(|(&k, v)| (k, v.to_string())).collect();
        pares.sort();
        assert_eq!(pares, vec![(10, "a".into()), (20, "b".into()), (30, "a".into())]);

        let mut valores: Vec<&str> = t.values().collect();
        valores.sort();
        assert_eq!(valores, vec!["a", "a", "b"]);
    }

    #[test]
    fn clear_zera_celulas_e_dicionario() {
        let mut t = TextosCrus::new();
        t.insert(1, "x");
        t.clear();
        assert!(t.is_empty());
        assert_eq!(t.formas_distintas(), 0);
    }
}
