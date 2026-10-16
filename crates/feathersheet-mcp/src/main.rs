//! `feathersheet-mcp`: lê uma mensagem JSON por linha no stdin e responde no
//! stdout. Com o FeatherSheet aberto, as chamadas de ferramenta vão para ele
//! (agem na planilha da tela); sem ele, o servidor trabalha sobre arquivos.
use std::io::{BufRead, Write};

use feathersheet_mcp::{ao_vivo, hospede::Avulso, rpc};
use serde_json::{json, Value};

fn main() {
    let mut avulso = Avulso::default();
    let mut vivo = ao_vivo::Cliente::default();
    let saida = std::io::stdout();
    for linha in std::io::stdin().lock().lines() {
        let Ok(linha) = linha else { break };
        if linha.trim().is_empty() {
            continue;
        }
        let resposta = match serde_json::from_str::<Value>(&linha) {
            Ok(msg) => responder(&mut avulso, &mut vivo, &linha, &msg),
            Err(e) => Some(json!({ "jsonrpc": "2.0", "id": null, "error": { "code": -32700, "message": format!("JSON inválido: {e}") } })),
        };
        if let Some(r) = resposta {
            let mut s = saida.lock();
            let _ = writeln!(s, "{r}");
            let _ = s.flush();
        }
    }
}

fn responder(avulso: &mut Avulso, vivo: &mut ao_vivo::Cliente, linha: &str, msg: &Value) -> Option<Value> {
    if msg.get("method").and_then(Value::as_str) == Some("tools/call") {
        match vivo.repassar(linha) {
            Some(Ok(r)) => return serde_json::from_str(&r).ok(),
            Some(Err(e)) => {
                let id = msg.get("id").cloned().unwrap_or(Value::Null);
                return Some(json!({ "jsonrpc": "2.0", "id": id, "result": { "isError": true, "content": [{ "type": "text", "text": e }] } }));
            }
            None => {}
        }
    }
    rpc::atender(avulso, msg)
}
