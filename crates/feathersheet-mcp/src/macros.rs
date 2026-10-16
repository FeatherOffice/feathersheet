//! Macros JavaScript (API `sheet`, a mesma do Alt+F11): listar, ler,
//! escrever, excluir e executar. O catálogo é o arquivo comum da suíte
//! (`featheroffice/macros.json`), o mesmo que o editor do aplicativo usa.
//!
//! Executar roda o código na pasta do hospedeiro; um erro reverte as escritas
//! da execução (o motor de macros já faz isso).
use std::path::PathBuf;

use feather_macros::{Catalogo, Macro};
use serde_json::Value as Json;

use crate::ferramentas::Saida;
use crate::hospede::Hospede;
use crate::texto::*;

/// `FEATHERSHEET_MCP_MACROS` troca o arquivo (testes); senão, o da suíte.
fn caminho() -> Result<PathBuf, String> {
    std::env::var_os("FEATHERSHEET_MCP_MACROS").map(PathBuf::from)
        .or_else(feather_macros::caminho_padrao)
        .ok_or_else(|| "Pasta de configuração indisponível".into())
}

fn carregar() -> Result<Catalogo, String> {
    let caminho = caminho()?;
    if !caminho.exists() { return Ok(Catalogo::default()); }
    Catalogo::carregar_de(&caminho)
}

fn gravar(catalogo: &Catalogo) -> Result<(), String> {
    let caminho = caminho()?;
    if let Some(pasta) = caminho.parent() {
        std::fs::create_dir_all(pasta).map_err(|e| e.to_string())?;
    }
    catalogo.salvar_em(&caminho)
}

pub fn macro_(h: &mut dyn Hospede, args: &Json) -> Saida {
    let acao = arg_texto(args, "acao").unwrap_or("listar");
    let nome = arg_texto(args, "nome");
    let mut catalogo = carregar()?;
    let texto = match acao {
        "listar" => {
            if catalogo.macros.is_empty() { "Nenhuma macro guardada.".into() } else {
                catalogo.macros.iter().map(|m| format!("{} ({} linha(s))", m.name, m.code.lines().count()))
                    .collect::<Vec<_>>().join("\n")
            }
        }
        "ler" => {
            let nome = nome.ok_or("Falta `nome`.")?;
            catalogo.obter(nome).map(|m| m.code.clone()).ok_or_else(|| format!("Não existe a macro \"{nome}\"."))?
        }
        "escrever" => {
            let nome = nome.ok_or("Falta `nome`.")?;
            let codigo = args.get("codigo").and_then(Json::as_str).ok_or("Falta `codigo`.")?;
            match catalogo.macros.iter_mut().find(|m| m.name == nome) {
                Some(m) => m.code = codigo.to_string(),
                None => catalogo.macros.push(Macro { name: nome.to_string(), code: codigo.to_string(), auto: false }),
            }
            // Passa pela mesma validação do editor (nome, tamanho, duplicata).
            let validado = Catalogo::importar_json(&catalogo.exportar_json()?)?;
            gravar(&validado)?;
            h.macros_mudaram();
            format!("Macro \"{nome}\" guardada. Ela não roda sozinha: use acao \"executar\".")
        }
        "excluir" => {
            let nome = nome.ok_or("Falta `nome`.")?;
            let antes = catalogo.macros.len();
            catalogo.macros.retain(|m| m.name != nome);
            if catalogo.macros.len() == antes { return Err(format!("Não existe a macro \"{nome}\".")); }
            gravar(&catalogo)?;
            h.macros_mudaram();
            format!("Macro \"{nome}\" excluída.")
        }
        "executar" => {
            let codigo = match (args.get("codigo").and_then(Json::as_str), nome) {
                (Some(c), _) => c.to_string(),
                (None, Some(n)) => catalogo.obter(n).map(|m| m.code.clone()).ok_or_else(|| format!("Não existe a macro \"{n}\"."))?,
                _ => return Err("Informe `nome` (macro guardada) ou `codigo`.".into()),
            };
            let aba = aba_do_arg(h, args)?;
            let r = feathersheet_macros::executar(h.pasta_mut(), aba, &codigo);
            if r.alterou { h.marcar_alterado(); }
            let mut s = r.saida.join("\n");
            match r.erro {
                Some(e) => s.push_str(&format!("\nErro: {e}\nAs escritas desta execução foram revertidas.")),
                None => s.push_str("\nMacro concluída."),
            }
            if let Some(st) = r.status { s.push_str(&format!("\nStatus: {st}")); }
            s.trim_start().to_string()
        }
        outra => return Err(format!("Ação desconhecida: {outra}. Use listar, ler, escrever, excluir ou executar.")),
    };
    Ok(conteudo(texto))
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::hospede::Avulso;
    use serde_json::json;

    #[test]
    fn escrever_ler_executar_e_excluir() {
        let arquivo = std::env::temp_dir().join(format!("fsheet_macros_{}.json", std::process::id()));
        std::env::set_var("FEATHERSHEET_MCP_MACROS", &arquivo);
        let mut h = Avulso::default();
        let texto = |r: Saida| r.unwrap()[0]["text"].as_str().unwrap().to_string();
        texto(macro_(&mut h, &json!({ "acao": "escrever", "nome": "Dobro", "codigo": "sheet.range(\"B1\").value = 42; log(\"ok\");" })));
        assert!(texto(macro_(&mut h, &json!({ "acao": "listar" }))).contains("Dobro"));
        let r = texto(macro_(&mut h, &json!({ "acao": "executar", "nome": "Dobro" })));
        assert!(r.contains("ok") && r.contains("concluída"), "{r}");
        assert_eq!(crate::texto::valor_texto(&h.pasta.valor(0, 0, 1)), "42");
        texto(macro_(&mut h, &json!({ "acao": "excluir", "nome": "Dobro" })));
        let _ = std::fs::remove_file(&arquivo);
    }
}
