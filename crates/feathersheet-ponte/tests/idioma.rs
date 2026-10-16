//! O comando `idioma` com uma língua que só existe como arquivo (a
//! configuração regional chega do `src/idiomas/<código>.json`). À parte dos
//! testes do `src/` porque o idioma é global no processo.

use serde_json::{json, Value};

fn chamar(p: &mut feathersheet_ponte::Ponte, cmd: &str, corpo: Value) -> Result<Value, String> {
    let bytes = p.chamar(cmd, &serde_json::to_vec(&corpo).unwrap())?;
    Ok(serde_json::from_slice(&bytes).unwrap())
}

#[test]
fn idioma_vindo_do_arquivo_de_lingua() {
    let mut p = feathersheet_ponte::Ponte::default();
    let moeda = json!({ "itens": [[-1250.5, { "format": "currency" }], [1234567.891, null]] });

    // Sem configuração regional, só as embutidas são aceitas.
    assert!(chamar(&mut p, "idioma", json!({ "idioma": "de-DE" })).is_err());

    let de = json!({
        "idioma": "de-DE",
        "regional": { "nome": "Deutsch", "decimal": ",", "milhar": ".", "separadorDeArgumentos": ";",
                      "moeda": "€", "codigoDaMoeda": "EUR", "moedaAntes": false, "espacoNaMoeda": true }
    });
    chamar(&mut p, "idioma", de).unwrap();
    assert_eq!(chamar(&mut p, "formatar", moeda.clone()).unwrap(), json!(["-1.250,50\u{a0}€", "1.234.567,891"]));

    // Separador com mais de um caractere é recusado com uma mensagem que diz qual.
    let ruim = json!({ "idioma": "xx", "regional": { "nome": "x", "decimal": ",,", "milhar": ".", "separadorDeArgumentos": ";", "moeda": "¤", "codigoDaMoeda": "XXX" } });
    let erro = chamar(&mut p, "idioma", ruim).unwrap_err();
    assert!(erro.contains("\"decimal\""), "{erro}");

    chamar(&mut p, "idioma", json!({ "idioma": "pt-BR" })).unwrap();
    assert_eq!(chamar(&mut p, "formatar", moeda).unwrap(), json!(["-R$\u{a0}1.250,50", "1.234.567,891"]));
}
