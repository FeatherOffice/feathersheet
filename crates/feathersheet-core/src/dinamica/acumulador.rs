//! Agregador incremental: não armazena um f64 por linha quando basta
//! acumular soma, contagem, mínimo, máximo ou média.
//! Moda conserva somente frequências por valor; o desvio-padrão amostral
//! usa Welford (média e M2), com memória O(1) mesmo em milhões de linhas.
use std::collections::HashMap;
use super::pivot::AgregacaoOp;

#[derive(Debug, Clone)]
pub(super) struct Acumulador {
    op: AgregacaoOp,
    quantidade: usize,
    soma: f64,
    menor: f64,
    maior: f64,
    // Algoritmo de Welford: reduz cancelamento numérico sem reter entradas.
    media_desvio: f64,
    soma_quadrados: f64,
    frequencias: HashMap<u64, usize>,
    moda: f64,
    moda_frequencia: usize,
}

impl Acumulador {
    pub fn novo(op: AgregacaoOp) -> Self {
        Self {
            op, quantidade: 0, soma: 0., menor: f64::INFINITY,
            maior: f64::NEG_INFINITY, media_desvio: 0.0, soma_quadrados: 0.0,
            frequencias: HashMap::new(), moda: 0., moda_frequencia: 0,
        }
    }
    pub fn receber(&mut self, n: f64) {
        self.quantidade += 1;
        match self.op {
            AgregacaoOp::Soma | AgregacaoOp::Media => self.soma += n,
            AgregacaoOp::DesvioPadrao => {
                let diferenca = n - self.media_desvio;
                self.media_desvio += diferenca / self.quantidade as f64;
                let nova_diferenca = n - self.media_desvio;
                self.soma_quadrados += diferenca * nova_diferenca;
            }
            AgregacaoOp::Min => self.menor = self.menor.min(n),
            AgregacaoOp::Max => self.maior = self.maior.max(n),
            AgregacaoOp::Moda => {
                let frequencia = self.frequencias.entry(n.to_bits()).or_insert(0);
                *frequencia += 1;
                if *frequencia > self.moda_frequencia {
                    self.moda_frequencia = *frequencia;
                    self.moda = n;
                }
            }
            AgregacaoOp::Contagem => {}
        }
    }
    pub fn resultado(&self) -> f64 {
        if self.quantidade == 0 { return 0.; }
        match self.op {
            AgregacaoOp::Soma => self.soma,
            AgregacaoOp::Contagem => self.quantidade as f64,
            AgregacaoOp::Media => self.soma / self.quantidade as f64,
            AgregacaoOp::Min => self.menor,
            AgregacaoOp::Max => self.maior,
            AgregacaoOp::Moda => self.moda,
            AgregacaoOp::DesvioPadrao => {
                if self.quantidade < 2 { return 0.; }
                (self.soma_quadrados.max(0.) / (self.quantidade - 1) as f64).sqrt()
            }
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use super::super::pivot::aggregate;
    #[test]
    fn acumulador_igual_ao_motor_legado_em_todas_as_operacoes() {
        let numeros = [3., 5., 3., 2., 5., 5., 1.];
        for op in [
            AgregacaoOp::Soma, AgregacaoOp::Contagem, AgregacaoOp::Media,
            AgregacaoOp::Min, AgregacaoOp::Max, AgregacaoOp::DesvioPadrao,
            AgregacaoOp::Moda,
        ] {
            let mut acc = Acumulador::novo(op);
            for &n in &numeros { acc.receber(n); }
            let esperado = aggregate(&numeros, op);
            assert!((acc.resultado() - esperado).abs() < 1e-10,
                "{op:?}: {} versus {esperado}", acc.resultado());
        }
    }
    #[test]
    fn desvio_amostral_em_milhao_de_linhas_usa_memoria_constante() {
        let mut a = Acumulador::novo(AgregacaoOp::DesvioPadrao);
        for i in 0..1_000_000 {
            a.receber((i % 2) as f64);
        }
        let esperado = ((1_000_000.0_f64 / 999_999.0) * 0.25).sqrt();
        assert!((a.resultado() - esperado).abs() < 1e-8);
        assert!(a.frequencias.is_empty());
    }

    #[test]
    fn milhao_de_linhas_na_soma_nao_aloca_vetor_de_valores() {
        let mut acc = Acumulador::novo(AgregacaoOp::Soma);
        for _ in 0..1_000_000 { acc.receber(2.); }
        assert_eq!(acc.resultado(), 2_000_000.);
        assert!(acc.frequencias.is_empty());
    }
}
