//! A ponte vista de fora de um `.wasm`: é assim que os testes do TypeScript
//! rodam contra o motor de verdade.
//!
//! O protocolo é o mais simples que a memória compartilhada permite, e é
//! síncrono como o `pasta://` do aplicativo:
//!
//! 1. `ponte_entrada(n)` devolve onde escrever `n` bytes: o nome do comando
//!    seguido do corpo JSON;
//! 2. `ponte_chamar(tamanho_do_nome, tamanho_do_corpo)` atende e devolve 0
//!    (deu certo) ou 1 (a saída é a mensagem de erro);
//! 3. `ponte_saida()` e `ponte_saida_tamanho()` dizem onde ler a resposta.
//!
//! `ponte_reiniciar()` começa uma pasta nova — cada teste nasce limpo.
//!
//! O WebAssembly aqui não tem linha de execução nenhuma além da principal,
//! então o `Mutex` nunca espera; ele só está aí porque `static mut` pede
//! `unsafe` a cada uso.

use std::sync::Mutex;

use crate::Ponte;

static PONTE: Mutex<Option<Ponte>> = Mutex::new(None);
static ENTRADA: Mutex<Vec<u8>> = Mutex::new(Vec::new());
static SAIDA: Mutex<Vec<u8>> = Mutex::new(Vec::new());

#[no_mangle]
pub extern "C" fn ponte_reiniciar() {
    *PONTE.lock().unwrap() = Some(Ponte::default());
}

#[no_mangle]
pub extern "C" fn ponte_entrada(n: u32) -> *mut u8 {
    let mut e = ENTRADA.lock().unwrap();
    e.clear();
    e.resize(n as usize, 0);
    e.as_mut_ptr()
}

#[no_mangle]
pub extern "C" fn ponte_chamar(tamanho_do_nome: u32, tamanho_do_corpo: u32) -> u32 {
    let entrada = ENTRADA.lock().unwrap();
    let n = tamanho_do_nome as usize;
    let fim = n + tamanho_do_corpo as usize;
    let resultado = match std::str::from_utf8(&entrada[..n]) {
        Err(_) => Err("nome de comando não é UTF-8".to_string()),
        Ok(nome) => {
            let mut guarda = PONTE.lock().unwrap();
            let ponte = guarda.get_or_insert_with(Ponte::default);
            ponte.chamar(nome, &entrada[n..fim])
        }
    };
    let mut saida = SAIDA.lock().unwrap();
    match resultado {
        Ok(bytes) => {
            *saida = bytes;
            0
        }
        Err(msg) => {
            *saida = msg.into_bytes();
            1
        }
    }
}

#[no_mangle]
pub extern "C" fn ponte_saida() -> *const u8 {
    SAIDA.lock().unwrap().as_ptr()
}

#[no_mangle]
pub extern "C" fn ponte_saida_tamanho() -> u32 {
    SAIDA.lock().unwrap().len() as u32
}
