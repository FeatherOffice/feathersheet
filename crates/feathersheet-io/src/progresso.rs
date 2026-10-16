//! Quanto do arquivo já foi lido — a barra de abertura do rodapé.
//!
//! Um só arquivo abre por vez (a janela trava o "Abrir" durante a carga),
//! então o estado é global: o leitor soma os bytes, a janela consulta.

use std::io::Read;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use std::sync::Mutex;

static LIDOS: AtomicU64 = AtomicU64::new(0);
static TOTAL: AtomicU64 = AtomicU64::new(0);
static FASE: AtomicU8 = AtomicU8::new(0);
static NOME: Mutex<String> = Mutex::new(String::new());
/// O leitor conta linhas (Parquet) em vez de bytes.
static EM_LINHAS: AtomicBool = AtomicBool::new(false);

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
    EM_LINHAS.store(false, Ordering::Relaxed);
    etapa_total(0);
    feathersheet_core::paralelismo::andamento::comecar(0);
    if let Ok(mut nome) = NOME.lock() {
        *nome = caminho.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    }
}

pub fn fase(f: Fase) {
    FASE.store(f as u8, Ordering::Relaxed);
}

static ETAPA_FEITO: AtomicU64 = AtomicU64::new(0);
static ETAPA_TOTAL: AtomicU64 = AtomicU64::new(0);

/// Quanto há para montar na memória (bytes de XML no `.xlsx`, células no
/// `.fshz`): a montagem também anda na barra, não fica parada em 100%.
pub fn etapa_total(total: u64) {
    ETAPA_FEITO.store(0, Ordering::Relaxed);
    ETAPA_TOTAL.store(total, Ordering::Relaxed);
}

pub fn etapa_somar(n: u64) {
    ETAPA_FEITO.fetch_add(n, Ordering::Relaxed);
}

/// (feito, total) da montagem.
pub fn etapa() -> (u64, u64) {
    (ETAPA_FEITO.load(Ordering::Relaxed), ETAPA_TOTAL.load(Ordering::Relaxed))
}

/// Leitor que soma à montagem o que passa por ele (o XML descompactado).
pub struct ContadorEtapa<R>(pub R);

impl<R: Read> Read for ContadorEtapa<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.0.read(buf)?;
        ETAPA_FEITO.fetch_add(n as u64, Ordering::Relaxed);
        Ok(n)
    }
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
    EM_LINHAS.store(true, Ordering::Relaxed);
    TOTAL.store(total, Ordering::Relaxed);
    LIDOS.store(feito.min(total), Ordering::Relaxed);
}

/// `true` quando `agora()` devolve linhas, não bytes.
pub fn em_linhas() -> bool {
    EM_LINHAS.load(Ordering::Relaxed)
}
