//! Quanto do arquivo já foi lido — a barra de abertura do rodapé.
//!
//! Um só arquivo abre por vez (a janela trava o "Abrir" durante a carga),
//! então o estado é global: o leitor soma os bytes, a janela consulta.

use std::io::Read;
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::Mutex;

static LIDOS: AtomicU64 = AtomicU64::new(0);
static TOTAL: AtomicU64 = AtomicU64::new(0);
static FASE: AtomicU8 = AtomicU8::new(0);
static NOME: Mutex<String> = Mutex::new(String::new());

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fase {
    /// Lendo o arquivo: a porcentagem vem dos bytes.
    Lendo,
    /// Arquivo lido; montando a planilha na memória (sem porcentagem).
    Montando,
    /// Planilha montada; calculando as fórmulas em paralelo.
    Calculando,
}

/// Zera o contador para um arquivo novo.
pub fn comecar(caminho: &std::path::Path) {
    let total = std::fs::metadata(caminho).map(|m| m.len()).unwrap_or(0);
    LIDOS.store(0, Ordering::Relaxed);
    TOTAL.store(total, Ordering::Relaxed);
    FASE.store(0, Ordering::Relaxed);
    if let Ok(mut nome) = NOME.lock() {
        *nome = caminho.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    }
}

pub fn fase(f: Fase) {
    FASE.store(f as u8, Ordering::Relaxed);
}

/// (bytes lidos, tamanho do arquivo, fase, nome do arquivo)
pub fn agora() -> (u64, u64, Fase, String) {
    let fase = match FASE.load(Ordering::Relaxed) {
        0 => Fase::Lendo,
        1 => Fase::Montando,
        _ => Fase::Calculando,
    };
    let nome = NOME.lock().map(|n| n.clone()).unwrap_or_default();
    (LIDOS.load(Ordering::Relaxed), TOTAL.load(Ordering::Relaxed), fase, nome)
}

/// Leitor que soma ao contador o que passa por ele.
pub struct Contador<R>(pub R);

impl<R: Read> Read for Contador<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.0.read(buf)?;
        LIDOS.fetch_add(n as u64, Ordering::Relaxed);
        Ok(n)
    }
}

/// Leitores que contam linhas em vez de bytes (Parquet): `feito` de `total`.
pub fn andamento(feito: u64, total: u64) {
    TOTAL.store(total, Ordering::Relaxed);
    LIDOS.store(feito.min(total), Ordering::Relaxed);
}
