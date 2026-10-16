//! A hora de agora, em milissegundos desde 1970 — o `Date.now()` do
//! JavaScript.
//!
//! Existe por causa do WebAssembly: lá não há relógio do sistema, e o
//! `SystemTime::now()` derruba a instância inteira. Os testes do TypeScript
//! rodam este motor como `.wasm` (ver `crates/sheet-ponte`), então no wasm a
//! hora é pedida a quem hospeda, pela importação `env.ponte_agora_ms`.

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "env")]
extern "C" {
    fn ponte_agora_ms() -> f64;
}

#[cfg(target_arch = "wasm32")]
pub fn agora_ms() -> f64 {
    // SAFETY: função importada sem argumento e sem efeito na memória do wasm.
    unsafe { ponte_agora_ms() }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn agora_ms() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
}
