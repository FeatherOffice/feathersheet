# Migração de funções numéricas das macros para Rust

Origem: `feathersheet/src/macros/fn-planilha.ts`. Nesta migração, os métodos `app.fn.sum`, `count`, `countA`, `max` e `min` passaram a chamar `feathersheet-macros/src/funcoes_criterio.rs` pelo host Boa (interface JS de macro preservada).

Reutiliza `achatar`, `numero` e limites do host existente (500 mil elementos, 64 níveis); não cria uma segunda implementação de listas. Ignora textos ao somar/contar números; `countA` inclui zero, `false` e objetos, mas ignora `null` e texto vazio; `min/max` sem números retornam zero. `countIf/sumIf` permanecem inalterados. Testes Rust foram escritos, não executados.

Ainda faltam `average`, `median`, `averageIf`, `vlookup`, `hlookup`, `match` e `index` (contrato `CellError`). Nenhum workflow remoto foi acionado.

Validação local: `cargo check --manifest-path feathersheet/crates/feathersheet-macros/Cargo.toml -j 2` e `cargo test --manifest-path feathersheet/crates/feathersheet-macros/Cargo.toml -j 2`.
