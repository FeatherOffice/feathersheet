# Dados → Classificar página — migração TypeScript → Rust/Iced

Origem arquivada: `descarte/feathersheet/legacy-frontend/src/ui/app/comandos.ts`
(`sort(app, 'sheet', ascending)`) e `src/ui/barras/ribbon.ts`.

Implementação ativa: `feathersheet-gui/src/app/ordenacao_pagina.rs`, integrada à Ribbon
**Dados → Classificar página → Página A → Z / Página Z → A**.

- Ordena da linha 2 até a última linha usada, mantendo a linha 1 como cabeçalho.
- Ordena pela coluna ativa e move todas as colunas juntas, via
  `feathersheet-core::operacoes::ops::{planejar_ordenacao,aplicar_ordenacao}`.
- Preserva fórmulas, estilos e desfazer/refazer no motor Rust.
- Corrige o seletor anterior com textos sem correspondência no despacho.
- Segurança: máximo de 250 mil linhas e 3 milhões de células na área retangular,
  com recusa explícita; não trunca nem tenta alocar os 100 milhões de linhas virtuais.
  Para volumes maiores, falta executor assíncrono/streaming.
- Corrige também o seletor Zoom (Exibir) para aceitar 25%–400%; limites pessoais
  continuam validados pelo handler `Msg::Zoom`.

O frontend TypeScript original já está em `descarte/feathersheet/legacy-frontend/`;
não duplicar cópias. Testes Rust foram escritos, **não executados** neste ambiente.
Nenhum GitHub Actions foi disparado para a verificação.

Comandos para validação local:

```bash
cargo check --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
cargo test --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml classificar_pagina -j 2
cargo run --manifest-path feathersheet/crates/feathersheet-gui/Cargo.toml -j 2
```
